//! Coverage tests for status_recovery.rs + status_recovery_b.rs.

#![allow(unused_imports)]
use super::status_recovery::*;
use super::status_recovery_b::*;
use std::io::Write as IoWrite;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
    }

    const LOCK_CONVERGED: &str = "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n";
    const LOCK_MIXED: &str = "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc\n    applied_at: '2025-01-01T00:00:00Z'\n  mysql:\n    resource_type: Package\n    status: Failed\n    hash: def\n  redis:\n    resource_type: Service\n    status: Drifted\n    hash: ghi\n";

    const EVENTS: &str = r#"{"ts":"2026-01-01T00:00:00Z","event":"resource_failed","resource":"nginx","machine":"web1"}
{"ts":"2026-01-01T00:05:00Z","event":"resource_converged","resource":"nginx","machine":"web1"}
{"ts":"2026-01-01T01:00:00Z","event":"resource_started","resource":"mysql","machine":"web1"}
{"ts":"2026-01-01T01:01:00Z","event":"resource_converged","resource":"mysql","machine":"web1"}
{"ts":"2026-01-01T02:00:00Z","event":"resource_failed","resource":"redis","machine":"web1"}
"#;

    // pct helper
    #[test]
    fn test_pct_normal() {
        assert!((pct(3, 10) - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_pct_zero_den() {
        assert!((pct(5, 0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_pct_full() {
        assert!((pct(10, 10) - 100.0).abs() < 0.01);
    }

    // FJ-878: error budget
    #[test]
    fn test_error_budget_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_error_budget(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_error_budget_healthy() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_error_budget(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_error_budget_failures() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_error_budget(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_error_budget_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_error_budget(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_error_budget_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        write_yaml(dir.path(), "web2/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_error_budget(dir.path(), Some("web1"), false).is_ok());
    }

    // FJ-882: fleet compliance
    #[test]
    fn test_fleet_compliance_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_compliance_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fleet_compliance_full() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_fleet_compliance_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fleet_compliance_mixed() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_compliance_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fleet_compliance_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_compliance_score(dir.path(), None, true).is_ok());
    }

    // FJ-884: MTTR
    #[test]
    fn test_mttr_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_mean_time_to_recovery(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_mttr_no_events() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_mean_time_to_recovery(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_mttr_with_events() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        write_yaml(dir.path(), "web1/events.jsonl", EVENTS);
        assert!(cmd_status_machine_mean_time_to_recovery(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_mttr_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        write_yaml(dir.path(), "web1/events.jsonl", EVENTS);
        assert!(cmd_status_machine_mean_time_to_recovery(dir.path(), None, true).is_ok());
    }

    // FJ-886: dependency health
    #[test]
    fn test_dep_health_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_dep_health_all_healthy() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_resource_dependency_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_dep_health_mixed() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_resource_dependency_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_dep_health_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_resource_dependency_health(dir.path(), None, true).is_ok());
    }

    // FJ-890: fleet type health
    #[test]
    fn test_fleet_type_health_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fleet_type_health_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_resource_type_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fleet_type_health_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_resource_type_health(dir.path(), None, true).is_ok());
    }

    // FJ-892: convergence rate
    #[test]
    fn test_convergence_rate_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_rate_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_resource_convergence_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_rate_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_resource_convergence_rate(dir.path(), None, true).is_ok());
    }

    // FJ-894: failure correlations
    #[test]
    fn test_failure_correlation_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_failure_correlation(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_failure_correlation_multi_machine() {
        let dir = tempfile::tempdir().unwrap();
        let lock_same_fail = "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc\n";
        write_yaml(dir.path(), "web1/state.lock.yaml", lock_same_fail);
        write_yaml(dir.path(), "web2/state.lock.yaml", lock_same_fail);
        assert!(cmd_status_machine_resource_failure_correlation(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_failure_correlation_json() {
        let dir = tempfile::tempdir().unwrap();
        let lock_same_fail = "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc\n";
        write_yaml(dir.path(), "web1/state.lock.yaml", lock_same_fail);
        write_yaml(dir.path(), "web2/state.lock.yaml", lock_same_fail);
        assert!(cmd_status_machine_resource_failure_correlation(dir.path(), None, true).is_ok());
    }

    // FJ-898: age distribution
    #[test]
    fn test_age_distribution_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_age_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_age_distribution_with_timestamps() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_fleet_resource_age_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_age_distribution_mixed() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_resource_age_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_age_distribution_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_fleet_resource_age_distribution(dir.path(), None, true).is_ok());
    }

    // FJ-900: rollback readiness
    #[test]
    fn test_rollback_readiness_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_rollback_readiness(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_rollback_readiness_lock_only() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_resource_rollback_readiness(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_rollback_readiness_with_snapshots() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        std::fs::create_dir_all(dir.path().join("web1/snapshots")).unwrap();
        assert!(cmd_status_machine_resource_rollback_readiness(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_rollback_readiness_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_resource_rollback_readiness(dir.path(), None, true).is_ok());
    }

    // FJ-902: health trends
    #[test]
    fn test_health_trends_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_health_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_health_trends_healthy() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_resource_health_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_health_trends_degraded() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_resource_health_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_health_trends_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_resource_health_trend(dir.path(), None, true).is_ok());
    }

    // FJ-906: drift velocity
    #[test]
    fn test_drift_velocity_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_velocity_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_resource_drift_velocity(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_velocity_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_resource_drift_velocity(dir.path(), None, true).is_ok());
    }

    // FJ-908: apply success trends
    #[test]
    fn test_apply_success_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_success_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_success_no_events() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_resource_apply_success_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_success_with_events() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        write_yaml(dir.path(), "web1/events.jsonl", EVENTS);
        assert!(cmd_status_machine_resource_apply_success_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_success_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        write_yaml(dir.path(), "web1/events.jsonl", EVENTS);
        assert!(cmd_status_machine_resource_apply_success_trend(dir.path(), None, true).is_ok());
    }
}
