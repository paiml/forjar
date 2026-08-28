//! FJ-1250 / Refs #356 / Refs #358 — `forjar apply --plan-file`.
//!
//! Executes a previously reviewed, sealed plan. Three things make this
//! different from an ordinary apply, and all three live here:
//!
//! * the plan's integrity is verified before anything runs
//!   (`plan_file::load_plan_file` → `core::plan_seal`);
//! * the plan's CLAIM is verified against a freshly computed plan
//!   (`check_plan_still_holds`), because integrity is not truth; and
//! * the executed set is the REVIEWED set — the scope derived from the plan
//!   body is what the executor is given, instead of the whole config.
//!
//! Lifted out of `apply_variants.rs`, which is a grab-bag of unrelated apply
//! modes and had no room left for any of it.

use super::apply_helpers::*;
use super::helpers::*;
use super::helpers_state::load_machine_locks;
use super::plan_file::{self, load_plan_file};
use super::workspace::*;
use crate::core::{executor, planner, resolver, types};
use std::collections::HashMap;
use std::path::Path;

/// Prefix on every refusal that comes from re-planning rather than from the
/// seal. `plan_seal` owns `PLAN_HASH_MISMATCH` / `PLAN_MALFORMED`; a plan whose
/// integrity is perfect and whose content is a lie is a different failure and
/// says so.
const PLAN_STALE: &str = "PLAN_STALE";

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

/// Refs #358: "this plan has no changes" is only an instruction worth obeying
/// when something vouches for the body that says it.
///
/// A `forjar-plan-v1` document's counters are unauthenticated JSON sitting
/// under a valid `config_hash`, so obeying a zero there is how a requested
/// apply prints a benign sentence and exits 0 having converged nothing — an
/// operator or CI job reading the exit code sees a successful apply over a
/// machine nothing was done to. A sealed body may legitimately say zero — but
/// only [`check_plan_still_holds`] decides whether it is TELLING THE TRUTH.
fn check_empty_plan_is_trustworthy(sealed: bool) -> Result<(), String> {
    if sealed {
        return Ok(());
    }
    Err(format!(
        "this '{}' plan file reports no changes, but its body is unsealed — forjar will \
         not report a successful apply on the word of an unauthenticated counter. \
         Re-run `forjar plan --out` to write a sealed '{}' plan.",
        plan_file::FORMAT_V1,
        plan_file::FORMAT_V2,
    ))
}

/// Recompute the plan from live inputs, with no filters.
///
/// Nearly free: `cmd_apply_from_plan` has already parsed and resolved the
/// config, the planner is pure over `(config, locks, probes)`, and the locks
/// are the same files `plan_seal`'s state leg has just read.
fn replan(config: &types::ForjarConfig, state_dir: &Path) -> Result<types::ExecutionPlan, String> {
    let locks = load_machine_locks(config, state_dir, None)?;
    let execution_order = resolver::build_execution_order(config)?;
    Ok(planner::plan(config, &execution_order, &locks, None))
}

/// Index a plan's changes by the `(machine, resource)` pair each one names.
fn by_pair(plan: &types::ExecutionPlan) -> HashMap<(&str, &str), &types::PlanAction> {
    plan.changes
        .iter()
        .map(|c| ((c.machine.as_str(), c.resource_id.as_str()), &c.action))
        .collect()
}

