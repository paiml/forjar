//! FJ-3200: Extracted pure logic from CLI apply dispatch.
//!
//! Functions here are pure decision logic extracted from `apply.rs` to make
//! them testable without full CLI orchestration. The CLI remains a thin
//! routing shim that calls these functions.

use crate::core::types;
use indexmap::IndexMap;

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

/// Apply subset filter to a resource map — retain only matching resources.
///
/// Returns the count of retained resources, or Err if none match.
pub(crate) fn filter_subset(
    resources: &mut IndexMap<String, types::Resource>,
    pattern: &str,
) -> Result<usize, String> {
    resources.retain(|id, _| super::helpers_state::simple_glob_match(pattern, id));
    if resources.is_empty() {
        return Err(format!("no resources match subset pattern '{pattern}'"));
    }
    Ok(resources.len())
}

/// Apply exclude filter to a resource map — remove matching resources.
///
/// Returns the number of resources removed.
pub(crate) fn filter_exclude(
    resources: &mut IndexMap<String, types::Resource>,
    pattern: &str,
) -> usize {
    let before = resources.len();
    resources.retain(|id, _| !super::helpers_state::simple_glob_match(pattern, id));
    before - resources.len()
}

/// Determine whether a pre-apply drift gate should block.
///
/// Pure decision logic: given policy flags and drift count, decide whether to block.
pub(crate) fn should_block_on_drift(
    tripwire_enabled: bool,
    force: bool,
    drift_count: usize,
) -> Option<String> {
    if !tripwire_enabled || force {
        return None;
    }
    if drift_count > 0 {
        Some(format!(
            "{drift_count} drift finding(s) block apply — use --force to override"
        ))
    } else {
        None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Resource, ResourceType};

    fn make_resources(names: &[&str]) -> IndexMap<String, Resource> {
        let mut map = IndexMap::new();
        for name in names {
            map.insert(
                name.to_string(),
                Resource {
                    resource_type: ResourceType::Package,
                    ..Default::default()
                },
            );
        }
        map
    }

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

    // ── subset filter ──

    #[test]
    fn filter_subset_exact_match() {
        let mut resources = make_resources(&["nginx", "postgres", "redis"]);
        let count = filter_subset(&mut resources, "nginx").unwrap();
        assert_eq!(count, 1);
        assert!(resources.contains_key("nginx"));
    }

    #[test]
    fn filter_subset_wildcard() {
        let mut resources = make_resources(&["web-nginx", "web-apache", "db-postgres"]);
        let count = filter_subset(&mut resources, "web-*").unwrap();
        assert_eq!(count, 2);
        assert!(resources.contains_key("web-nginx"));
        assert!(resources.contains_key("web-apache"));
        assert!(!resources.contains_key("db-postgres"));
    }

    #[test]
    fn filter_subset_no_match() {
        let mut resources = make_resources(&["nginx", "postgres"]);
        let err = filter_subset(&mut resources, "missing-*").unwrap_err();
        assert!(err.contains("no resources match"));
    }

    #[test]
    fn filter_subset_star_matches_all() {
        let mut resources = make_resources(&["a", "b", "c"]);
        let count = filter_subset(&mut resources, "*").unwrap();
        assert_eq!(count, 3);
    }

    // ── exclude filter ──

    #[test]
    fn filter_exclude_removes_matching() {
        let mut resources = make_resources(&["nginx", "postgres", "redis"]);
        let removed = filter_exclude(&mut resources, "nginx");
        assert_eq!(removed, 1);
        assert_eq!(resources.len(), 2);
        assert!(!resources.contains_key("nginx"));
    }

    #[test]
    fn filter_exclude_wildcard() {
        let mut resources = make_resources(&["web-nginx", "web-apache", "db-postgres"]);
        let removed = filter_exclude(&mut resources, "web-*");
        assert_eq!(removed, 2);
        assert_eq!(resources.len(), 1);
        assert!(resources.contains_key("db-postgres"));
    }

    #[test]
    fn filter_exclude_no_match() {
        let mut resources = make_resources(&["nginx", "postgres"]);
        let removed = filter_exclude(&mut resources, "missing-*");
        assert_eq!(removed, 0);
        assert_eq!(resources.len(), 2);
    }

    // ── drift gate ──

    #[test]
    fn drift_gate_disabled() {
        assert!(should_block_on_drift(false, false, 5).is_none());
    }

    #[test]
    fn drift_gate_force_override() {
        assert!(should_block_on_drift(true, true, 5).is_none());
    }

    #[test]
    fn drift_gate_blocks() {
        let msg = should_block_on_drift(true, false, 3).unwrap();
        assert!(msg.contains("3 drift"));
    }

    #[test]
    fn drift_gate_no_drift() {
        assert!(should_block_on_drift(true, false, 0).is_none());
    }

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
