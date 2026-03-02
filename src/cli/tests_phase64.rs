//! Phase 64 tests: FJ-773 through FJ-780.

#[cfg(test)]
mod tests {
    use super::super::graph_export::{cmd_graph_density, cmd_graph_out_degree};
    use super::super::status_diagnostics::{
        cmd_status_apply_history_count, cmd_status_lock_file_count,
        cmd_status_resource_type_distribution,
    };
    use super::super::validate_safety::{
        cmd_validate_check_tag_consistency, cmd_validate_check_unused_machines,
    };

    fn yaml_header() -> &'static str {
        "version: \"1.0\"\nname: test\n"
    }

    fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let f = dir.join("forjar.yaml");
        std::fs::write(&f, format!("{}{}", yaml_header(), body)).unwrap();
        f
    }

    // ----- FJ-773: validate --check-unused-machines -----

    #[test]
    fn test_fj773_all_used() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n");
        assert!(cmd_validate_check_unused_machines(&f, false).is_ok());
    }

    #[test]
    fn test_fj773_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_unused_machines(&f, false).is_ok());
    }

    #[test]
    fn test_fj773_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_unused_machines(&f, true).is_ok());
    }

    // ----- FJ-777: validate --check-tag-consistency -----

    #[test]
    fn test_fj777_no_tags() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines: {}\nresources:\n  pkg1:\n    type: package\n    provider: apt\n    packages: [curl]\n",
        );
        assert!(cmd_validate_check_tag_consistency(&f, false).is_ok());
    }

    #[test]
    fn test_fj777_valid_tags() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines: {}\nresources:\n  pkg1:\n    type: package\n    provider: apt\n    packages: [curl]\n    tags: [web-server, production]\n");
        assert!(cmd_validate_check_tag_consistency(&f, false).is_ok());
    }

    #[test]
    fn test_fj777_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_tag_consistency(&f, true).is_ok());
    }

    // ----- FJ-775: graph --out-degree -----

    #[test]
    fn test_fj775_no_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n",
        );
        assert!(cmd_graph_out_degree(&f, false).is_ok());
    }

    #[test]
    fn test_fj775_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_graph_out_degree(&f, false).is_ok());
    }

    #[test]
    fn test_fj775_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_out_degree(&f, true).is_ok());
    }

    // ----- FJ-779: graph --density -----

    #[test]
    fn test_fj779_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_density(&f, false).is_ok());
    }

    #[test]
    fn test_fj779_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_graph_density(&f, false).is_ok());
    }

    #[test]
    fn test_fj779_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_graph_density(&f, true).is_ok());
    }

    // ----- FJ-774: status --apply-history-count -----

    #[test]
    fn test_fj774_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_apply_history_count(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj774_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_apply_history_count(dir.path(), None, true).is_ok());
    }

    // ----- FJ-778: status --lock-file-count -----

    #[test]
    fn test_fj778_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_lock_file_count(dir.path(), false).is_ok());
    }

    #[test]
    fn test_fj778_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_lock_file_count(dir.path(), true).is_ok());
    }

    // ----- FJ-780: status --resource-type-distribution -----

    #[test]
    fn test_fj780_with_resources() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  cfg1:\n    type: file\n    machine: web\n    path: /etc/app.conf\n",
        );
        assert!(cmd_status_resource_type_distribution(&f, false).is_ok());
    }

    #[test]
    fn test_fj780_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_status_resource_type_distribution(&f, true).is_ok());
    }

    #[test]
    fn test_fj780_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_status_resource_type_distribution(&f, false).is_ok());
    }
}
