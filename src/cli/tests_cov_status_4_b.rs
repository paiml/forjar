//! Tests: Coverage for status_intelligence (part 2).

use super::status_recovery::*;
use super::status_intelligence::*;

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

    /// StateLock YAML with all converged (no failures, no drift).
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

    fn make_state_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for m in &["web", "db"] {
            write_yaml(dir.path(), &format!("{}/state.lock.yaml", m), state_lock_yaml());
            write_yaml(dir.path(), &format!("{}/lock.yaml", m), state_lock_yaml());
            write_yaml(dir.path(), &format!("{}.lock.yaml", m), state_lock_yaml());
        }
        dir
    }

    // ========================================================================
    // status_intelligence tests
    // ========================================================================

    // -- cmd_status_machine_resource_mttr_estimate --
    #[test]
    fn mttr_estimate_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_mttr_estimate(d.path(), None, false).is_ok());
    }

    #[test]
    fn mttr_estimate_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_mttr_estimate(d.path(), None, false).is_ok());
    }

    #[test]
    fn mttr_estimate_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_mttr_estimate(d.path(), Some("web"), true).is_ok());
    }

    #[test]
    fn mttr_estimate_all_converged() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", state_lock_all_converged());
        write_yaml(d.path(), "web/lock.yaml", state_lock_all_converged());
        assert!(cmd_status_machine_resource_mttr_estimate(d.path(), Some("web"), false).is_ok());
    }

    // -- cmd_status_fleet_resource_convergence_forecast --
    #[test]
    fn convergence_forecast_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_forecast(d.path(), None, false).is_ok());
    }

    #[test]
    fn convergence_forecast_plain() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_convergence_forecast(d.path(), None, false).is_ok());
    }

    #[test]
    fn convergence_forecast_json() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_convergence_forecast(d.path(), Some("web"), true).is_ok());
    }

    #[test]
    fn convergence_forecast_all_converged() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", state_lock_all_converged());
        write_yaml(d.path(), "web/lock.yaml", state_lock_all_converged());
        assert!(cmd_status_fleet_resource_convergence_forecast(d.path(), None, false).is_ok());
    }

    // -- cmd_status_machine_resource_error_budget_forecast --
    #[test]
    fn error_budget_forecast_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_budget_forecast(d.path(), None, false).is_ok());
    }

    #[test]
    fn error_budget_forecast_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_error_budget_forecast(d.path(), None, false).is_ok());
    }

    #[test]
    fn error_budget_forecast_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_error_budget_forecast(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_dependency_lag --
    #[test]
    fn dependency_lag_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn dependency_lag_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_dependency_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn dependency_lag_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_dependency_lag(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_fleet_resource_dependency_lag --
    #[test]
    fn fleet_dep_lag_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_dependency_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_dep_lag_plain() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_dependency_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_dep_lag_json() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_dependency_lag(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_config_drift_rate --
    #[test]
    fn config_drift_rate_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_config_drift_rate(d.path(), None, false).is_ok());
    }

    #[test]
    fn config_drift_rate_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_config_drift_rate(d.path(), None, false).is_ok());
    }

    #[test]
    fn config_drift_rate_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_config_drift_rate(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_convergence_lag --
    #[test]
    fn convergence_lag_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn convergence_lag_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_convergence_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn convergence_lag_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_convergence_lag(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_fleet_resource_convergence_lag --
    #[test]
    fn fleet_convergence_lag_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_convergence_lag_plain() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_convergence_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_convergence_lag_json() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_convergence_lag(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_dependency_depth --
    #[test]
    fn dependency_depth_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_depth(d.path(), None, false).is_ok());
    }

    #[test]
    fn dependency_depth_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_dependency_depth(d.path(), None, false).is_ok());
    }

    #[test]
    fn dependency_depth_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_dependency_depth(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_convergence_velocity --
    #[test]
    fn convergence_velocity_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_velocity(d.path(), None, false).is_ok());
    }

    #[test]
    fn convergence_velocity_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_convergence_velocity(d.path(), None, false).is_ok());
    }

    #[test]
    fn convergence_velocity_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_convergence_velocity(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_fleet_resource_convergence_velocity --
    #[test]
    fn fleet_convergence_velocity_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_velocity(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_convergence_velocity_plain() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_convergence_velocity(d.path(), None, false).is_ok());
    }

    #[test]
    fn fleet_convergence_velocity_json() {
        let d = make_state_dir();
        assert!(cmd_status_fleet_resource_convergence_velocity(d.path(), Some("web"), true).is_ok());
    }

    // -- cmd_status_machine_resource_failure_recurrence --
    #[test]
    fn failure_recurrence_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_failure_recurrence(d.path(), None, false).is_ok());
    }

    #[test]
    fn failure_recurrence_plain() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_failure_recurrence(d.path(), None, false).is_ok());
    }

    #[test]
    fn failure_recurrence_json() {
        let d = make_state_dir();
        assert!(cmd_status_machine_resource_failure_recurrence(d.path(), Some("web"), true).is_ok());
    }

}