/// Refs #358 — the seal says a plan is UNEDITED. Re-planning says whether it is
/// TRUE, and only the second question is the one an operator is asking.
///
/// The seal is an unkeyed BLAKE3 hash. Anyone who can run `forjar` can compute
/// one, so no arrangement of hashing distinguishes a plan forjar issued from a
/// plan an adversary issued: copy `config_hash` and `state_hash` out of an
/// honest plan (neither leg has moved), empty the change list, zero the four
/// counters, recompute the diff leg and the composition through the public
/// `plan_seal::digest` API, and every check the seal can perform passes.
/// `check_body_partition` cannot help here — `0/0/0/0` over an EMPTY list
/// partitions trivially; it catches a plan that claims zero WHILE LISTING
/// several, and the attack simply empties the list.
///
/// What is checkable with no secret at all is the plan's claim. A plan file
/// asserts an action for each pair it names; the planner asserts one too, from
/// the live config and the live locks, and this command already holds both.
/// Where they disagree, the plan file is the one that is wrong — an adversary
/// cannot make the real planner return `NoOp` while a create is pending.
///
/// # Why the plan's pairs, and not the planner's
///
/// A saved plan may legitimately be NARROWER than the config: `plan -r`, `-m`,
/// `-g` and `-t` all filter the body while `config_hash` still covers the whole
/// file. Demanding that the freshly computed plan equal the saved one would
/// refuse every filtered plan. Demanding that the planner agree about the pairs
/// the saved plan actually names refuses none of them, and is strictly stronger
/// than a superset check on the change set, because it compares the ACTION too:
/// a plan reviewed as `create` that the planner now calls `destroy` is caught.
fn check_plan_still_holds(
    plan: &types::ExecutionPlan,
    fresh: &types::ExecutionPlan,
) -> Result<(), String> {
    let live = by_pair(fresh);
    for change in &plan.changes {
        let key = (change.machine.as_str(), change.resource_id.as_str());
        match live.get(&key) {
            Some(actual) if **actual == change.action => {}
            Some(actual) => {
                return Err(format!(
                    "{PLAN_STALE}: the plan file says {} for '{}' on '{}', but planning the \
                     live config against the live state says {actual}. A plan file is \
                     unauthenticated JSON — its seal proves it was not edited in transit, \
                     not that what it says is true — so forjar re-plans and believes the \
                     planner. Re-run `forjar plan --out` to write a plan that matches the \
                     world.",
                    change.action, change.resource_id, change.machine
                ));
            }
            None => {
                return Err(format!(
                    "{PLAN_STALE}: the plan file names '{}' on '{}', which planning the live \
                     config does not produce at all. Re-run `forjar plan --out`.",
                    change.resource_id, change.machine
                ));
            }
        }
    }
    check_an_empty_body_is_honest(plan, fresh)
}

/// A plan body naming NOTHING has nothing for [`check_plan_still_holds`] to
/// compare, so it needs its own clause — and it is exactly the shape the
/// re-sealing adversary produces.
///
/// An honest plan over a converged stack still LISTS its resources, as `NoOp`
/// entries under `unchanged`; a body with an empty `changes` array asserts that
/// the planner considered nothing. That is only true when the planner really
/// does find nothing, so the freshly computed plan is the arbiter. If anything
/// is still pending, obeying the file would print a successful apply over a
/// machine nothing examined.
fn check_an_empty_body_is_honest(
    plan: &types::ExecutionPlan,
    fresh: &types::ExecutionPlan,
) -> Result<(), String> {
    if !plan.changes.is_empty() {
        return Ok(());
    }
    let pending: Vec<String> = fresh
        .changes
        .iter()
        .filter(|c| c.action != types::PlanAction::NoOp)
        .map(|c| format!("{} on {} ({})", c.resource_id, c.machine, c.action))
        .collect();
    if pending.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{PLAN_STALE}: this plan file names no resources at all, yet planning the live \
         config against the live state finds {} change(s) still pending: {}. Obeying it \
         would print a successful apply over a machine nothing was examined on. Re-run \
         `forjar plan --out`.",
        pending.len(),
        pending.join(", ")
    ))
}

