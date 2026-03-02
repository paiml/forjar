//! Phase 65 tests: FJ-781 through FJ-788.

#[cfg(test)]
mod tests {
    use super::super::graph_export::{
        cmd_graph_critical_path_resources, cmd_graph_topological_sort,
    };
    use super::super::status_diagnostics::{
        cmd_status_machine_uptime, cmd_status_resource_apply_age, cmd_status_resource_churn,
    };
    use super::super::validate_safety::{
        cmd_validate_check_dependency_exists, cmd_validate_check_path_conflicts_strict,
    };

    fn yaml_header() -> &'static str {
        "version: \"1.0\"\nname: test\n"
    }

    fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let f = dir.join("forjar.yaml");
        std::fs::write(&f, format!("{}{}", yaml_header(), body)).unwrap();
        f
    }

    // ----- FJ-781: validate --check-dependency-exists -----

    #[test]
    fn test_fj781_all_valid() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_validate_check_dependency_exists(&f, false).is_ok());
    }

    #[test]
    fn test_fj781_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_dependency_exists(&f, false).is_ok());
    }

    #[test]
    fn test_fj781_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_dependency_exists(&f, true).is_ok());
    }

    // ----- FJ-785: validate --check-path-conflicts-strict -----

    #[test]
    fn test_fj785_no_conflicts() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: file\n    machine: web\n    path: /opt/a\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n",
        );
        assert!(cmd_validate_check_path_conflicts_strict(&f, false).is_ok());
    }

    #[test]
    fn test_fj785_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_path_conflicts_strict(&f, false).is_ok());
    }

    #[test]
    fn test_fj785_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_path_conflicts_strict(&f, true).is_ok());
    }

    // ----- FJ-783: graph --topological-sort -----

    #[test]
    fn test_fj783_no_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n",
        );
        assert!(cmd_graph_topological_sort(&f, false).is_ok());
    }

    #[test]
    fn test_fj783_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_graph_topological_sort(&f, false).is_ok());
    }

    #[test]
    fn test_fj783_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_topological_sort(&f, true).is_ok());
    }

    // ----- FJ-787: graph --critical-path-resources -----

    #[test]
    fn test_fj787_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_critical_path_resources(&f, false).is_ok());
    }

    #[test]
    fn test_fj787_with_chain() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n\
  c:\n    type: file\n    machine: web\n    path: /opt/c\n    depends_on: [b]\n",
        );
        assert!(cmd_graph_critical_path_resources(&f, false).is_ok());
    }

    #[test]
    fn test_fj787_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_critical_path_resources(&f, true).is_ok());
    }

    // ----- FJ-782: status --resource-apply-age -----

    #[test]
    fn test_fj782_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_apply_age(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj782_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_apply_age(dir.path(), None, true).is_ok());
    }

    // ----- FJ-786: status --machine-uptime -----

    #[test]
    fn test_fj786_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_uptime(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj786_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_uptime(dir.path(), None, true).is_ok());
    }

    // ----- FJ-788: status --resource-churn -----

    #[test]
    fn test_fj788_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_churn(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj788_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_churn(dir.path(), None, true).is_ok());
    }
}
