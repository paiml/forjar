//! FJ-1250 / Refs #356 / Refs #358 — `forjar apply --plan-file`.
//!
//! Executes a previously reviewed, sealed plan. Four things make this different
//! from an ordinary apply, and all four live here or in
//! [`super::apply_from_plan_checks`]:
//!
//! * the plan's integrity is verified before anything runs
//!   (`plan_file::load_plan_file` → `core::plan_seal`);
//! * the plan's CLAIM is verified against a freshly computed plan
//!   (`check_plan_still_holds`), because integrity is not truth;
//! * the executed set is the REVIEWED set — the scope derived from the plan
//!   body is what the executor is given, instead of the whole config; and
//! * every apply flag the operator passes is either honoured or refused.
//!
//! Lifted out of `apply_variants.rs`, which is a grab-bag of unrelated apply
//! modes and had no room left for any of it.

use super::apply_from_plan_checks::*;
use super::apply_helpers::*;
use super::helpers::*;
use super::helpers_state::load_machine_locks;
use super::plan_file::load_plan_file;
use super::workspace::*;
use crate::core::plan_selectors::PlanSelectors;
use crate::core::{executor, resolver, types};
use std::path::Path;

/// The apply flags that say HOW the reviewed delta executes.
///
/// Refs #358: `execute_scoped_plan` built its `ApplyConfig` from a literal in
/// which every one of these was hard-coded off, so
/// `apply --plan-file --rollback-on-failure` armed nothing, `--progress` showed
/// nothing, `--retry 3` retried nothing and `--timeout 5` timed out never. All
/// of them parsed, none of them applied, and the run exited 0 — an operator who
/// believed a rollback was armed was wrong, silently.
///
/// These are separated from the SELECTORS deliberately. A selector asks *what*
/// converges and has to be intersected with the reviewed set; a knob asks *how*
/// the reviewed set converges and needs no such reasoning, which is why passing
/// it through is simply correct.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ApplyKnobs {
    /// FJ-266: remove a stale state lock before the run.
    pub force_unlock: bool,
    /// FJ-272: `[N/total]` counter.
    pub progress: bool,
    /// Per-transport-operation timeout.
    pub timeout_secs: Option<u64>,
    /// FJ-283: retry failed resources N times.
    pub retry: u32,
    /// FJ-290: parallel wave execution.
    pub parallel: bool,
    /// FJ-313: max concurrent resources per wave.
    pub max_parallel: Option<usize>,
    /// FJ-304: per-resource timeout.
    pub resource_timeout: Option<u64>,
    /// FJ-310: restore the pre-apply locks if any resource fails.
    pub rollback_on_failure: bool,
}

/// Everything `apply --plan-file` was invoked with.
///
/// A struct rather than fourteen positional arguments: the defect being fixed
/// here is a long `ApplyConfig` literal whose fields were silently wrong, and a
/// long argument list is the same failure mode one call frame up.
pub(crate) struct PlanApplyRequest<'a> {
    /// The config file, parsed exactly as an ordinary apply would.
    pub file: &'a Path,
    /// State directory holding the machine locks.
    pub state_dir: &'a Path,
    /// The saved plan to execute.
    pub plan_path: &'a Path,
    /// Announce what was loaded, and trace the generated scripts.
    pub verbose: bool,
    /// `--env-file`.
    pub env_file: Option<&'a Path>,
    /// `--workspace`.
    pub workspace: Option<&'a str>,
    /// forjar#370: `--operator`, or the ambient identity when unset. This is an
    /// apply, so it is authorized like one.
    pub operator: Option<&'a str>,
    /// Any flag in the `--dry-run` family (GH-208).
    pub dry_run: bool,
    /// `-m` / `-r` / `-t` / `-g` as passed to THIS invocation; they may only
    /// narrow the reviewed delta.
    pub selectors: PlanSelectors,
    /// How the reviewed delta executes.
    pub knobs: ApplyKnobs,
}

/// Rebuild the config exactly as an ordinary apply would see it.
///
/// The plan was sealed against this shape, not against the raw file: `--env-file`
/// params, the injected workspace param and resolved data sources all move the
/// canonical config hash, so `plan --out` and `apply --plan-file` have to agree
/// on the order these run in.
fn prepare_config(
    file: &Path,
    env_file: Option<&Path>,
    workspace: Option<&str>,
) -> Result<types::ForjarConfig, String> {
    let mut config = parse_and_validate(file)?;
    if let Some(path) = env_file {
        load_env_params(&mut config, path)?;
    }
    inject_workspace_param(&mut config, workspace);
    resolver::resolve_data_sources(&mut config)?;
    Ok(config)
}

