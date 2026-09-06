//! FJ-3200: Extracted pure logic from CLI apply dispatch.
//!
//! Functions here are pure decision logic extracted from `apply.rs` to make
//! them testable without full CLI orchestration. The CLI remains a thin
//! routing shim that calls these functions.

use crate::core::state;
use crate::core::types;
use std::path::Path;

/// FJ-1270: verify every lock file against its BLAKE3 `.b3` sidecar.
///
/// # No flag turns this off
///
/// It used to be written `has_errors(&issues) && !yes`. `--yes` is documented
/// as "skip confirmation prompt (CI mode)" and is *mandatory* for any
/// non-interactive apply, so tamper detection was off for exactly the runs
/// nobody watches: apply printed `ERROR: integrity check failed`, converged
/// over the corrupt lock, and exited 0. Prompting and integrity are separate
/// concerns and one flag must not decide both — so this takes no override
/// argument at all, and there is no flag to add one back.
///
/// # Why no escape hatch, rather than a differently-named one
///
/// The recovery path already exists and is narrower: `forjar reseal` re-seals
/// a lock whose contents the operator has decided are good, as one deliberate,
/// auditable act. An `--ignore-*` flag would be the weaker control — it says
/// nothing about the state it waves through, and it is the sort of flag a CI
/// job acquires permanently after one bad night, which is how the gate was
/// lost the first time. Measured: an override buys nothing for the corrupt-YAML
/// case either, because the lock then fails to parse at load
/// (`error: invalid lock file …`) whether or not the check ran.
pub(crate) fn check_state_integrity(state_dir: &Path, verbose: bool) -> Result<(), String> {
    if !state_dir.exists() {
        return Ok(());
    }
    let issues = state::integrity::verify_state_integrity(state_dir);
    state::integrity::print_issues(&issues, verbose);
    if !state::integrity::has_errors(&issues) {
        return Ok(());
    }
    Err(
        "state integrity check failed — the state file(s) above do not match their .b3 \
         sidecars, so forjar cannot vouch for what it would be converging against. \
         Restore the state (`forjar snapshot restore` / `forjar generation`), or if the \
         lock contents are known good, bless them with `forjar reseal --all` and apply \
         again. No apply flag overrides this check."
            .to_string(),
    )
}

/// Determine whether a convergence budget has been exceeded.
///
/// Returns `Ok(())` if no budget is set or the budget was not exceeded.
/// Returns `Err` with a message if the actual duration exceeds the budget.
pub(crate) fn check_convergence_budget_pure(
    budget_secs: Option<u64>,
    elapsed_secs: u64,
) -> Result<(), String> {
    if let Some(budget) = budget_secs {
        if elapsed_secs > budget {
            return Err(format!(
                "convergence budget exceeded: {elapsed_secs}s > {budget}s"
            ));
        }
    }
    Ok(())
}

/// Determine whether a security gate threshold is exceeded.
///
/// Given severity counts (critical, high, medium, low) and a threshold string,
/// returns whether the gate should block the apply.
pub(crate) fn security_gate_should_block(
    threshold: &str,
    critical: usize,
    high: usize,
    medium: usize,
    total: usize,
) -> Result<bool, String> {
    match threshold.to_lowercase().as_str() {
        "critical" => Ok(critical > 0),
        "high" => Ok(critical + high > 0),
        "medium" => Ok(critical + high + medium > 0),
        "low" => Ok(total > 0),
        _ => Err(format!("unknown security_gate severity: {threshold}")),
    }
}

/// Determine whether destructive actions should be blocked.
///
/// Returns `Some(message)` if destructive actions are blocked, `None` if they should proceed.
pub(crate) fn should_block_destructive(
    destroy_count: usize,
    confirm_destructive: bool,
    dry_run: bool,
    yes: bool,
) -> Option<String> {
    if !confirm_destructive || dry_run || yes || destroy_count == 0 {
        return None;
    }
    Some(format!(
        "{destroy_count} destructive action(s) blocked by --confirm-destructive"
    ))
}

/// Format a notification event JSON payload.
pub(crate) fn format_event_json(status: &str, config_path: &str) -> String {
    format!(r#"{{"event":"forjar_apply","status":"{status}","config":"{config_path}"}}"#)
}

/// Determine the notification status string from a Result.
pub(crate) fn notify_status(result: &Result<(), String>) -> &'static str {
    if result.is_ok() {
        "success"
    } else {
        "failure"
    }
}

/// Determine VictorOps status from a Result.
pub(crate) fn victorops_status(result: &Result<(), String>) -> (&'static str, &'static str) {
    if result.is_ok() {
        ("RECOVERY", "succeeded")
    } else {
        ("CRITICAL", "failed")
    }
}

/// Count how many snapshots to remove given total and keep threshold.
pub(crate) fn snapshots_to_remove(total: usize, keep: u32) -> usize {
    total.saturating_sub(keep as usize)
}

/// Determine parallel flag value from boolean.
pub(crate) fn parallel_flag(parallel: bool) -> Option<bool> {
    if parallel {
        Some(true)
    } else {
        None
    }
}

