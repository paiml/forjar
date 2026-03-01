//! Tests: Coverage for status_recovery, status_intelligence, status_diagnostics.

#![allow(unused_imports)]
use super::status_recovery::*;
use super::status_intelligence::*;
use super::status_diagnostics::*;

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

    /// StateLock YAML with mixed statuses (converged, drifted, failed).
    fn state_lock_yaml() -> &'static str {
        concat!(
            "schema: \"1.0\"\n",
            "machine: web\n",
            "hostname: web\n",
            "generated_at: \"2026-02-28T00:00:00Z\"\n",
            "generator: forjar\n",
            "blake3_version: \"1.8\"\n",
            "resources:\n",
            "  f:\n",
            "    type: file\n",
            "    status: converged\n",
            "    hash: \"blake3:abc\"\n",
            "    applied_at: \"2026-02-28T00:00:00Z\"\n",
            "    duration_seconds: 1.5\n",
            "  g:\n",
            "    type: service\n",
            "    status: drifted\n",
            "    hash: \"blake3:def\"\n",
            "  h:\n",
            "    type: package\n",
            "    status: failed\n",
            "    hash: \"blake3:ghi\"\n",
        )
    }

    fn make_state_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for m in &["web", "db"] {
            // discover_machines needs: <machine>/state.lock.yaml
            write_yaml(dir.path(), &format!("{}/state.lock.yaml", m), state_lock_yaml());
            // status_recovery + status_intelligence read: <machine>/lock.yaml
            write_yaml(dir.path(), &format!("{}/lock.yaml", m), state_lock_yaml());
            // status_diagnostics reads: <machine>.lock.yaml (flat)
            write_yaml(dir.path(), &format!("{}.lock.yaml", m), state_lock_yaml());
        }
        dir
    }

    /// State dir with events files and snapshot dirs for recovery tests.
    fn make_state_dir_with_events() -> tempfile::TempDir {
        let dir = make_state_dir();
        for m in &["web", "db"] {
            // events.yaml used by status_recovery for MTTR and apply-success-trend
            write_yaml(dir.path(), &format!("{}/events.yaml", m), "some event data\n");
            // events.jsonl used by status_diagnostics for apply history count and churn
            let ev1 = r#"{"event":"apply_complete","resource":"f","timestamp":"2026-02-28T01:00:00Z"}"#;
            let ev2 = r#"{"event":"resource_applied","resource":"f","timestamp":"2026-02-28T01:05:00Z"}"#;
            write_yaml(
                dir.path(),
                &format!("{}/events.jsonl", m),
                &format!("{ev1}\n{ev2}\n{ev1}\n"),
            );
            // snapshots dir used by rollback readiness
            std::fs::create_dir_all(dir.path().join(m).join("snapshots")).unwrap();
        }
        dir
    }

    // ========================================================================
    // status_recovery tests
    // ========================================================================

    // -- cmd_status_machine_error_budget --
    #[test]
    fn error_budget_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_error_budget(d.path(), None, false).is_ok());
    }

    #[test]
    fn error_budget_with_data_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_error_budget(d.path(), None, false).is_ok());
    }

    #[test]
    fn error_budget_with_data_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_error_budget(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_fleet_compliance_score --
    #[test]
    fn fleet_compliance_score_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_compliance_score(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_compliance_score_plain() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_compliance_score(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_compliance_score_json() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_compliance_score(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_mean_time_to_recovery --
    #[test]
    fn mttr_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_mean_time_to_recovery(d.path(), None, false).is_ok());
    }

    #[test]
    fn mttr_with_events_plain() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_machine_mean_time_to_recovery(d.path(), None, false).is_ok());
    }

    #[test]
    fn mttr_json() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_machine_mean_time_to_recovery(d.path(), Some("web"), true).is_ok());
    }

    #[test]
    fn mttr_no_events_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_mean_time_to_recovery(d.path(), None, false).is_ok());
    }

    // -- cmd_status_machine_resource_dependency_health --
    #[test]
    fn dep_health_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_health(d.path(), None, false).is_ok());
    }

    #[test]
    fn dep_health_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_dependency_health(d.path(), None, false).is_ok());
    }

    #[test]
    fn dep_health_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_dependency_health(d.path(), Some("db"), true).is_ok());
    }

    // -- cmd_status_fleet_resource_type_health --
    #[test]
    fn fleet_type_health_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_health(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_type_health_plain() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_type_health(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_type_health_json() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_type_health(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_convergence_rate --
    #[test]
    fn convergence_rate_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_rate(d.path(), None, false).is_ok());
    }

    #[test]
    fn convergence_rate_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_convergence_rate(d.path(), None, false).is_ok());
    }

    #[test]
    fn convergence_rate_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_convergence_rate(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_failure_correlation --
    #[test]
    fn failure_correlation_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_failure_correlation(d.path(), None, false).is_ok());
    }

    #[test]
    fn failure_correlation_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_failure_correlation(d.path(), None, false).is_ok());
    }

    #[test]
    fn failure_correlation_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_failure_correlation(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_fleet_resource_age_distribution --
    #[test]
    fn age_distribution_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_age_distribution(d.path(), None, false).is_ok());
    }

    #[test]
    fn age_distribution_plain() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_age_distribution(d.path(), None, false).is_ok());
    }

    #[test]
    fn age_distribution_json() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_age_distribution(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_rollback_readiness --
    #[test]
    fn rollback_readiness_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_rollback_readiness(d.path(), None, false).is_ok());
    }

    #[test]
    fn rollback_readiness_with_snapshots_plain() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_machine_resource_rollback_readiness(d.path(), None, false).is_ok());
    }

    #[test]
    fn rollback_readiness_json() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_machine_resource_rollback_readiness(d.path(), Some("web"), true).is_ok());
    }

    #[test]
    fn rollback_readiness_lock_only() {
        let d = make_state_dir(); // has lock but no snapshots
        assert!(cmd_status_machine_resource_rollback_readiness(d.path(), None, false).is_ok());
    }

    // -- cmd_status_machine_resource_health_trend --
    #[test]
    fn health_trend_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_health_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn health_trend_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_health_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn health_trend_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_health_trend(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_fleet_resource_drift_velocity --
    #[test]
    fn drift_velocity_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_velocity(d.path(), None, false).is_ok());
    }

    #[test]
    fn drift_velocity_plain() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_drift_velocity(d.path(), None, false).is_ok());
    }

    #[test]
    fn drift_velocity_json() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_drift_velocity(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_apply_success_trend --
    #[test]
    fn apply_success_trend_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_success_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn apply_success_trend_with_events_plain() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_machine_resource_apply_success_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn apply_success_trend_json() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_machine_resource_apply_success_trend(d.path(), Some("web"), true).is_ok());
    }

    #[test]
    fn apply_success_trend_no_events() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_apply_success_trend(d.path(), None, false).is_ok());
    }

}