/// Recompute the plan from live inputs, under `selectors`.
///
/// Nearly free: `cmd_apply_from_plan` has already parsed and resolved the
/// config, the planner is pure over `(config, locks)`, and the locks are the
/// same files `plan_seal`'s state leg has just read.
///
/// The pipeline is `plan_compute::plan_filtered` — the same function `cmd_plan`
/// uses — preceded by the same phony strip. Two spellings of "the planner plus
/// the four selectors" would make the comparison fire on plans nobody edited.
fn replan(
    config: &types::ForjarConfig,
    state_dir: &Path,
    selectors: &PlanSelectors,
) -> Result<types::ExecutionPlan, String> {
    let mut config = config.clone();
    super::apply_selection::strip_unrequested_phony(&mut config, &[]);
    let locks = load_machine_locks(&config, state_dir, selectors.machine.as_deref())?;
    super::plan_compute::plan_filtered(&config, &locks, selectors)
}

/// The `plan`-style sigil for an action.
fn sigil(action: &types::PlanAction) -> &'static str {
    match action {
        types::PlanAction::Create => "+",
        types::PlanAction::Update => "~",
        types::PlanAction::Destroy => "-",
        types::PlanAction::NoOp => "=",
    }
}

/// Refs #358: `--dry-run` PREVIEWS the reviewed plan instead of converging it.
///
/// `cmd_apply_from_plan` took no `dry_run` at all, so `apply --plan-file
/// --dry-run` converged the machine for real and printed `Plan applied: 1
/// converged`. For a two-phase plan/review/apply feature that is the worst
/// available default — the flag exists precisely so an operator can ask what a
/// sealed plan would do without doing it.
///
/// The preview is printed from the plan BODY, which is the artifact that was
/// reviewed, narrowed by the same predicate the real run will use. Asking
/// `executor::apply_scoped` instead would honour `dry_run` (it returns before
/// `dispatch_apply`) but return a result carrying no changes, so the preview
/// would print "0 converged" and tell the operator nothing.
fn preview_scoped_plan(
    plan: &types::ExecutionPlan,
    scope: &executor::PlanScope,
    config: &types::ForjarConfig,
    selectors: &PlanSelectors,
) {
    println!("Dry run — the reviewed plan would execute:");
    let mut shown = 0usize;
    for change in &plan.changes {
        if !scope.covers(&change.machine, &change.resource_id) {
            continue;
        }
        if !survives(config, selectors, &change.machine, &change.resource_id) {
            continue;
        }
        shown += 1;
        println!(
            "  {} {} on {}: {}",
            sigil(&change.action),
            change.resource_id,
            change.machine,
            change.description
        );
    }
    println!("\n{shown} reviewed change(s). No changes applied.");
}

/// Converge exactly the `(machine, resource)` pairs the reviewed plan named,
/// under the flags this invocation passed.
///
/// This function used to be inline under the comment "Execute as a normal apply
/// using the plan's resource list", which said the opposite of what it did —
/// with all selectors `None` and no scope, `executor::apply` converged the whole
/// current config and the plan body was decorative. None of those selectors CAN
/// express a plan: `resource_filter` is a single id, and a plan names a set of
/// pairs. That set is `scope`.
///
/// The comment that replaced it was wrong in the other direction: it claimed the
/// selectors stay `None` because they "were already applied when the plan body
/// was written". That is true of how the plan was PRODUCED and no reason at all
/// to drop flags the operator is passing NOW, to this invocation — which is what
/// the whole literal was doing, to every field except `machine_filter`. Every
/// field is fed from the request; the three that cannot be honoured
/// (`--force`, `--force-tag`, `--refresh`) are refused before this runs, so
/// there is nothing left for a `false` here to hide.
fn execute_scoped_plan(
    config: &types::ForjarConfig,
    req: &PlanApplyRequest,
    scope: &executor::PlanScope,
) -> Result<(), String> {
    let knobs = req.knobs;
    let cfg = executor::ApplyConfig {
        config,
        state_dir: req.state_dir,
        // Refused by `reject_replanning_flags`: each of these three re-plans the
        // delta rather than executing the reviewed one.
        force: false,
        refresh: false,
        force_tag: None,
        dry_run: false,
        machine_filter: req.selectors.machine.as_deref(),
        resource_filter: req.selectors.resource.as_deref(),
        tag_filter: req.selectors.tag.as_deref(),
        group_filter: req.selectors.group.as_deref(),
        timeout_secs: knobs.timeout_secs,
        force_unlock: knobs.force_unlock,
        progress: knobs.progress,
        retry: knobs.retry,
        parallel: super::apply_gates::parallel_flag(knobs.parallel),
        resource_timeout: knobs.resource_timeout,
        rollback_on_failure: knobs.rollback_on_failure,
        max_parallel: knobs.max_parallel,
        trace: req.verbose,
        run_id: Some(types::generate_run_id()),
    };

    let results = executor::apply_scoped(&cfg, Some(scope))?;
    let (converged, unchanged, failed) = super::apply_output::count_results(&results);
    println!("Plan applied: {converged} converged, {unchanged} unchanged, {failed} failed");

    if failed > 0 {
        return Err(format!("{failed} resource(s) failed"));
    }
    Ok(())
}

