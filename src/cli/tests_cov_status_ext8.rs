//! Coverage tests for status_operational_ext.rs, status_observability.rs.

#![allow(unused_imports)]
use super::status_operational_ext::*;
use super::status_observability::*;

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

    fn setup_state() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 2.5\n  mysql:\n    resource_type: Package\n    status: Failed\n    hash: def456\n  redis:\n    resource_type: Service\n    status: Drifted\n    hash: ghi789\n");
        write_yaml(dir.path(), "web1/events.jsonl", "{\"ts\":\"2026-01-01T00:00:00Z\",\"event\":\"resource_started\",\"resource\":\"nginx\",\"machine\":\"web1\"}\n{\"ts\":\"2026-01-01T00:01:00Z\",\"event\":\"resource_converged\",\"resource\":\"nginx\",\"machine\":\"web1\"}\n{\"ts\":\"2026-01-01T00:02:00Z\",\"event\":\"resource_failed\",\"resource\":\"mysql\",\"machine\":\"web1\"}\n{\"ts\":\"2026-01-01T01:00:00Z\",\"event\":\"resource_drifted\",\"resource\":\"redis\",\"machine\":\"web1\"}\n");
        dir
    }

    // status_operational_ext
    #[test]
    fn test_fleet_apply_success_rate_trend() {
        let d = setup_state();
        assert!(cmd_status_fleet_apply_success_rate_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fleet_apply_success_rate_trend_json() {
        let d = setup_state();
        assert!(cmd_status_fleet_apply_success_rate_trend(d.path(), None, true).is_ok());
    }
    #[test]
    fn test_machine_drift_flapping() {
        let d = setup_state();
        assert!(cmd_status_machine_resource_drift_flapping(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_machine_drift_flapping_json() {
        let d = setup_state();
        assert!(cmd_status_machine_resource_drift_flapping(d.path(), None, true).is_ok());
    }
    #[test]
    fn test_type_drift_heatmap() {
        let d = setup_state();
        assert!(cmd_status_fleet_resource_type_drift_heatmap(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_type_drift_heatmap_json() {
        let d = setup_state();
        assert!(cmd_status_fleet_resource_type_drift_heatmap(d.path(), None, true).is_ok());
    }
    #[test]
    fn test_operational_ext_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_apply_success_rate_trend(d.path(), None, false).is_ok());
    }

    // status_observability
    #[test]
    fn test_prometheus() {
        let d = setup_state();
        let _ = cmd_status_prometheus(d.path(), None);
    }
    #[test]
    fn test_prometheus_empty() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_prometheus(d.path(), None);
    }
    #[test]
    fn test_anomalies() {
        let d = setup_state();
        assert!(cmd_status_anomalies(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_anomalies_json() {
        let d = setup_state();
        assert!(cmd_status_anomalies(d.path(), None, true).is_ok());
    }
    #[test]
    fn test_error_summary() {
        let d = setup_state();
        assert!(cmd_status_error_summary(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_error_summary_json() {
        let d = setup_state();
        assert!(cmd_status_error_summary(d.path(), None, true).is_ok());
    }
    #[test]
    fn test_error_summary_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_error_summary(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_export() {
        let d = setup_state();
        let out = tempfile::tempdir().unwrap();
        let out_path = out.path().join("export.json");
        assert!(cmd_status_export(d.path(), None, &out_path, false).is_ok());
    }
    #[test]
    fn test_export_json() {
        let d = setup_state();
        let out = tempfile::tempdir().unwrap();
        let out_path = out.path().join("export.json");
        assert!(cmd_status_export(d.path(), None, &out_path, true).is_ok());
    }
}
