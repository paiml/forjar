//! Phase 66 tests: FJ-789 through FJ-796.

#[cfg(test)]
mod tests {
    use super::super::graph_advanced::cmd_graph_bipartite_check;
    use super::super::graph_export::cmd_graph_sink_resources;
    use super::super::status_fleet_detail::{
        cmd_status_convergence_score, cmd_status_last_drift_time, cmd_status_machine_resource_count,
    };
    use super::super::validate_safety::{
        cmd_validate_check_duplicate_names, cmd_validate_check_resource_groups,
    };

    fn yaml_header() -> &'static str {
        "version: \"1.0\"\nname: test\n"
    }

    fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let f = dir.join("forjar.yaml");
        std::fs::write(&f, format!("{}{}", yaml_header(), body)).unwrap();
        f
    }

    // ----- FJ-789: validate --check-duplicate-names -----

    #[test]
    fn test_fj789_no_dupes() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines: {}\nresources:\n  web/pkg1:\n    type: package\n    provider: apt\n    packages: [curl]\n\
  db/pkg2:\n    type: package\n    provider: apt\n    packages: [vim]\n",
        );
        assert!(cmd_validate_check_duplicate_names(&f, false).is_ok());
    }

    #[test]
    fn test_fj789_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_duplicate_names(&f, false).is_ok());
    }

    #[test]
    fn test_fj789_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_duplicate_names(&f, true).is_ok());
    }

    // ----- FJ-793: validate --check-resource-groups -----

    #[test]
    fn test_fj793_with_groups() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines: {}\nresources:\n  web/pkg1:\n    type: package\n    provider: apt\n    packages: [curl]\n\
  web/cfg1:\n    type: file\n    path: /opt/cfg\n",
        );
        assert!(cmd_validate_check_resource_groups(&f, false).is_ok());
    }

    #[test]
    fn test_fj793_no_groups() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines: {}\nresources:\n  pkg1:\n    type: package\n    provider: apt\n    packages: [curl]\n",
        );
        assert!(cmd_validate_check_resource_groups(&f, false).is_ok());
    }

    #[test]
    fn test_fj793_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_validate_check_resource_groups(&f, true).is_ok());
    }

    // ----- FJ-791: graph --sink-resources -----

    #[test]
    fn test_fj791_all_sinks() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: package\n    machine: web\n    provider: apt\n    packages: [vim]\n",
        );
        assert!(cmd_graph_sink_resources(&f, false).is_ok());
    }

    #[test]
    fn test_fj791_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_graph_sink_resources(&f, false).is_ok());
    }

    #[test]
    fn test_fj791_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_sink_resources(&f, true).is_ok());
    }

    // ----- FJ-795: graph --bipartite-check -----

    #[test]
    fn test_fj795_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_bipartite_check(&f, false).is_ok());
    }

    #[test]
    fn test_fj795_linear() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  a:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  b:\n    type: file\n    machine: web\n    path: /opt/b\n    depends_on: [a]\n",
        );
        assert!(cmd_graph_bipartite_check(&f, false).is_ok());
    }

    #[test]
    fn test_fj795_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_graph_bipartite_check(&f, true).is_ok());
    }

    // ----- FJ-790: status --last-drift-time -----

    #[test]
    fn test_fj790_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_last_drift_time(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj790_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_last_drift_time(dir.path(), None, true).is_ok());
    }

    // ----- FJ-794: status --machine-resource-count -----

    #[test]
    fn test_fj794_with_resources() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(
            dir.path(),
            "\
machines:\n  web:\n    hostname: web\n    addr: 1.2.3.4\n\
resources:\n  pkg1:\n    type: package\n    machine: web\n    provider: apt\n    packages: [curl]\n\
  cfg1:\n    type: file\n    machine: web\n    path: /etc/app.conf\n",
        );
        assert!(cmd_status_machine_resource_count(&f, false).is_ok());
    }

    #[test]
    fn test_fj794_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_status_machine_resource_count(&f, true).is_ok());
    }

    #[test]
    fn test_fj794_empty() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines: {}\nresources: {}\n");
        assert!(cmd_status_machine_resource_count(&f, false).is_ok());
    }

    // ----- FJ-796: status --convergence-score -----

    #[test]
    fn test_fj796_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_convergence_score(dir.path(), false).is_ok());
    }

    #[test]
    fn test_fj796_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_convergence_score(dir.path(), true).is_ok());
    }
}
