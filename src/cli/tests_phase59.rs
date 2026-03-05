//! Phase 59 tests: FJ-730 through FJ-737.

#[cfg(test)]
mod tests {
    use super::super::graph_topology::cmd_graph_breadth_first;
    use super::super::status_resource_detail::{
        cmd_status_machine_health_summary, cmd_status_resource_health,
    };
    use super::super::validate_paths::cmd_validate_check_cron_syntax;

    fn yaml_header() -> &'static str {
        "version: \"1.0\"\nname: test\n"
    }

    fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        let f = dir.join("forjar.yaml");
        let content = format!("{}{}", yaml_header(), body);
        std::fs::write(&f, content).unwrap();
        f
    }

    // ----- FJ-731: validate --check-cron-syntax -----

    #[test]
    fn test_fj731_cron_valid_schedules() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  r1:\n    type: file\n    path: /tmp/r1\n    content: hello\n    machine: m1\n    schedule: \"0 * * * *\"\n  r2:\n    type: file\n    path: /tmp/r2\n    content: hello\n    machine: m1\n    schedule: \"30 2 1,15 * 0-6\"\n");
        let result = cmd_validate_check_cron_syntax(&f, false);
        assert!(result.is_ok(), "Expected Ok, got: {result:?}");
    }

    #[test]
    fn test_fj731_cron_invalid_schedule() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  r1:\n    type: file\n    path: /tmp/r1\n    content: hello\n    machine: m1\n    schedule: \"99 * * * *\"\n");
        let result = cmd_validate_check_cron_syntax(&f, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj731_cron_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  r1:\n    type: file\n    path: /tmp/r1\n    content: hello\n    machine: m1\n    schedule: \"0 * * * *\"\n");
        let result = cmd_validate_check_cron_syntax(&f, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj731_cron_no_schedules() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  r1:\n    type: file\n    path: /tmp/r1\n    content: hello\n    machine: m1\n");
        let result = cmd_validate_check_cron_syntax(&f, false);
        assert!(result.is_ok());
    }

    // ----- FJ-732: status --resource-health -----

    #[test]
    fn test_fj732_resource_health_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_resource_health(dir.path(), None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj732_resource_health_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_resource_health(dir.path(), None, true);
        assert!(result.is_ok());
    }

    // ----- FJ-734: graph --breadth-first -----

    #[test]
    fn test_fj734_breadth_first_simple() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    path: /tmp/a\n    content: a\n    machine: m1\n  b:\n    type: file\n    path: /tmp/b\n    content: b\n    machine: m1\n    depends_on: [a]\n  c:\n    type: file\n    path: /tmp/c\n    content: c\n    machine: m1\n    depends_on: [a]\n");
        let result = cmd_graph_breadth_first(&f, false);
        assert!(result.is_ok(), "Expected Ok, got: {result:?}");
    }

    #[test]
    fn test_fj734_breadth_first_json() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    path: /tmp/a\n    content: a\n    machine: m1\n");
        let result = cmd_graph_breadth_first(&f, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj734_breadth_first_chain() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_config(dir.path(), "machines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    path: /tmp/a\n    content: a\n    machine: m1\n  b:\n    type: file\n    path: /tmp/b\n    content: b\n    machine: m1\n    depends_on: [a]\n  c:\n    type: file\n    path: /tmp/c\n    content: c\n    machine: m1\n    depends_on: [b]\n");
        let result = cmd_graph_breadth_first(&f, false);
        assert!(result.is_ok());
    }

    // ----- FJ-737: status --machine-health-summary -----

    #[test]
    fn test_fj737_machine_health_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_machine_health_summary(dir.path(), None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj737_machine_health_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_machine_health_summary(dir.path(), None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj737_machine_health_filter() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_machine_health_summary(dir.path(), Some("web1"), false);
        assert!(result.is_ok());
    }
}
