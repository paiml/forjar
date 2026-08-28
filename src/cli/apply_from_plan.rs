//! FJ-1250 / Refs #356 / Refs #358 — `forjar apply --plan-file`.
//!
//! Executes a previously reviewed, sealed plan. The two things that make this
//! different from an ordinary apply both live here:
//!
//! * the plan's integrity is verified before anything runs
//!   (`plan_file::load_plan_file` → `core::plan_seal`), and
//! * the executed set is the REVIEWED set — the scope derived from the plan
//!   body is what the executor is given, instead of the whole config.
//!
//! Lifted out of `apply_variants.rs`, which is a grab-bag of unrelated apply
//! modes and had no room left for either concern.

use super::apply_helpers::*;
use super::helpers::*;
use super::plan_file::{self, load_plan_file};
use super::workspace::*;
use crate::core::{executor, resolver, types};
use std::path::Path;

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
/// machine nothing was done to. A sealed body may legitimately say zero.
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

/// Converge exactly the `(machine, resource)` pairs the reviewed plan named.
///
/// Every selector below stays `None` on purpose. This function used to be
/// inline under the comment "Execute as a normal apply using the plan's
/// resource list", which said the opposite of what it did — with all selectors
/// `None` and no scope, `executor::apply` converged the whole current config
/// and the plan body was decorative. None of those selectors CAN express a
/// plan: `resource_filter` is a single id, and a plan names a set of pairs.
/// That set is `scope`.
fn execute_scoped_plan(
    config: &types::ForjarConfig,
    state_dir: &Path,
    scope: &executor::PlanScope,
) -> Result<(), String> {
    let cfg = executor::ApplyConfig {
        config,
        state_dir,
        force: false,
        dry_run: false,
        machine_filter: None,
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

    if scope.is_empty() {
        check_empty_plan_is_trustworthy(loaded.sealed)?;
        println!("Plan has no changes to apply.");
        return Ok(());
    }

    execute_scoped_plan(&config, state_dir, &scope)
}

#[cfg(test)]
#[path = "tests_apply_from_plan.rs"]
mod tests_apply_from_plan;
