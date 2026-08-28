//! The `apply` preflight: every gate that runs before anything is mutated.
//!
//! Extracted from `apply.rs` (forjar#334): that file sat 137 lines over the
//! repo's 500-line ceiling, so the ratchet forbade it growing by even the one
//! gate this issue needed. The behaviour here is unchanged by the move.

use super::apply_helpers::run_hook;
use super::helpers_state::*;
use crate::core::{parser, planner, resolver, types};
use std::path::Path;

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
) -> Result<(), String> {
    // REFUSE BEFORE WRITING, NOT AFTER.
    //
    // Everything below this line can mutate the state dir — `check_pre_apply_drift`
    // persists `ResourceStatus::Drifted` — but the process lock is not acquired
    // until the executor runs, much later. So a concurrent apply used to do its
    // drift pass, rewrite state.lock.yaml and re-seal the `.b3` over the holder's
    // state, and only then be told the directory was locked. Measured: "error:
    // state directory is locked by PID N" with the lock file MUTATED by that very
    // run. (forjar#310.)
    //
    // A read-only probe, not an early acquire — see locked_by_other_live_pid for
    // why. It does not replace the acquire; it only ensures the loser of the race
    // has not written anything first.
    if let Some(msg) = crate::core::state::locked_by_other_live_pid(state_dir) {
        return Err(msg);
    }

    // REFUSE BEFORE MUTATING IF WE CANNOT RECORD WHAT WE DID.
    //
    // `ensure_event_log_writable` was written for exactly this (FJ-266) and had
    // ZERO CALLERS — its own doc comment says "Call this in the apply preflight",
    // and nothing did. So a full disk, a read-only state dir or a bad permission
    // produced an apply that MUTATED THE HOST and recorded nothing, behind a
    // stderr warning nobody reads.
    //
    // An absent event is indistinguishable from an apply that never ran. That
    // ambiguity is what left paiml/infra#208 unattributable across three
    // toolchain deletions in one day.
    //
    // Checked here, in the preflight, because stopping is still free at this
    // point: nothing has been changed yet. `--dry-run` is exempt — it mutates
    // nothing, so an unwritable log costs nothing, and failing it would make the
    // read-only inspection path depend on write access.
    if !dry_run {
        for machine_name in config.machines.keys() {
            if machine_filter.is_some_and(|m| m != machine_name) {
                continue;
            }
            crate::tripwire::eventlog::ensure_event_log_writable(state_dir, machine_name)?;
        }
    }
    super::apply_gates::check_state_integrity(state_dir, verbose)?;

    // forjar#334: an ignored preview request is worse than a rejected one.
    // `--dry-run` is exempt: it mutates nothing and is one of the two answers
    // this gate points at.
    if !dry_run {
        if let Some(msg) = super::apply_gates_budget::budget_dry_run_env_is_unhonoured(
            std::env::var("FORJAR_BUDGET_DRY_RUN").ok().as_deref(),
            super::apply_gates_budget::scope_holds_a_disk_budget(
                config,
                machine_filter,
                resource_filter,
                tag_filter,
            ),
        ) {
            return Err(msg);
        }
    }

    super::apply_drift::check_pre_apply_drift(
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

    check_policy_violations(config)?;
    check_security_gate(config)?;

    // Run pre_apply hook
    if let Some(ref hook) = config.policy.pre_apply {
        if !dry_run {
            run_hook("pre_apply", hook, verbose)?;
        }
    }

    // FJ-286: Confirmation prompt
    if !yes && !dry_run {
        let execution_order = resolver::build_execution_order(config)?;
        let preview_locks = load_machine_locks(config, state_dir, machine_filter)?;
        let preview_plan = planner::plan(config, &execution_order, &preview_locks, tag_filter);
        let (to_create, to_update, to_destroy) =
            super::apply_gates::scoped_action_counts(&preview_plan.changes, resource_filter);
        let n_changes = to_create + to_update + to_destroy;
        if n_changes > 0 {
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
        }
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
