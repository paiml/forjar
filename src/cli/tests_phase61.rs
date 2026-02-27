//! Phase 61 tests: FJ-749 through FJ-756.

#[cfg(test)]
mod tests {
    use super::super::validate_paths::{
        cmd_validate_check_resource_count,
        cmd_validate_check_duplicate_paths,
    };
    use super::super::graph_export::{
        cmd_graph_root_resources,
        cmd_graph_edge_list,
    };
    use super::super::status_counts::{
        cmd_status_convergence_percentage,
        cmd_status_failed_count,
        cmd_status_drift_count,
    };

    fn yaml_header() -> &'static str {
        "version: \"1.0\"\nname: test\n"
    }

    fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let f = dir.join("forjar.yaml");
        std::fs::write(&f, format!("{}{}", yaml_header(), body)).unwrap();
        f
    }

    // ----- FJ-749: validate --check-resource-count -----

    #[test]
    fn test_fj749_resource_count_under_limit() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n");
        assert!(cmd_validate_check_resource_count(&f, false, 5).is_ok());
    }

    #[test]
    fn test_fj749_resource_count_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n");
        assert!(cmd_validate_check_resource_count(&f, true, 5).is_ok());
    }

    #[test]
    fn test_fj749_resource_count_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_resource_count(&f, false, 1).is_ok());
    }

    // ----- FJ-753: validate --check-duplicate-paths -----

    #[test]
    fn test_fj753_no_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines: {}\n\
resources:\n  f1:\n    type: file\n    path: /etc/a\n  f2:\n    type: file\n    path: /etc/b\n");
        assert!(cmd_validate_check_duplicate_paths(&f, false).is_ok());
    }

    #[test]
    fn test_fj753_duplicates_detected() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines: {}\n\
resources:\n  f1:\n    type: file\n    path: /etc/same\n  f2:\n    type: file\n    path: /etc/same\n");
        assert!(cmd_validate_check_duplicate_paths(&f, false).is_ok());
    }

    #[test]
    fn test_fj753_duplicates_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines: {}\n\
resources:\n  f1:\n    type: file\n    path: /etc/same\n  f2:\n    type: file\n    path: /etc/same\n");
        assert!(cmd_validate_check_duplicate_paths(&f, true).is_ok());
    }

    // ----- FJ-751: graph --root-resources -----

    #[test]
    fn test_fj751_root_resources_all_roots() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  pkg2:\n    type: package\n    machine: web\n    provider: apt\n    packages: [git]\n");
        assert!(cmd_graph_root_resources(&f, false).is_ok());
    }

    #[test]
    fn test_fj751_root_resources_with_dep() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  base:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  app:\n    type: file\n    machine: web\n    path: /opt/app\n    depends_on: [base]\n");
        assert!(cmd_graph_root_resources(&f, false).is_ok());
    }

    #[test]
    fn test_fj751_root_resources_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_root_resources(&f, true).is_ok());
    }

    // ----- FJ-755: graph --edge-list -----

    #[test]
    fn test_fj755_edge_list_no_edges() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n");
        assert!(cmd_graph_edge_list(&f, false).is_ok());
    }

    #[test]
    fn test_fj755_edge_list_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  base:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  app:\n    type: file\n    machine: web\n    path: /opt/app\n    depends_on: [base]\n");
        assert!(cmd_graph_edge_list(&f, false).is_ok());
    }

    #[test]
    fn test_fj755_edge_list_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  base:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  app:\n    type: file\n    machine: web\n    path: /opt/app\n    depends_on: [base]\n");
        assert!(cmd_graph_edge_list(&f, true).is_ok());
    }

    // ----- FJ-750: status --convergence-percentage -----

    #[test]
    fn test_fj750_convergence_percentage_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_convergence_percentage(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj750_convergence_percentage_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_convergence_percentage(dir.path(), None, true).is_ok());
    }

    // ----- FJ-754: status --failed-count -----

    #[test]
    fn test_fj754_failed_count_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_failed_count(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj754_failed_count_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_failed_count(dir.path(), None, true).is_ok());
    }

    // ----- FJ-756: status --drift-count -----

    #[test]
    fn test_fj756_drift_count_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_drift_count(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj756_drift_count_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_drift_count(dir.path(), None, true).is_ok());
    }
}
