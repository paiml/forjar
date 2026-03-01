//! Phase 60 tests: FJ-741 through FJ-748.

#![allow(unused_imports)]
#[cfg(test)]
mod tests {
    use super::super::validate_paths::{
        cmd_validate_check_env_refs,
        cmd_validate_check_resource_names,
    };
    use super::super::graph_topology::{
        cmd_graph_subgraph_stats,
        cmd_graph_dependency_count,
    };
    use super::super::status_resource_detail::{
        cmd_status_last_apply_status,
        cmd_status_resource_staleness,
    };

    fn yaml_header() -> &'static str {
        "version: \"1.0\"\nname: test\n"
    }

    fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let f = dir.join("forjar.yaml");
        std::fs::write(&f, format!("{}{}", yaml_header(), body)).unwrap();
        f
    }


    // ----- FJ-741: validate --check-env-refs -----

    #[test]
    fn test_fj741_env_refs_no_refs() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_env_refs(&f, false).is_ok());
    }

    #[test]
    fn test_fj741_env_refs_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_env_refs(&f, true).is_ok());
    }

    #[test]
    #[allow(clippy::disallowed_methods)]
    fn test_fj741_env_refs_with_set_var() {
        std::env::set_var("FORJAR_TEST_741", "1");
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("f.yaml");
        std::fs::write(&f, "version: \"1.0\"\nname: test\nparams:\n  x: \"{{env.FORJAR_TEST_741}}\"\nmachines: {}\nresources: {}\n").unwrap();
        assert!(cmd_validate_check_env_refs(&f, false).is_ok());
        std::env::remove_var("FORJAR_TEST_741");
    }


    // ----- FJ-745: validate --check-resource-names -----

    #[test]
    fn test_fj745_resource_names_kebab() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  my-resource:\n    type: file\n    path: /tmp/a\n    content: a\n    machines: [m1]\n");
        assert!(cmd_validate_check_resource_names(&f, false, "kebab-case").is_ok());
    }

    #[test]
    fn test_fj745_resource_names_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  app-web:\n    type: file\n    path: /tmp/a\n    content: a\n    machines: [m1]\n");
        assert!(cmd_validate_check_resource_names(&f, false, "app-").is_ok());
    }

    #[test]
    fn test_fj745_resource_names_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_resource_names(&f, true, "kebab-case").is_ok());
    }


    // ----- FJ-743: graph --subgraph-stats -----

    #[test]
    fn test_fj743_subgraph_stats_single() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    path: /tmp/a\n    content: a\n    machines: [m1]\n  b:\n    type: file\n    path: /tmp/b\n    content: b\n    machines: [m1]\n    depends_on: [a]\n");
        assert!(cmd_graph_subgraph_stats(&f, false).is_ok());
    }

    #[test]
    fn test_fj743_subgraph_stats_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_subgraph_stats(&f, true).is_ok());
    }


    // ----- FJ-747: graph --dependency-count -----

    #[test]
    fn test_fj747_dependency_count() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    path: /tmp/a\n    content: a\n    machines: [m1]\n  b:\n    type: file\n    path: /tmp/b\n    content: b\n    machines: [m1]\n    depends_on: [a]\n");
        assert!(cmd_graph_dependency_count(&f, false).is_ok());
    }

    #[test]
    fn test_fj747_dependency_count_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_dependency_count(&f, true).is_ok());
    }


    // ----- FJ-746: status --last-apply-status -----

    #[test]
    fn test_fj746_last_apply_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_last_apply_status(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj746_last_apply_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_last_apply_status(dir.path(), None, true).is_ok());
    }


    // ----- FJ-748: status --resource-staleness -----

    #[test]
    fn test_fj748_staleness_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_staleness(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj748_staleness_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_resource_staleness(dir.path(), None, true).is_ok());
    }
}
