//! Tests: Coverage for fleet_ops and fleet_reporting (part 2 — audit, compliance, suggest, edge cases).

use super::fleet_ops::*;
use super::fleet_reporting::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn write_yaml(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
        p
    }

    fn state_lock_yaml() -> &'static str {
        "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n"
    }

    fn state_lock_yaml_db() -> &'static str {
        "schema: \"1.0\"\nmachine: db\nhostname: db\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  g:\n    type: service\n    status: converged\n    hash: \"blake3:def\"\n"
    }

    // ========================================================================
    // cmd_audit tests
    // ========================================================================

    #[test]
    fn test_cov_audit_empty_state_dir_text() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_audit(dir.path(), None, 20, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_audit_empty_state_dir_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_audit(dir.path(), None, 20, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_audit_nonexistent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("no_such_dir");
        let result = cmd_audit(&bad, None, 20, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_audit_with_events_text() {
        let dir = tempfile::tempdir().unwrap();
        let events = concat!(
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"web","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:01:00Z","event":"apply_completed","machine":"web","run_id":"r1","resources_converged":1,"resources_unchanged":0,"resources_failed":0,"total_seconds":60.0}"#,
            "\n",
        );
        write_yaml(dir.path(), "web/events.jsonl", events);
        let result = cmd_audit(dir.path(), None, 20, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_audit_with_events_json() {
        let dir = tempfile::tempdir().unwrap();
        let events = concat!(
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"web","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:01:00Z","event":"apply_completed","machine":"web","run_id":"r1","resources_converged":1,"resources_unchanged":0,"resources_failed":0,"total_seconds":60.0}"#,
            "\n",
        );
        write_yaml(dir.path(), "web/events.jsonl", events);
        let result = cmd_audit(dir.path(), None, 20, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_audit_machine_filter_match() {
        let dir = tempfile::tempdir().unwrap();
        let events = concat!(
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"web","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
        );
        write_yaml(dir.path(), "web/events.jsonl", events);
        write_yaml(dir.path(), "db/events.jsonl", events);
        let result = cmd_audit(dir.path(), Some("web"), 20, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_audit_machine_filter_no_match() {
        let dir = tempfile::tempdir().unwrap();
        let events = concat!(
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"web","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
        );
        write_yaml(dir.path(), "web/events.jsonl", events);
        let result = cmd_audit(dir.path(), Some("nonexistent"), 20, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_audit_limit_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let mut events_str = String::new();
        for i in 0..10 {
            events_str.push_str(&format!(
                r#"{{"ts":"2026-01-{:02}T00:00:00Z","event":"apply_started","machine":"web","run_id":"r{}","forjar_version":"0.1"}}"#,
                i + 1, i
            ));
            events_str.push('\n');
        }
        write_yaml(dir.path(), "web/events.jsonl", &events_str);
        let result = cmd_audit(dir.path(), None, 3, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_audit_limit_truncation_json() {
        let dir = tempfile::tempdir().unwrap();
        let mut events_str = String::new();
        for i in 0..10 {
            events_str.push_str(&format!(
                r#"{{"ts":"2026-01-{:02}T00:00:00Z","event":"apply_started","machine":"web","run_id":"r{}","forjar_version":"0.1"}}"#,
                i + 1, i
            ));
            events_str.push('\n');
        }
        write_yaml(dir.path(), "web/events.jsonl", &events_str);
        let result = cmd_audit(dir.path(), None, 3, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_audit_empty_lines_in_events() {
        let dir = tempfile::tempdir().unwrap();
        let events = concat!(
            "\n",
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"web","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
            "\n",
            "  \n",
        );
        write_yaml(dir.path(), "web/events.jsonl", events);
        let result = cmd_audit(dir.path(), None, 20, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_audit_malformed_events_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let events = concat!(
            "not json at all\n",
            "{broken}\n",
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"web","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
        );
        write_yaml(dir.path(), "web/events.jsonl", events);
        let result = cmd_audit(dir.path(), None, 20, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_audit_file_not_dir_skipped() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "somefile.txt", "not a dir");
        let result = cmd_audit(dir.path(), None, 20, false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // compliance, suggest, and edge cases
    // ========================================================================

    #[test]
    fn test_cov_compliance_file_no_mode_text() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n";
        let f = write_temp_config(yaml);
        assert!(cmd_compliance(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_compliance_invalid_config() {
        let f = write_temp_config("totally invalid: [[[");
        assert!(cmd_compliance(f.path(), false).is_err());
    }

    #[test]
    fn test_cov_suggest_no_dependencies_text() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n";
        let f = write_temp_config(yaml);
        assert!(cmd_suggest(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_suggest_invalid_config() {
        let f = write_temp_config("totally invalid: [[[");
        assert!(cmd_suggest(f.path(), false).is_err());
    }

    #[test]
    fn test_cov_export_nonexistent_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("no_such_dir");
        assert!(cmd_export(&bad, "csv", None, None).is_err());
    }

    #[test]
    fn test_cov_audit_multi_machine_events() {
        let dir = tempfile::tempdir().unwrap();
        let events_web = concat!(
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"web","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
        );
        let events_db = concat!(
            r#"{"ts":"2026-01-02T00:00:00Z","event":"apply_started","machine":"db","run_id":"r2","forjar_version":"0.1"}"#,
            "\n",
        );
        write_yaml(dir.path(), "web/events.jsonl", events_web);
        write_yaml(dir.path(), "db/events.jsonl", events_db);
        assert!(cmd_audit(dir.path(), None, 50, true).is_ok());
    }

    #[test]
    fn test_cov_export_ansible_multi_machine() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "db/state.lock.yaml", state_lock_yaml_db());
        assert!(cmd_export(dir.path(), "ansible", None, None).is_ok());
    }

    #[test]
    fn test_cov_export_terraform_multi_machine() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "db/state.lock.yaml", state_lock_yaml_db());
        assert!(cmd_export(dir.path(), "terraform", None, None).is_ok());
    }

    #[test]
    fn test_cov_audit_dir_without_events() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("web")).unwrap();
        assert!(cmd_audit(dir.path(), None, 20, false).is_ok());
    }

    #[test]
    fn test_cov_audit_resource_events() {
        let dir = tempfile::tempdir().unwrap();
        let events = concat!(
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"web","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:00:01Z","event":"resource_started","machine":"web","resource":"a","action":"apply"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:00:02Z","event":"resource_converged","machine":"web","resource":"a","duration_seconds":1.0,"hash":"abc"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:00:03Z","event":"resource_failed","machine":"web","resource":"b","error":"timeout"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:00:05Z","event":"apply_completed","machine":"web","run_id":"r1","resources_converged":1,"resources_unchanged":0,"resources_failed":1,"total_seconds":5.0}"#,
            "\n",
        );
        write_yaml(dir.path(), "web/events.jsonl", events);
        assert!(cmd_audit(dir.path(), None, 20, false).is_ok());
    }

    #[test]
    fn test_cov_audit_resource_events_json() {
        let dir = tempfile::tempdir().unwrap();
        let events = concat!(
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"web","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:00:01Z","event":"resource_failed","machine":"web","resource":"b","error":"timeout"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:00:05Z","event":"apply_completed","machine":"web","run_id":"r1","resources_converged":0,"resources_unchanged":0,"resources_failed":1,"total_seconds":5.0}"#,
            "\n",
        );
        write_yaml(dir.path(), "web/events.jsonl", events);
        assert!(cmd_audit(dir.path(), None, 20, true).is_ok());
    }

    #[test]
    fn test_cov_suggest_multi_machine_resources_no_deps() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n  d:\n    type: file\n    machine: m\n    path: /tmp/d\n    content: d\n";
        let f = write_temp_config(yaml);
        assert!(cmd_suggest(f.path(), true).is_ok());
    }

    #[test]
    fn test_cov_inventory_multi_resource_types() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: package\n    machine: m\n    provider: apt\n    packages:\n      - curl\n  c:\n    type: service\n    machine: m\n    name: nginx\n    enabled: true\n";
        let f = write_temp_config(yaml);
        assert!(cmd_inventory(f.path(), true).is_ok());
    }

    #[test]
    fn test_cov_inventory_multi_resource_types_text() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: package\n    machine: m\n    provider: apt\n    packages:\n      - curl\n  c:\n    type: service\n    machine: m\n    name: nginx\n    enabled: true\n";
        let f = write_temp_config(yaml);
        assert!(cmd_inventory(f.path(), false).is_ok());
    }
}
