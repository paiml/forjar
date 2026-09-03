//! Refs #368 — the preflight for `apply --plan-file`.
//!
//! `cmd_apply` is the only production caller of
//! [`super::apply_preflight::apply_pre_validate`], and
//! `dispatch_apply_b::apply_mode_exits` returns for `--plan-file` before
//! `apply_execute` ever reaches it. So `cmd_apply_from_plan` ran
//! `check_operator_auth` (moved there by forjar#370) and then went straight to
//! `execute_scoped_plan` → `executor::apply_scoped`, which acquires the process
//! lock and plans and contains no policy, security, integrity or confirmation
//! gate at all. Measured on 1.24.0, every run using the SAME config that
//! declares the gate:
//!
//! ```text
//!   apply --yes                      -> Apply 1 change(s) … [y/N] aborted by user
//!   apply --plan-file p.json         -> Plan applied  (the file was DESTROYED, never asked)
//!
//!   apply --confirm-destructive      -> error: 1 destructive action(s) blocked
//!   apply --plan-file --confirm-destructive -> Plan applied
//!
//!   apply --yes  (deny policy)       -> error: policy violations block apply
//!   apply --plan-file --yes          -> Plan applied
//!
//!   apply --yes  (security_gate)     -> error: security gate blocks apply
//!   apply --plan-file --yes          -> Plan applied, secret WRITTEN
//!
//!   apply --yes  (pre_apply hook)    -> HOOK_RAN
//!   apply --plan-file --yes          -> hook never ran
//!
//!   apply --yes  (tampered .b3)      -> error: state integrity check failed
//!   apply --plan-file --yes          -> Plan applied
//! ```
//!
//! This is not a corner reachable only with a forged artifact: the plan is
//! produced legitimately by the ungated `forjar plan --out` from the config
//! that declares the policy, so the documented two-stage plan/review/apply
//! flow — the flow the feature exists for — ran with every gate off.
//!
//! The gates live here rather than inline in `apply_from_plan` so that file
//! stays under the repo's 500-line ceiling and so the ORDER, which mirrors
//! `apply_pre_validate` exactly, is stated once in one place.

use super::apply_from_plan::PlanApplyRequest;
use super::apply_from_plan_checks::survives;
use crate::core::plan_selectors::PlanSelectors;
use crate::core::{executor, types};

/// `(create, update, destroy)` over the delta this invocation will execute.
///
/// The reviewed plan BODY, narrowed twice: by the scope derived from that body
/// and by the selectors passed to THIS invocation — the same two predicates
/// `preview_scoped_plan` prints through and `execute_scoped_plan` runs under.
/// `apply_gates::scoped_action_counts` cannot be reused: it takes a single
/// `resource_filter` and re-plans the whole config, and a plan names a SET of
/// `(machine, resource)` pairs.
fn reviewed_action_counts(
    plan: &types::ExecutionPlan,
    scope: &executor::PlanScope,
    config: &types::ForjarConfig,
    selectors: &PlanSelectors,
) -> (usize, usize, usize) {
    let mut counts = (0usize, 0usize, 0usize);
    for change in &plan.changes {
        if !scope.covers(&change.machine, &change.resource_id) {
            continue;
        }
        if !survives(config, selectors, &change.machine, &change.resource_id) {
            continue;
        }
        match change.action {
            types::PlanAction::Create => counts.0 += 1,
            types::PlanAction::Update => counts.1 += 1,
            types::PlanAction::Destroy => counts.2 += 1,
            types::PlanAction::NoOp => {}
        }
    }
    counts
}

/// Every preflight gate an ordinary `apply` runs, in the order it runs them.
///
/// Called from `cmd_apply_from_plan` after `check_plan_still_holds` and
/// `check_selectors_narrow_the_plan`, and before the `--dry-run` branch. That
/// placement is deliberate: `check_pre_apply_drift` is NOT here, because it
/// WRITES `ResourceStatus::Drifted` into the lock, and a gate must not mutate
/// state a later gate may refuse.
///
/// KNOWN AND DELIBERATE GAP, so it is not later mistaken for coverage:
/// `honour_an_empty_plan` returns before this call, so a sealed zero-change
/// plan still runs no gate. Harmless — nothing converges — but real.
pub(super) fn run_plan_apply_gates(
    config: &types::ForjarConfig,
    req: &PlanApplyRequest,
    plan: &types::ExecutionPlan,
    scope: &executor::PlanScope,
) -> Result<(), String> {
    // The machines this run will converge are the ones the REVIEWED plan names,
    // narrowed by this invocation's `-m` — not `config.machines`. The event-log
    // gate creates `<state>/<machine>/`, and a plan written with `-m db` must
    // leave no trace of `web` (falsification_plan_file_scopes_the_apply).
    let machines: Vec<String> = scope
        .machine_names()
        .into_iter()
        .filter(|m| {
            req.selectors
                .machine
                .as_deref()
                .is_none_or(|f| f == m.as_str())
        })
        .collect();
    super::apply_preflight::apply_state_gates(
        config,
        req.state_dir,
        &machines,
        req.selectors.machine.as_deref(),
        req.selectors.resource.as_deref(),
        req.selectors.tag.as_deref(),
        req.dry_run,
        req.verbose,
    )?;

    let (to_create, to_update, to_destroy) =
        reviewed_action_counts(plan, scope, config, &req.selectors);

    // FJ-335: a HARD BLOCK, not a prompt — it returns without reading stdin.
    if let Some(msg) = super::apply_gates::should_block_destructive(
        to_destroy,
        req.confirm_destructive,
        req.dry_run,
        req.yes,
    ) {
        eprintln!("WARNING: {to_destroy} resource(s) will be DESTROYED. Use --yes to confirm.");
        return Err(msg);
    }

    super::apply_preflight::apply_config_gates(config, req.dry_run, req.verbose)?;

    // FJ-286. BREAKING for a pipeline that runs `apply --plan-file` over a
    // non-empty delta without `--yes`: it will now abort on EOF instead of
    // converging. That is the defect, not a regression — the measured
    // behaviour was a destroy nobody was asked about — and `--yes` is the
    // same one-flag answer every other non-interactive forjar apply already
    // needs.
    if !req.yes && !req.dry_run {
        super::apply_preflight::confirm_changes(to_create, to_update, to_destroy)?;
    }
    Ok(())
}
