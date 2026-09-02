//! The `apply` preflight: every gate that runs before anything is mutated.
//!
//! Extracted from `apply.rs` (forjar#334): that file sat 137 lines over the
//! repo's 500-line ceiling, so the ratchet forbade it growing by even the one
//! gate this issue needed. The behaviour here is unchanged by the move.

use super::apply_helpers::run_hook;
use super::helpers_state::*;
use crate::core::{parser, planner, resolver, types};
use std::path::Path;

/// REFUSE BEFORE WRITING, NOT AFTER.
///
/// Everything after this can mutate the state dir — `check_pre_apply_drift`
/// persists `ResourceStatus::Drifted` — but the process lock is not acquired
/// until the executor runs, much later. So a concurrent apply used to do its
/// drift pass, rewrite state.lock.yaml and re-seal the `.b3` over the holder's
/// state, and only then be told the directory was locked. Measured: "error:
/// state directory is locked by PID N" with the lock file MUTATED by that very
/// run. (forjar#310.)
///
/// A read-only probe, not an early acquire — see `locked_by_other_live_pid` for
/// why. It does not replace the acquire; it only ensures the loser of the race
/// has not written anything first.
fn concurrent_writer_gate(state_dir: &Path) -> Result<(), String> {
    match crate::core::state::locked_by_other_live_pid(state_dir) {
        Some(msg) => Err(msg),
        None => Ok(()),
    }
}

/// REFUSE BEFORE MUTATING IF WE CANNOT RECORD WHAT WE DID.
///
/// `ensure_event_log_writable` was written for exactly this (FJ-266) and had
/// ZERO CALLERS — its own doc comment says "Call this in the apply preflight",
/// and nothing did. So a full disk, a read-only state dir or a bad permission
/// produced an apply that MUTATED THE HOST and recorded nothing, behind a
/// stderr warning nobody reads.
///
/// An absent event is indistinguishable from an apply that never ran. That
/// ambiguity is what left paiml/infra#208 unattributable across three
/// toolchain deletions in one day.
///
/// Checked in the preflight, because stopping is still free at that point:
/// nothing has been changed yet.
///
/// Refs #368: takes the machines EXPLICITLY rather than filtering the config
/// itself, because `ensure_event_log_writable` creates `<state>/<machine>/` as a
/// side effect and the two callers scope differently. `apply --plan-file`
/// converges the machines the reviewed plan names, which is not
/// `config.machines` minus `-m`: passing the whole config there wrote a state
/// directory for a machine the plan deliberately excluded, which
/// `falsification_plan_file_scopes_the_apply` catches as "a machine outside the
/// plan must not even get a lock written".
fn event_log_gate(state_dir: &Path, machines: &[String]) -> Result<(), String> {
    for machine_name in machines {
        crate::tripwire::eventlog::ensure_event_log_writable(state_dir, machine_name)?;
    }
    Ok(())
}

/// The machines an ordinary apply under `machine_filter` will converge.
pub(super) fn machines_in_scope(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
) -> Vec<String> {
    config
        .machines
        .keys()
        .filter(|m| machine_filter.is_none_or(|f| f == m.as_str()))
        .cloned()
        .collect()
}

/// forjar#334: an ignored preview request is worse than a rejected one.
fn budget_preview_gate(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
) -> Result<(), String> {
    match super::apply_gates_budget::budget_dry_run_env_is_unhonoured(
        std::env::var("FORJAR_BUDGET_DRY_RUN").ok().as_deref(),
        super::apply_gates_budget::scope_holds_a_disk_budget(
            config,
            machine_filter,
            resource_filter,
            tag_filter,
        ),
    ) {
        Some(msg) => Err(msg),
        None => Ok(()),
    }
}