/// Refs #358: `-m` INTERSECTS the reviewed scope — it can only narrow it.
///
/// `--plan-file` executes the delta that was REVIEWED, so widening it with a
/// selector would be the defect this command was just fixed for: a resource
/// converged from a plan that never named it. Narrowing to a machine the plan
/// does touch is a legitimate staged rollout, and `executor` already intersects
/// `machine_filter` with the scope, so honouring `-m` is just passing it.
///
/// The EMPTY intersection is the case that earns a message. Converging nothing
/// and exiting 0 is how an operator asks for one machine, reads success, and
/// believes it happened — the same class of silent green as the zero-change
/// forgery above. Erroring costs them one re-run and names what the plan covers.
fn check_machine_is_in_scope(
    scope: &executor::PlanScope,
    machine: Option<&str>,
) -> Result<(), String> {
    let Some(name) = machine else {
        return Ok(());
    };
    if scope.covers_machine(name) {
        return Ok(());
    }
    Err(format!(
        "-m '{name}' names a machine this plan does not touch — the reviewed plan covers: \
         {}. `--plan-file` executes the reviewed delta, so -m can only narrow it, never \
         widen it; obeying this would converge nothing and still exit 0. Drop -m, name one \
         of the plan's machines, or re-run `forjar plan -m {name} --out`.",
        scope.machine_names().join(", ")
    ))
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
/// reviewed. `executor::apply_scoped` honours `dry_run` too (it returns before
/// `dispatch_apply`), but it returns a result carrying no changes, so asking it
/// would print "0 converged" and tell the operator nothing.
fn preview_scoped_plan(
    plan: &types::ExecutionPlan,
    scope: &executor::PlanScope,
    machine: Option<&str>,
) {
    println!("Dry run — the reviewed plan would execute:");
    let mut shown = 0usize;
    for change in &plan.changes {
        if !scope.covers(&change.machine, &change.resource_id) {
            continue;
        }
        if machine.is_some_and(|m| m != change.machine) {
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

/// Converge exactly the `(machine, resource)` pairs the reviewed plan named.
///
/// This function used to be inline under the comment "Execute as a normal apply
/// using the plan's resource list", which said the opposite of what it did —
/// with all selectors `None` and no scope, `executor::apply` converged the whole
/// current config and the plan body was decorative. None of those selectors CAN
/// express a plan: `resource_filter` is a single id, and a plan names a set of
/// pairs. That set is `scope`.
///
/// The selectors that stay `None` do so because a saved plan carries no request
/// for them — `resource_filter`, `tag_filter` and `group_filter` were already
/// applied when the plan body was written, and re-applying them here could only
/// narrow the reviewed set a second time. `machine_filter` is NOT one of them:
/// it comes from `-m` on this invocation and is honoured, after
/// `check_machine_is_in_scope` has established that it narrows the plan rather
/// than missing it entirely.
fn execute_scoped_plan(
    config: &types::ForjarConfig,
    state_dir: &Path,
    scope: &executor::PlanScope,
    machine: Option<&str>,
) -> Result<(), String> {
    let cfg = executor::ApplyConfig {
        config,
        state_dir,
        force: false,
        dry_run: false,
        machine_filter: machine,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: Some(types::generate_run_id()),
        refresh: false,
        force_tag: None,
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
fn report_plan(plan: &types::ExecutionPlan, scope: &executor::PlanScope) {
    let n_changes = plan.to_create + plan.to_update + plan.to_destroy;
    eprintln!(
        "Executing saved plan: {} changes ({} create, {} update, {} destroy)",
        n_changes, plan.to_create, plan.to_update, plan.to_destroy
    );
    eprintln!(
        "Plan scope: {} resource(s) across machine(s): {}",
        scope.len(),
        scope.machine_names().join(", ")
    );
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
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_apply_from_plan(
    file: &Path,
    state_dir: &Path,
    plan_path: &Path,
    verbose: bool,
    env_file: Option<&Path>,
    workspace: Option<&str>,
    operator: Option<&str>,
    dry_run: bool,
    machine: Option<&str>,
) -> Result<(), String> {
    // forjar#370: FIRST, and before the plan file is even read — the same
    // position and the same function `apply_execute` uses.
    super::dispatch_apply::check_operator_auth(file, operator)?;

    let config = prepare_config(file, env_file, workspace)?;
    let loaded = load_plan_file(plan_path, &config, state_dir)?;
    let scope = executor::PlanScope::from_plan(&loaded.plan);

    if verbose {
        report_plan(&loaded.plan, &scope);
    }

    check_plan_still_holds(&loaded.plan, &replan(&config, state_dir)?)?;

    if scope.is_empty() {
        check_empty_plan_is_trustworthy(loaded.sealed)?;
        println!("Plan has no changes to apply.");
        return Ok(());
    }

    check_machine_is_in_scope(&scope, machine)?;

    if dry_run {
        preview_scoped_plan(&loaded.plan, &scope, machine);
        return Ok(());
    }

    execute_scoped_plan(&config, state_dir, &scope, machine)
}

#[cfg(test)]
#[path = "tests_apply_from_plan.rs"]
mod tests_apply_from_plan;
