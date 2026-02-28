//! Phase 68 tests: FJ-805 through FJ-812.

#[cfg(test)]
mod tests {
    use super::super::validate_advanced::{
        cmd_validate_check_resource_health_conflicts,
        cmd_validate_check_resource_overlap,
    };
    use super::super::graph_advanced::{
        cmd_graph_resource_weight,
        cmd_graph_dependency_depth_per_resource,
    };
    use super::super::status_fleet_detail::{
        cmd_status_machine_convergence_history,
        cmd_status_drift_history,
        cmd_status_resource_failure_rate,
    };

    fn yaml_header() -> &'static str {
        "version: \"1.0\"\nname: test\n"
    }

    fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let f = dir.join("forjar.yaml");
        std::fs::write(&f, format!("{}{}", yaml_header(), body)).unwrap();
        f
    }

    // ----- FJ-805: validate --check-resource-health-conflicts -----

    #[test]
    fn test_fj805_no_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  svc:\n    type: service\n    machine: web\n    service_name: nginx\n    state: running\n");
        assert!(cmd_validate_check_resource_health_conflicts(&f, false).is_ok());
    }

    #[test]
    fn test_fj805_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_resource_health_conflicts(&f, false).is_ok());
    }

    #[test]
    fn test_fj805_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_resource_health_conflicts(&f, true).is_ok());
    }

    // ----- FJ-809: validate --check-resource-overlap -----

    #[test]
    fn test_fj809_no_overlaps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: file\n    machine: web\n    path: /opt/a\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n");
        assert!(cmd_validate_check_resource_overlap(&f, false).is_ok());
    }

    #[test]
    fn test_fj809_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_resource_overlap(&f, false).is_ok());
    }

    #[test]
    fn test_fj809_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_resource_overlap(&f, true).is_ok());
    }

    // ----- FJ-807: graph --resource-weight -----

    #[test]
    fn test_fj807_no_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n");
        assert!(cmd_graph_resource_weight(&f, false).is_ok());
    }

    #[test]
    fn test_fj807_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n\
  c:\n    type: file\n    machine: web\n    path: /opt/c\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_weight(&f, false).is_ok());
    }

    #[test]
    fn test_fj807_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_resource_weight(&f, true).is_ok());
    }

    // ----- FJ-811: graph --dependency-depth-per-resource -----

    #[test]
    fn test_fj811_no_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n");
        assert!(cmd_graph_dependency_depth_per_resource(&f, false).is_ok());
    }

    #[test]
    fn test_fj811_chain() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n\
  c:\n    type: file\n    machine: web\n    path: /opt/c\n    depends_on: [b]\n");
        assert!(cmd_graph_dependency_depth_per_resource(&f, false).is_ok());
    }

    #[test]
    fn test_fj811_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_dependency_depth_per_resource(&f, true).is_ok());
    }

    // ----- FJ-806: status --machine-convergence-history -----

    #[test]
    fn test_fj806_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_convergence_history(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj806_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_convergence_history(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_fj806_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_convergence_history(dir.path(), Some("web"), false).is_ok());
    }

    // ----- FJ-810: status --drift-history -----

    #[test]
    fn test_fj810_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_drift_history(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj810_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_drift_history(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_fj810_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_drift_history(dir.path(), Some("web"), false).is_ok());
    }

    // ----- FJ-812: status --resource-failure-rate -----

    #[test]
    fn test_fj812_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_failure_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj812_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_failure_rate(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_fj812_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_failure_rate(dir.path(), Some("web"), false).is_ok());
    }
}