/// Refs #368: every gate that reads the STATE DIRECTORY, as one callable unit.
///
/// Extracted so a mode that is not `cmd_apply` can run it. `cmd_apply` was the
/// ONLY production caller of [`apply_pre_validate`], and
/// `dispatch_apply_b::apply_mode_exits` returns for `--plan-file` and for
/// `--refresh-only` BEFORE `apply_execute` ever reaches it — so the whole
/// preflight, the BLAKE3 integrity gate included, was skippable by choosing a
/// different apply mode. Measured on 1.24.0, `.b3` sidecar corrupted:
///
/// ```text
///   apply --yes                    -> error: state integrity check failed …
///                                     No apply flag overrides this check.
///   apply --plan-file p.json --yes -> Plan applied: 1 converged, 1 unchanged
/// ```
///
/// CONTIGUOUS BY CONSTRUCTION, and that is the point. This is exactly what
/// `apply_pre_validate` ran between its first line and `check_pre_apply_drift`,
/// in the same order, so the move is a pure re-spelling. It deliberately stops
/// short of `check_pre_apply_drift`, which WRITES `ResourceStatus::Drifted`
/// into the lock: a gate must not mutate state a later gate may refuse. An
/// extraction that swept the config gates in here too would necessarily hoist
/// them above the `--confirm-destructive` block, which would run an operator's
/// `pre_apply` hook on an apply that is then REFUSED — a side effect on the one
/// path whose entire purpose is refusing.
///
/// `machines` is the set this run will actually converge — see
/// [`event_log_gate`] for why the caller supplies it instead of the config
/// being filtered here.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_state_gates(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machines: &[String],
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    dry_run: bool,
    verbose: bool,
) -> Result<(), String> {
    concurrent_writer_gate(state_dir)?;
    // `--dry-run` is exempt from the two write-side gates: it mutates nothing,
    // so an unwritable log costs nothing, and failing it would make the
    // read-only inspection path depend on write access. The budget gate is
    // exempt because `--dry-run` is itself one of the answers it points at.
    if !dry_run {
        event_log_gate(state_dir, machines)?;
    }
    super::apply_gates::check_state_integrity(state_dir, verbose)?;
    if !dry_run {
        budget_preview_gate(config, machine_filter, resource_filter, tag_filter)?;
    }
    Ok(())
}

/// Refs #368: the two gates that guard a LOCK REWRITE specifically.
///
/// `cmd_refresh_only` takes the same early return out of `apply_mode_exits` and
/// calls `state::save_lock` in a loop with no gate at all, so it re-sealed a
/// tampered lock and LAUNDERED the integrity gate for the next ordinary apply.
/// Measured on 1.24.0, lock BODY tampered (not the sidecar):
///
/// ```text
///   apply --yes           -> error: state integrity check failed … No apply
///                            flag overrides this check.
///   apply --refresh-only  -> Refresh complete   (.b3 rewritten over the tamper)
///   apply --yes           -> Apply complete: 1 converged (1 repaired drift)
/// ```
///
/// A SUBSET of [`apply_state_gates`], not the whole of it, and the omissions
/// are deliberate: a refresh records no provenance event and evaluates no disk
/// budget, and `event_log_gate` would create a state directory for every
/// machine in the config — including machines a refresh will not touch, because
/// they have no lock — which is a new empty-machine-dir side effect that
/// `list_state_machines` would then report as state.
pub(super) fn lock_write_gates(state_dir: &Path, verbose: bool) -> Result<(), String> {
    concurrent_writer_gate(state_dir)?;
    super::apply_gates::check_state_integrity(state_dir, verbose)
}

/// Refs #368: every gate that reads only the CONFIG, as one callable unit.
///
/// The second contiguous half of [`apply_pre_validate`] — the policy engine,
/// `policy.security_gate` and the `policy.pre_apply` hook, in that order,
/// running AFTER the `--confirm-destructive` block for the reason given on
/// [`apply_state_gates`].
///
/// Measured on 1.24.0, the same config declaring the policy in both runs:
///
/// ```text
///   apply --yes                    -> error: policy violations block apply
///   apply --plan-file p.json --yes -> Plan applied: 1 converged, 1 unchanged
///   apply --yes                    -> error: security gate blocks apply
///   apply --plan-file p.json --yes -> Plan applied  (and the secret was WRITTEN)
/// ```
pub(super) fn apply_config_gates(
    config: &types::ForjarConfig,
    dry_run: bool,
    verbose: bool,
) -> Result<(), String> {
    check_policy_violations(config)?;
    check_security_gate(config)?;
    if let Some(ref hook) = config.policy.pre_apply {
        if !dry_run {
            run_hook("pre_apply", hook, verbose)?;
        }
    }
    Ok(())
}