/// Count planned actions within the scope the executor will actually act on.
///
/// GH-253: `planner::plan` honours `tag_filter` but knows nothing about `-r`,
/// which is applied later by the executor
/// (`resource_ops.rs`: `cfg.resource_filter.is_some_and(|f| change.resource_id != f)`).
/// Counting the unscoped plan told an operator "Apply 69 change(s)" for
/// `-r stack-tool-forjar`, where `plan -r` promised 1 and apply acted on 1.
///
/// The predicate here is deliberately the same shape as the executor's, so the
/// number shown and the set acted on cannot drift apart again.
pub(crate) fn scoped_action_counts(
    changes: &[types::PlannedChange],
    resource_filter: Option<&str>,
) -> (usize, usize, usize) {
    let count = |action: types::PlanAction| {
        changes
            .iter()
            .filter(|c| resource_filter.is_none_or(|f| c.resource_id == f))
            .filter(|c| c.action == action)
            .count()
    };
    (
        count(types::PlanAction::Create),
        count(types::PlanAction::Update),
        count(types::PlanAction::Destroy),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── convergence budget ──

    #[test]
    fn budget_none_always_ok() {
        assert!(check_convergence_budget_pure(None, 999).is_ok());
    }

    #[test]
    fn budget_within_limit() {
        assert!(check_convergence_budget_pure(Some(60), 30).is_ok());
    }

    #[test]
    fn budget_at_limit() {
        assert!(check_convergence_budget_pure(Some(60), 60).is_ok());
    }

    #[test]
    fn budget_exceeded() {
        let err = check_convergence_budget_pure(Some(60), 90).unwrap_err();
        assert!(err.contains("90s > 60s"));
    }

    // ── security gate ──

    #[test]
    fn security_gate_critical_blocks() {
        assert!(security_gate_should_block("critical", 1, 0, 0, 1).unwrap());
    }

    #[test]
    fn security_gate_critical_passes() {
        assert!(!security_gate_should_block("critical", 0, 5, 10, 15).unwrap());
    }

    #[test]
    fn security_gate_high_blocks() {
        assert!(security_gate_should_block("high", 0, 1, 0, 1).unwrap());
    }

    #[test]
    fn security_gate_high_passes() {
        assert!(!security_gate_should_block("high", 0, 0, 5, 5).unwrap());
    }

    #[test]
    fn security_gate_medium_blocks() {
        assert!(security_gate_should_block("medium", 0, 0, 1, 1).unwrap());
    }

    #[test]
    fn security_gate_medium_passes() {
        assert!(!security_gate_should_block("medium", 0, 0, 0, 3).unwrap());
    }

    #[test]
    fn security_gate_low_blocks() {
        assert!(security_gate_should_block("low", 0, 0, 0, 1).unwrap());
    }

    #[test]
    fn security_gate_low_passes() {
        assert!(!security_gate_should_block("low", 0, 0, 0, 0).unwrap());
    }

    #[test]
    fn security_gate_unknown_severity() {
        let err = security_gate_should_block("extreme", 0, 0, 0, 0).unwrap_err();
        assert!(err.contains("unknown"));
    }

    #[test]
    fn security_gate_case_insensitive() {
        assert!(security_gate_should_block("CRITICAL", 1, 0, 0, 1).unwrap());
        assert!(security_gate_should_block("High", 0, 1, 0, 1).unwrap());
    }

    // ── drift gate ──

    // ── destructive gate ──

    #[test]
    fn destructive_blocks() {
        let msg = should_block_destructive(5, true, false, false).unwrap();
        assert!(msg.contains("5 destructive"));
    }

    #[test]
    fn destructive_not_confirmed() {
        assert!(should_block_destructive(5, false, false, false).is_none());
    }

    #[test]
    fn destructive_dry_run() {
        assert!(should_block_destructive(5, true, true, false).is_none());
    }

    #[test]
    fn destructive_yes_override() {
        assert!(should_block_destructive(5, true, false, true).is_none());
    }

    #[test]
    fn destructive_zero_count() {
        assert!(should_block_destructive(0, true, false, false).is_none());
    }

    // ── event JSON ──

    #[test]
    fn event_json_format() {
        let json = format_event_json("success", "/path/to/forjar.yaml");
        assert!(json.contains("forjar_apply"));
        assert!(json.contains("success"));
        assert!(json.contains("/path/to/forjar.yaml"));
    }

    // ── notify status ──

    #[test]
    fn notify_status_success() {
        assert_eq!(notify_status(&Ok(())), "success");
    }

    #[test]
    fn notify_status_failure() {
        assert_eq!(notify_status(&Err("boom".into())), "failure");
    }

    // ── victorops ──

    #[test]
    fn victorops_recovery_on_success() {
        let (status, verb) = victorops_status(&Ok(()));
        assert_eq!(status, "RECOVERY");
        assert_eq!(verb, "succeeded");
    }

    #[test]
    fn victorops_critical_on_failure() {
        let (status, verb) = victorops_status(&Err("err".into()));
        assert_eq!(status, "CRITICAL");
        assert_eq!(verb, "failed");
    }

    // ── snapshots ──

    #[test]
    fn snapshots_to_remove_within_limit() {
        assert_eq!(snapshots_to_remove(3, 5), 0);
    }

    #[test]
    fn snapshots_to_remove_at_limit() {
        assert_eq!(snapshots_to_remove(5, 5), 0);
    }

    #[test]
    fn snapshots_to_remove_exceeds() {
        assert_eq!(snapshots_to_remove(8, 5), 3);
    }

    // ── parallel flag ──

    #[test]
    fn parallel_flag_true() {
        assert_eq!(parallel_flag(true), Some(true));
    }

    #[test]
    fn parallel_flag_false() {
        assert_eq!(parallel_flag(false), None);
    }
}
