//! Coverage tests for status_intelligence.rs — MTTR, convergence forecast, budget forecast, dep lag.

#![allow(unused_imports)]
use super::status_intelligence::*;
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

    const LOCK_CONVERGED: &str = "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n";
    const LOCK_FAILED: &str = "resources:\n  nginx:\n    resource_type: Package\n    status: Failed\n    hash: abc123\n";
    const LOCK_MIXED: &str = "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc\n  mysql:\n    resource_type: Package\n    status: Failed\n    hash: def\n  redis:\n    resource_type: Service\n    status: Drifted\n    hash: ghi\n";

    const EVENTS: &str = r#"{"ts":"2026-01-01T00:00:00Z","event":"resource_failed","resource":"nginx","machine":"web1"}
{"ts":"2026-01-01T00:05:00Z","event":"resource_converged","resource":"nginx","machine":"web1"}
{"ts":"2026-01-01T01:00:00Z","event":"resource_drifted","resource":"mysql","machine":"web1"}
{"ts":"2026-01-01T01:10:00Z","event":"resource_converged","resource":"mysql","machine":"web1"}
"#;

    // FJ-910: MTTR estimates
    #[test]
    fn test_mttr_estimate_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_mttr_estimate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_mttr_estimate_no_events() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_resource_mttr_estimate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_mttr_estimate_with_events() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        write_yaml(dir.path(), "web1/events.jsonl", EVENTS);
        assert!(cmd_status_machine_resource_mttr_estimate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_mttr_estimate_with_failures() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_FAILED);
        write_yaml(dir.path(), "web1/events.jsonl", EVENTS);
        assert!(cmd_status_machine_resource_mttr_estimate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_mttr_estimate_failed_no_events() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_FAILED);
        assert!(cmd_status_machine_resource_mttr_estimate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_mttr_estimate_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        write_yaml(dir.path(), "web2/state.lock.yaml", LOCK_FAILED);
        assert!(cmd_status_machine_resource_mttr_estimate(dir.path(), Some("web1"), false).is_ok());
    }

    #[test]
    fn test_mttr_estimate_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        write_yaml(dir.path(), "web1/events.jsonl", EVENTS);
        assert!(cmd_status_machine_resource_mttr_estimate(dir.path(), None, true).is_ok());
    }

    // FJ-914: convergence forecast
    #[test]
    fn test_convergence_forecast_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_forecast(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_forecast_converged() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_fleet_resource_convergence_forecast(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_forecast_mixed() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_resource_convergence_forecast(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_forecast_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_resource_convergence_forecast(dir.path(), None, true).is_ok());
    }

    // FJ-916: error budget forecast
    #[test]
    fn test_budget_forecast_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_budget_forecast(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_budget_forecast_healthy() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_resource_error_budget_forecast(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_budget_forecast_failures() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_resource_error_budget_forecast(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_budget_forecast_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_resource_error_budget_forecast(dir.path(), None, true).is_ok());
    }

    // FJ-918: dependency lag
    #[test]
    fn test_dep_lag_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_dependency_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_dep_lag_converged() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        assert!(cmd_status_machine_resource_dependency_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_dep_lag_with_failures() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_resource_dependency_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_dep_lag_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_machine_resource_dependency_lag(dir.path(), None, true).is_ok());
    }

    // FJ-922: fleet dependency lag
    #[test]
    fn test_fleet_dep_lag_no_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_dependency_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fleet_dep_lag_multi() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_CONVERGED);
        write_yaml(dir.path(), "web2/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_resource_dependency_lag(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fleet_dep_lag_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", LOCK_MIXED);
        assert!(cmd_status_fleet_resource_dependency_lag(dir.path(), None, true).is_ok());
    }

    // forecast_label helper
    #[test]
    fn test_forecast_label_no_resources() {
        assert_eq!(forecast_label(0, 0), "no resources");
    }

    #[test]
    fn test_forecast_label_fully_converged() {
        assert_eq!(forecast_label(10, 10), "fully converged");
    }

    #[test]
    fn test_forecast_label_near() {
        assert_eq!(forecast_label(9, 10), "near convergence");
    }

    #[test]
    fn test_forecast_label_partial() {
        assert_eq!(forecast_label(6, 10), "partial convergence");
    }

    #[test]
    fn test_forecast_label_low() {
        assert_eq!(forecast_label(2, 10), "low convergence");
    }
}
