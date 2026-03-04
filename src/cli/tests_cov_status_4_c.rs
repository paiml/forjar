//! Tests: Coverage for status_diagnostics and edge cases (part 3).

use super::status_diagnostics::*;
use super::status_intelligence::*;
use super::status_recovery::*;

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

    fn state_lock_all_converged() -> &'static str {
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
        )
    }

    fn forjar_config_yaml() -> &'static str {
        concat!(
            "version: \"1.0\"\n",
            "name: test\n",
            "machines:\n",
            "  web:\n",
            "    hostname: web.example.com\n",
            "    addr: 10.0.0.1\n",
            "  db:\n",
            "    hostname: db.example.com\n",
            "    addr: 10.0.0.2\n",
            "resources:\n",
            "  nginx:\n",
            "    type: package\n",
            "    machine: web\n",
            "    packages:\n",
            "      - nginx\n",
            "  postgres:\n",
            "    type: package\n",
            "    machine: db\n",
            "    packages:\n",
            "      - postgresql\n",
            "  config:\n",
            "    type: file\n",
            "    machine: web\n",
            "    path: /etc/nginx/nginx.conf\n",
            "    content: \"server {}\"\n",
        )
    }

    fn make_state_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for m in &["web", "db"] {
            write_yaml(
                dir.path(),
                &format!("{m}/state.lock.yaml"),
                state_lock_yaml(),
            );
            write_yaml(dir.path(), &format!("{m}/state.lock.yaml"), state_lock_yaml());
            write_yaml(dir.path(), &format!("{m}/state.lock.yaml"), state_lock_yaml());
        }
        dir
    }

    fn make_state_dir_with_events() -> tempfile::TempDir {
        let dir = make_state_dir();
        for m in &["web", "db"] {
            write_yaml(
                dir.path(),
                &format!("{m}/events.jsonl"),
                "some event data\n",
            );
            let ev1 =
                r#"{"event":"apply_complete","resource":"f","timestamp":"2026-02-28T01:00:00Z"}"#;
            let ev2 =
                r#"{"event":"resource_applied","resource":"f","timestamp":"2026-02-28T01:05:00Z"}"#;
            write_yaml(
                dir.path(),
                &format!("{m}/events.jsonl"),
                &format!("{ev1}\n{ev2}\n{ev1}\n"),
            );
            std::fs::create_dir_all(dir.path().join(m).join("snapshots")).unwrap();
        }
        dir
    }

    // ========================================================================
    // status_diagnostics tests
    // ========================================================================

    // -- cmd_status_resource_duration --
    #[test]
    fn resource_duration_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_duration(d.path(), None, false).is_ok());
    }

    #[test]
    fn resource_duration_plain() {
        let d = make_state_dir();
        assert!(cmd_status_resource_duration(d.path(), None, false).is_ok());
    }

    #[test]
    fn resource_duration_json() {
        let d = make_state_dir();
        assert!(cmd_status_resource_duration(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_map --
    #[test]
    fn machine_resource_map_plain() {
        let d = tempfile::tempdir().unwrap();
        let config_path = d.path().join("forjar.yaml");
        std::fs::write(&config_path, forjar_config_yaml()).unwrap();
        assert!(cmd_status_machine_resource_map(&config_path, false).is_ok());
    }

    #[test]
    fn machine_resource_map_json() {
        let d = tempfile::tempdir().unwrap();
        let config_path = d.path().join("forjar.yaml");
        std::fs::write(&config_path, forjar_config_yaml()).unwrap();
        assert!(cmd_status_machine_resource_map(&config_path, true).is_ok());
    }

    #[test]
    fn machine_resource_map_missing_file() {
        let d = tempfile::tempdir().unwrap();
        let config_path = d.path().join("nonexistent.yaml");
        assert!(cmd_status_machine_resource_map(&config_path, false).is_err());
    }

    // -- cmd_status_fleet_convergence --
    #[test]
    fn fleet_convergence_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_convergence(d.path(), false).is_ok());
    }

    #[test]
    fn fleet_convergence_plain() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_convergence(d.path(), false).is_ok());
    }

    #[test]
    fn fleet_convergence_json() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_convergence(d.path(), true).is_ok());
    }

    // -- cmd_status_resource_hash --
    #[test]
    fn resource_hash_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_hash(d.path(), None, false).is_ok());
    }

    #[test]
    fn resource_hash_plain() {
        let d = make_state_dir();
        assert!(cmd_status_resource_hash(d.path(), None, false).is_ok());
    }

    #[test]
    fn resource_hash_json() {
        let d = make_state_dir();
        assert!(cmd_status_resource_hash(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_drift_summary --
    #[test]
    fn machine_drift_summary_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_drift_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn machine_drift_summary_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_drift_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn machine_drift_summary_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_drift_summary(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_apply_history_count --
    #[test]
    fn apply_history_count_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_apply_history_count(d.path(), None, false).is_ok());
    }

    #[test]
    fn apply_history_count_with_events_plain() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_apply_history_count(d.path(), None, false).is_ok());
    }

    #[test]
    fn apply_history_count_json() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_apply_history_count(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_lock_file_count --
    #[test]
    fn lock_file_count_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_lock_file_count(d.path(), false).is_ok());
    }

    #[test]
    fn lock_file_count_plain() {
        let d = make_state_dir();
        assert!(cmd_status_lock_file_count(d.path(), false).is_ok());
    }

    #[test]
    fn lock_file_count_json() {
        let d = make_state_dir();
        assert!(cmd_status_lock_file_count(d.path(), true).is_ok());
    }

    // -- cmd_status_resource_type_distribution --
    #[test]
    fn resource_type_dist_plain() {
        let d = tempfile::tempdir().unwrap();
        let config_path = d.path().join("forjar.yaml");
        std::fs::write(&config_path, forjar_config_yaml()).unwrap();
        assert!(cmd_status_resource_type_distribution(&config_path, false).is_ok());
    }

    #[test]
    fn resource_type_dist_json() {
        let d = tempfile::tempdir().unwrap();
        let config_path = d.path().join("forjar.yaml");
        std::fs::write(&config_path, forjar_config_yaml()).unwrap();
        assert!(cmd_status_resource_type_distribution(&config_path, true).is_ok());
    }

    #[test]
    fn resource_type_dist_missing_file() {
        let d = tempfile::tempdir().unwrap();
        let config_path = d.path().join("missing.yaml");
        assert!(cmd_status_resource_type_distribution(&config_path, false).is_err());
    }

    // -- cmd_status_resource_apply_age --
    #[test]
    fn resource_apply_age_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_apply_age(d.path(), None, false).is_ok());
    }

    #[test]
    fn resource_apply_age_plain() {
        let d = make_state_dir();
        assert!(cmd_status_resource_apply_age(d.path(), None, false).is_ok());
    }

    #[test]
    fn resource_apply_age_json() {
        let d = make_state_dir();
        assert!(cmd_status_resource_apply_age(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_uptime --
    #[test]
    fn machine_uptime_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_uptime(d.path(), None, false).is_ok());
    }

    #[test]
    fn machine_uptime_with_events_plain() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_machine_uptime(d.path(), None, false).is_ok());
    }

    #[test]
    fn machine_uptime_json() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_machine_uptime(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_resource_churn --
    #[test]
    fn resource_churn_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_churn(d.path(), None, false).is_ok());
    }

    #[test]
    fn resource_churn_with_events_plain() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_resource_churn(d.path(), None, false).is_ok());
    }

    #[test]
    fn resource_churn_json() {
        let d = make_state_dir_with_events();
        assert!(cmd_status_resource_churn(d.path(), Some("web"), true).is_ok());
    }

    // ========================================================================
    // Additional edge-case tests for better branch coverage
    // ========================================================================

    #[test]
    fn error_budget_filter_nonexistent_machine() {
        let d = make_state_dir();
        assert!(cmd_status_machine_error_budget(d.path(), Some("nonexistent"), false).is_ok());
    }

    #[test]
    fn fleet_compliance_all_converged() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", state_lock_all_converged());
        write_yaml(d.path(), "web/lock.yaml", state_lock_all_converged());
        assert!(cmd_status_fleet_compliance_score(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_dep_lag_no_failures() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", state_lock_all_converged());
        write_yaml(d.path(), "web/lock.yaml", state_lock_all_converged());
        assert!(cmd_status_fleet_resource_dependency_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn convergence_velocity_with_state_lock() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_convergence_velocity(d.path(), None, true).is_ok());
    }

    #[test]
    fn failure_recurrence_with_state_lock() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_failure_recurrence(d.path(), None, true).is_ok());
    }

    #[test]
    fn fleet_convergence_velocity_filtered() {
        let d = make_state_dir();
        assert!(
            cmd_status_fleet_resource_convergence_velocity(d.path(), Some("db"), false).is_ok()
        );
    }

    #[test]
    fn machine_uptime_with_timestamp_events() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", state_lock_yaml());
        let event = r#"{"event":"apply_complete","timestamp":"2026-01-15T10:30:00Z"}"#;
        write_yaml(d.path(), "web/events.jsonl", &format!("{event}\n"));
        assert!(cmd_status_machine_uptime(d.path(), None, false).is_ok());
    }

    #[test]
    fn resource_churn_with_resource_applied() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", state_lock_yaml());
        let ev =
            r#"{"event":"resource_applied","resource":"nginx","timestamp":"2026-02-28T01:00:00Z"}"#;
        write_yaml(d.path(), "web/events.jsonl", &format!("{ev}\n{ev}\n{ev}\n"));
        assert!(cmd_status_resource_churn(d.path(), None, true).is_ok());
    }

    #[test]
    fn rollback_readiness_no_lock_machine() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", state_lock_yaml());
        assert!(cmd_status_machine_resource_rollback_readiness(d.path(), None, false).is_ok());
    }

    #[test]
    fn config_drift_rate_all_converged() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", state_lock_all_converged());
        write_yaml(d.path(), "web/lock.yaml", state_lock_all_converged());
        assert!(cmd_status_machine_resource_config_drift_rate(d.path(), None, false).is_ok());
    }

    #[test]
    fn apply_history_count_no_events() {
        let d = make_state_dir();
        assert!(cmd_status_apply_history_count(d.path(), None, false).is_ok());
    }

    #[test]
    fn convergence_lag_all_converged() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", state_lock_all_converged());
        write_yaml(d.path(), "web/lock.yaml", state_lock_all_converged());
        assert!(cmd_status_machine_resource_convergence_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_convergence_lag_all_converged() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", state_lock_all_converged());
        write_yaml(d.path(), "web/lock.yaml", state_lock_all_converged());
        assert!(cmd_status_fleet_resource_convergence_lag(d.path(), None, true).is_ok());
    }

    #[test]
    fn dependency_depth_filtered_machine() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_dependency_depth(d.path(), Some("db"), true).is_ok());
    }

    #[test]
    fn resource_duration_filtered() {
        let d = make_state_dir();
        assert!(cmd_status_resource_duration(d.path(), Some("db"), false).is_ok());
    }

    #[test]
    fn resource_hash_filtered() {
        let d = make_state_dir();
        assert!(cmd_status_resource_hash(d.path(), Some("db"), false).is_ok());
    }

    #[test]
    fn machine_drift_summary_filtered() {
        let d = make_state_dir();
        assert!(cmd_status_machine_drift_summary(d.path(), Some("db"), false).is_ok());
    }
}
