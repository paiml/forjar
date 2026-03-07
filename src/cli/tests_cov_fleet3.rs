//! Coverage tests for fleet_reporting.rs, check_test.rs, destroy.rs, apply_output.rs.

#![allow(unused_imports)]
use super::fleet_reporting::*;
use super::check_test::*;
use super::check_test_runners::RunnerOpts;
use super::destroy::*;
use super::apply_output::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
    }

    const CFG: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n    state: present\n    depends_on:\n      - pkg\n";

    fn setup_state() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "m1/state.lock.yaml", "resources:\n  pkg:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 1.0\n");
        write_yaml(dir.path(), "m1/events.jsonl", "{\"ts\":\"2026-01-01T00:00:00Z\",\"event\":\"resource_started\",\"resource\":\"pkg\",\"machine\":\"m1\"}\n");
        dir
    }

    // fleet_reporting.rs
    #[test]
    fn test_audit() {
        let d = setup_state();
        let _ = cmd_audit(d.path(), None, 10, false);
    }
    #[test]
    fn test_audit_json() {
        let d = setup_state();
        let _ = cmd_audit(d.path(), None, 10, true);
    }
    #[test]
    fn test_audit_machine() {
        let d = setup_state();
        let _ = cmd_audit(d.path(), Some("m1"), 5, false);
    }
    #[test]
    fn test_compliance() {
        let f = write_cfg(CFG);
        let _ = cmd_compliance(f.path(), false);
    }
    #[test]
    fn test_compliance_json() {
        let f = write_cfg(CFG);
        let _ = cmd_compliance(f.path(), true);
    }
    #[test]
    fn test_export_yaml() {
        let d = setup_state();
        let _ = cmd_export(d.path(), "yaml", None, None);
    }
    #[test]
    fn test_export_json() {
        let d = setup_state();
        let _ = cmd_export(d.path(), "json", None, None);
    }
    #[test]
    fn test_export_machine() {
        let d = setup_state();
        let _ = cmd_export(d.path(), "yaml", Some("m1"), None);
    }
    #[test]
    fn test_export_output() {
        let d = setup_state();
        let out = tempfile::tempdir().unwrap();
        let outf = out.path().join("export.yaml");
        let _ = cmd_export(d.path(), "yaml", None, Some(outf.as_path()));
    }
    #[test]
    fn test_suggest() {
        let f = write_cfg(CFG);
        let _ = cmd_suggest(f.path(), false);
    }
    #[test]
    fn test_suggest_json() {
        let f = write_cfg(CFG);
        let _ = cmd_suggest(f.path(), true);
    }

    // check_test.rs
    #[test]
    fn test_cmd_test() {
        let f = write_cfg(CFG);
        let _ = cmd_test(f.path(), None, None, None, None, false, false, &RunnerOpts::default());
    }
    #[test]
    fn test_cmd_test_json() {
        let f = write_cfg(CFG);
        let _ = cmd_test(f.path(), None, None, None, None, true, false, &RunnerOpts::default());
    }
    #[test]
    fn test_cmd_test_machine() {
        let f = write_cfg(CFG);
        let _ = cmd_test(f.path(), Some("m1"), None, None, None, false, false, &RunnerOpts::default());
    }
    #[test]
    fn test_cmd_test_resource() {
        let f = write_cfg(CFG);
        let _ = cmd_test(f.path(), None, Some("pkg"), None, None, false, false, &RunnerOpts::default());
    }
    #[test]
    fn test_cmd_test_verbose() {
        let f = write_cfg(CFG);
        let _ = cmd_test(f.path(), None, None, None, None, false, true, &RunnerOpts::default());
    }

    // destroy.rs
    #[test]
    fn test_destroy() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_destroy(f.path(), d.path(), None, false, false);
    }
    #[test]
    fn test_destroy_machine() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_destroy(f.path(), d.path(), Some("m1"), false, false);
    }
    #[test]
    fn test_destroy_verbose() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_destroy(f.path(), d.path(), None, false, true);
    }
    #[test]
    fn test_rollback() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_rollback(f.path(), d.path(), 1, None, true, false);
    }
    #[test]
    fn test_rollback_machine() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_rollback(f.path(), d.path(), 1, Some("m1"), true, false);
    }
    #[test]
    fn test_rollback_verbose() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_rollback(f.path(), d.path(), 1, None, true, true);
    }

    // apply_output.rs
    #[test]
    fn test_count_results_empty() {
        let (c, u, f) = count_results(&[]);
        assert_eq!((c, u, f), (0, 0, 0));
    }
    #[test]
    fn test_print_events_empty() {
        let _ = print_events_output(&[]);
    }
    #[test]
    fn test_print_resource_report_empty() {
        print_resource_report(&[]);
    }
    #[test]
    fn test_print_timing() {
        use std::time::Duration;
        print_timing(Duration::from_millis(10), Duration::from_millis(50), Duration::from_millis(60));
    }
}
