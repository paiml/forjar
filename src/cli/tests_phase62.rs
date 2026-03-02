//! Phase 62 tests: FJ-757 through FJ-764.

#[cfg(test)]
mod tests {
    use super::super::graph_export::{cmd_graph_adjacency_matrix, cmd_graph_connected_components};
    use super::super::status_diagnostics::{
        cmd_status_machine_resource_map, cmd_status_resource_duration,
    };
    use super::super::validate_safety::{
        cmd_validate_check_circular_deps, cmd_validate_check_machine_refs,
    };

    fn yaml_header() -> &'static str {
        "version: \"1.0\"\nname: test\n"
    }

    fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let f = dir.join("forjar.yaml");
        std::fs::write(&f, format!("{}{}", yaml_header(), body)).unwrap();
        f
    }

    // ----- FJ-757: validate --check-circular-deps -----

    #[test]
    fn test_fj757_no_cycles() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_validate_check_circular_deps(&f, false).is_ok());
    }

    #[test]
    fn test_fj757_empty_graph() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_circular_deps(&f, false).is_ok());
    }

    #[test]
    fn test_fj757_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_circular_deps(&f, true).is_ok());
    }

    // ----- FJ-761: validate --check-machine-refs -----

    #[test]
    fn test_fj761_valid_refs() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n");
        assert!(cmd_validate_check_machine_refs(&f, false).is_ok());
    }

    #[test]
    fn test_fj761_localhost_ok() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines: {}\n\
resources:\n  pkg1:\n    type: package\n    provider: apt\n    packages: [curl]\n",
        );
        assert!(cmd_validate_check_machine_refs(&f, false).is_ok());
    }

    #[test]
    fn test_fj761_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_machine_refs(&f, true).is_ok());
    }

    // ----- FJ-759: graph --connected-components -----

    #[test]
    fn test_fj759_single_component() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_graph_connected_components(&f, false).is_ok());
    }

    #[test]
    fn test_fj759_disconnected() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n",
        );
        assert!(cmd_graph_connected_components(&f, false).is_ok());
    }

    #[test]
    fn test_fj759_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_connected_components(&f, true).is_ok());
    }

    // ----- FJ-763: graph --adjacency-matrix -----

    #[test]
    fn test_fj763_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_adjacency_matrix(&f, false).is_ok());
    }

    #[test]
    fn test_fj763_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_graph_adjacency_matrix(&f, false).is_ok());
    }

    #[test]
    fn test_fj763_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_graph_adjacency_matrix(&f, true).is_ok());
    }

    // ----- FJ-762: status --resource-duration -----

    #[test]
    fn test_fj762_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_duration(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj762_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_duration(dir.path(), None, true).is_ok());
    }

    // ----- FJ-764: status --machine-resource-map -----

    #[test]
    fn test_fj764_with_resources() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  cfg1:\n    type: file\n    machine: web\n    path: /etc/app.conf\n",
        );
        assert!(cmd_status_machine_resource_map(&f, false).is_ok());
    }

    #[test]
    fn test_fj764_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_status_machine_resource_map(&f, true).is_ok());
    }

    #[test]
    fn test_fj764_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_status_machine_resource_map(&f, false).is_ok());
    }
}