/// Pre-apply validation: policies, confirmation, hooks.
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_pre_validate(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machine_filter: Option<&str>,
    tag_filter: Option<&str>,
    resource_filter: Option<&str>,
    confirm_destructive: bool,
    dry_run: bool,
    force: bool,
    yes: bool,
    verbose: bool,
) -> Result<Vec<super::apply_drift::DriftRepair>, String> {
    apply_state_gates(
        config,
        state_dir,
        &machines_in_scope(config, machine_filter),
        machine_filter,
        resource_filter,
        tag_filter,
        dry_run,
        verbose,
    )?;

    // forjar#336: the observation is CARRIED, not consumed. Before this it was
    // spent on an stderr line and a lock write and the function returned unit,
    // so the summary two frames later could not say why a resource converged.
    let observed_drift = super::apply_drift::check_pre_apply_drift(
        config,
        state_dir,
        machine_filter,
        force,
        dry_run,
        verbose,
    )?;

    // FJ-335: Confirm destructive actions
    if confirm_destructive && !dry_run && !yes {
        let order = resolver::build_execution_order(config)?;
        let cd_locks = load_machine_locks(config, state_dir, machine_filter)?;
        let plan = planner::plan(config, &order, &cd_locks, tag_filter);
        // GH-253: scoped, so `-r` on a non-destructive resource is not blocked
        // by destroys the operator did not select and apply would not perform.
        let (_, _, destroy_count) =
            super::apply_gates::scoped_action_counts(&plan.changes, resource_filter);
        if let Some(msg) = super::apply_gates::should_block_destructive(
            destroy_count,
            confirm_destructive,
            dry_run,
            yes,
        ) {
            eprintln!(
                "WARNING: {destroy_count} resource(s) will be DESTROYED. Use --yes to confirm."
            );
            return Err(msg);
        }
    }

    apply_config_gates(config, dry_run, verbose)?;

    // FJ-286: Confirmation prompt
    if !yes && !dry_run {
        let execution_order = resolver::build_execution_order(config)?;
        let preview_locks = load_machine_locks(config, state_dir, machine_filter)?;
        let preview_plan = planner::plan(config, &execution_order, &preview_locks, tag_filter);
        let (to_create, to_update, to_destroy) =
            super::apply_gates::scoped_action_counts(&preview_plan.changes, resource_filter);
        confirm_changes(to_create, to_update, to_destroy)?;
    }

    Ok(observed_drift)
}

/// FJ-286: ask before converging.
///
/// Refs #368: takes COUNTS, not a config, because the two callers count
/// different things. The ordinary path re-plans the whole config under the
/// operator's selectors; `apply --plan-file` counts the REVIEWED delta the
/// sealed document names, intersected with the scope and this invocation's
/// selectors — `planner::plan` knows nothing of a `PlanScope`, so re-planning
/// there would prompt about work the run will not do. One prompt, so the two
/// paths cannot drift apart in wording or in the meaning of "aborted".
pub(super) fn confirm_changes(
    to_create: usize,
    to_update: usize,
    to_destroy: usize,
) -> Result<(), String> {
    let n_changes = to_create + to_update + to_destroy;
    if n_changes == 0 {
        return Ok(());
    }
    eprint!(
        "Apply {n_changes} change(s) ({to_create} create, {to_update} update, {to_destroy} destroy)? [y/N] "
    );
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .map_err(|e| format!("stdin error: {e}"))?;
    if !answer.trim().eq_ignore_ascii_case("y") {
        return Err("aborted by user".to_string());
    }
    Ok(())
}

/// FJ-220 + FJ-3200: Check policy rules and block apply if any error-severity violations exist.
fn check_policy_violations(config: &types::ForjarConfig) -> Result<(), String> {
    if config.policies.is_empty() {
        return Ok(());
    }
    let result = parser::evaluate_policies_full(config);
    if !result.has_blocking_violations() {
        // Print warnings if any
        for v in &result.violations {
            eprintln!("  [WARN] {}: {}", v.resource_id, v.rule_message);
        }
        return Ok(());
    }
    for v in &result.violations {
        let sev = if v.is_blocking() { "DENY" } else { "WARN" };
        eprintln!("  [{sev}] {}: {}", v.resource_id, v.rule_message);
    }
    Err(format!(
        "policy violations block apply ({} error(s))",
        result.error_count()
    ))
}

/// FJ-1390: Run security scanner as pre-apply gate if policy.security_gate is set.
fn check_security_gate(config: &types::ForjarConfig) -> Result<(), String> {
    let threshold = match &config.policy.security_gate {
        Some(t) => t.clone(),
        None => return Ok(()),
    };
    let findings = crate::core::security_scanner::scan(config);
    if findings.is_empty() {
        return Ok(());
    }
    let (crit, high, med, _low) = crate::core::security_scanner::severity_counts(&findings);
    let should_fail = super::apply_gates::security_gate_should_block(
        &threshold,
        crit,
        high,
        med,
        findings.len(),
    )?;
    if !should_fail {
        return Ok(());
    }
    for f in &findings {
        eprintln!(
            "  [{:?}] {} ({}): {}",
            f.severity, f.rule_id, f.resource_id, f.message
        );
    }
    Err(format!(
        "security gate blocks apply: {} findings at or above '{threshold}'",
        findings.len()
    ))
}