/// Say what was loaded and what it will touch, when asked.
fn report_plan(
    plan: &types::ExecutionPlan,
    scope: &executor::PlanScope,
    selectors: &PlanSelectors,
) {
    let n_changes = plan.to_create + plan.to_update + plan.to_destroy;
    eprintln!(
        "Executing saved plan: {} changes ({} create, {} update, {} destroy)",
        n_changes, plan.to_create, plan.to_update, plan.to_destroy
    );
    if let Some(flags) = selectors.describe() {
        eprintln!("Plan was written with the selectors: {flags}");
    }
    eprintln!(
        "Plan scope: {} resource(s) across machine(s): {}",
        scope.len(),
        scope.machine_names().join(", ")
    );
}

/// The reviewed plan asks for nothing. Say so, and say what it leaves undone.
fn honour_an_empty_plan(
    config: &types::ForjarConfig,
    state_dir: &Path,
    sealed: bool,
    written_with: &PlanSelectors,
) -> Result<(), String> {
    check_empty_plan_is_trustworthy(sealed)?;
    println!("Plan has no changes to apply.");
    if !written_with.is_unfiltered() {
        let whole = replan(config, state_dir, &PlanSelectors::default())?;
        disclose_work_outside_the_filter(written_with, &whole);
    }
    Ok(())
}

/// FJ-1250: Execute a previously saved plan file.
///
/// # forjar#370: this is an apply, so it is authorized like one
///
/// `dispatch_apply_b::apply_mode_exits` returns here for `--plan-file` BEFORE
/// `apply_execute`, whose first line is `check_operator_auth`. So the gate was
/// reachable only on the ordinary path, and the whole of it was skippable by
/// routing through a plan file. Measured on 1.21.0 with
/// `allowed_operators: [alice]`, as a non-alice operator:
///
/// ```text
///   forjar apply --yes                              -> not authorized   EXIT=1
///   forjar plan --out p.json                        -> EXIT=0
///   forjar apply --plan-file p.json --yes           -> 2 converged      EXIT=0
///   forjar apply --plan-file p2.json --operator mallory --yes -> applied EXIT=0
/// ```
///
/// A plan file is unauthenticated — any user can write one in a text editor —
/// so the bypass needed no privilege at all. The check lives at the TOP of this
/// function, not at the call site, so it holds for every caller of the
/// plan-file executor rather than for the one dispatcher that remembered.
///
/// `forjar plan --out` is deliberately NOT gated; see `cli::plan::cmd_plan`.
pub(crate) fn cmd_apply_from_plan(req: &PlanApplyRequest) -> Result<(), String> {
    // forjar#370: FIRST, and before the plan file is even read — the same
    // position and the same function `apply_execute` uses.
    super::dispatch_apply::check_operator_auth(req.file, req.operator)?;

    let config = prepare_config(req.file, req.env_file, req.workspace)?;
    let loaded = load_plan_file(req.plan_path, &config, req.state_dir)?;
    let scope = executor::PlanScope::from_plan(&loaded.plan);

    if req.verbose {
        report_plan(&loaded.plan, &scope, &loaded.selectors);
    }

    let fresh = replan(&config, req.state_dir, &loaded.selectors)?;
    check_plan_still_holds(&loaded.plan, &fresh, &loaded.selectors)?;

    if scope.is_empty() {
        return honour_an_empty_plan(&config, req.state_dir, loaded.sealed, &loaded.selectors);
    }

    check_selectors_narrow_the_plan(&scope, &config, &req.selectors)?;

    if req.dry_run {
        preview_scoped_plan(&loaded.plan, &scope, &config, &req.selectors);
        return Ok(());
    }

    execute_scoped_plan(&config, req, &scope)
}

#[cfg(test)]
#[path = "tests_apply_from_plan.rs"]
mod tests_apply_from_plan;
