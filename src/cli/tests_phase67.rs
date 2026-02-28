//! Phase 67 tests: FJ-797 through FJ-804.

#[cfg(test)]
mod tests {
    use super::super::validate_safety::{
        cmd_validate_check_orphan_resources,
        cmd_validate_check_machine_arch,
    };
    use super::super::graph_advanced::{
        cmd_graph_strongly_connected,
        cmd_graph_dependency_matrix_csv,
    };
    use super::super::status_fleet_detail::{
        cmd_status_apply_success_rate,
        cmd_status_error_rate,
        cmd_status_fleet_health_summary,
    };

    fn yaml_header() -> &'static str {
        "version: \"1.0\"\nname: test\n"
    }

    fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let f = dir.join("forjar.yaml");
        std::fs::write(&f, format!("{}{}", yaml_header(), body)).unwrap();
        f
    }

    // ----- FJ-797: validate --check-orphan-resources -----

    #[test]
    fn test_fj797_no_orphans_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_orphan_resources(&f, false).is_ok());
    }

    #[test]
    fn test_fj797_all_orphans() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines: {}\nresources:\n  a:\n    type: package\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: package\n    provider: apt\n    packages: [vim]\n");
        assert!(cmd_validate_check_orphan_resources(&f, false).is_ok());
    }

    #[test]
    fn test_fj797_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_orphan_resources(&f, true).is_ok());
    }

    // ----- FJ-801: validate --check-machine-arch -----

    #[test]
    fn test_fj801_valid_archs() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n    arch: x86_64\n\
  arm:\n    hostname: arm\n    addr: 2.3.4.5\n    arch: aarch64\n\
resources: {}\n");
        assert!(cmd_validate_check_machine_arch(&f, false).is_ok());
    }

    #[test]
    fn test_fj801_default_arch() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\nresources: {}\n");
        assert!(cmd_validate_check_machine_arch(&f, false).is_ok());
    }

    #[test]
    fn test_fj801_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_machine_arch(&f, true).is_ok());
    }

    // ----- FJ-799: graph --strongly-connected -----

    #[test]
    fn test_fj799_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_strongly_connected(&f, false).is_ok());
    }

    #[test]
    fn test_fj799_linear() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n");
        assert!(cmd_graph_strongly_connected(&f, false).is_ok());
    }

    #[test]
    fn test_fj799_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_strongly_connected(&f, true).is_ok());
    }

    // ----- FJ-803: graph --dependency-matrix-csv -----

    #[test]
    fn test_fj803_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_dependency_matrix_csv(&f, false).is_ok());
    }

    #[test]
    fn test_fj803_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n");
        assert!(cmd_graph_dependency_matrix_csv(&f, false).is_ok());
    }

    #[test]
    fn test_fj803_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_dependency_matrix_csv(&f, true).is_ok());
    }

    // ----- FJ-800: status --apply-success-rate -----

    #[test]
    fn test_fj800_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_apply_success_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj800_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_apply_success_rate(dir.path(), None, true).is_ok());
    }

    // ----- FJ-802: status --error-rate -----

    #[test]
    fn test_fj802_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_error_rate(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj802_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_error_rate(dir.path(), None, true).is_ok());
    }

    // ----- FJ-804: status --fleet-health-summary -----

    #[test]
    fn test_fj804_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_health_summary(dir.path(), false).is_ok());
    }

    #[test]
    fn test_fj804_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_health_summary(dir.path(), true).is_ok());
    }
}
