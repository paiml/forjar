//! Phase 63 tests: FJ-765 through FJ-772.

#[cfg(test)]
mod tests {
    use super::super::graph_export::{cmd_graph_in_degree, cmd_graph_longest_path};
    use super::super::status_diagnostics::{
        cmd_status_fleet_convergence, cmd_status_machine_drift_summary, cmd_status_resource_hash,
    };
    use super::super::validate_safety::{
        cmd_validate_check_provider_consistency, cmd_validate_check_state_values,
    };

    fn yaml_header() -> &'static str {
        "version: \"1.0\"\nname: test\n"
    }

    fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let f = dir.join("forjar.yaml");
        std::fs::write(&f, format!("{}{}", yaml_header(), body)).unwrap();
        f
    }

    // ----- FJ-765: validate --check-provider-consistency -----

    #[test]
    fn test_fj765_consistent_providers() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  pkg2:\n    type: package\n    machine: web\n    provider: apt\n    packages: [git]\n",
        );
        assert!(cmd_validate_check_provider_consistency(&f, false).is_ok());
    }

    #[test]
    fn test_fj765_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_provider_consistency(&f, false).is_ok());
    }

    #[test]
    fn test_fj765_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_provider_consistency(&f, true).is_ok());
    }

    // ----- FJ-769: validate --check-state-values -----

    #[test]
    fn test_fj769_valid_states() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  cfg:\n    type: file\n    machine: web\n    path: /etc/app\n    state: file\n",
        );
        assert!(cmd_validate_check_state_values(&f, false).is_ok());
    }

    #[test]
    fn test_fj769_no_state() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n");
        assert!(cmd_validate_check_state_values(&f, false).is_ok());
    }

    #[test]
    fn test_fj769_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_state_values(&f, true).is_ok());
    }

    // ----- FJ-767: graph --longest-path -----

    #[test]
    fn test_fj767_no_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n",
        );
        assert!(cmd_graph_longest_path(&f, false).is_ok());
    }

    #[test]
    fn test_fj767_chain() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n\
  c:\n    type: file\n    machine: web\n    path: /opt/c\n    depends_on: [b]\n",
        );
        assert!(cmd_graph_longest_path(&f, false).is_ok());
    }

    #[test]
    fn test_fj767_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_longest_path(&f, true).is_ok());
    }

    // ----- FJ-771: graph --in-degree -----

    #[test]
    fn test_fj771_no_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n",
        );
        assert!(cmd_graph_in_degree(&f, false).is_ok());
    }

    #[test]
    fn test_fj771_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  base:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  app1:\n    type: file\n    machine: web\n    path: /opt/a\n    depends_on: [base]\n\
  app2:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [base]\n",
        );
        assert!(cmd_graph_in_degree(&f, false).is_ok());
    }

    #[test]
    fn test_fj771_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_in_degree(&f, true).is_ok());
    }

    // ----- FJ-766: status --fleet-convergence -----

    #[test]
    fn test_fj766_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_convergence(dir.path(), false).is_ok());
    }

    #[test]
    fn test_fj766_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_convergence(dir.path(), true).is_ok());
    }

    // ----- FJ-770: status --resource-hash -----

    #[test]
    fn test_fj770_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_hash(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj770_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_hash(dir.path(), None, true).is_ok());
    }

    // ----- FJ-772: status --machine-drift-summary -----

    #[test]
    fn test_fj772_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_drift_summary(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj772_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_drift_summary(dir.path(), None, true).is_ok());
    }
}
