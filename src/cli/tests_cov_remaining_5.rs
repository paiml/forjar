//! Tests: Coverage for remaining validate, lock, destroy, observe (part 5).

use super::destroy::*;
use super::lock_core::*;
use super::lock_ops::*;
use super::lock_security::*;
use super::observe::*;
use super::validate_compliance::*;
use super::validate_resources::*;
use super::validate_structural::*;
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

    fn empty_config() -> &'static str {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n"
    }

    fn basic_config() -> &'static str {
        "version: \"1.0\"\nname: test-project\nmachines:\n  web:\n    hostname: web\n    addr: 127.0.0.1\nresources:\n  app-config:\n    type: file\n    machine: web\n    path: /etc/app.conf\n    content: \"port=8080\"\n    owner: root\n    group: root\n    mode: \"0644\"\n  web-svc:\n    type: service\n    machine: web\n    name: nginx\n    depends_on: [app-config]\n"
    }

    fn state_lock_yaml() -> &'static str {
        "schema: \"1\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789\"\n  g:\n    type: service\n    status: drifted\n    hash: \"1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef\"\n"
    }

    fn make_state_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "web.lock.yaml", state_lock_yaml());
        dir
    }

    fn make_state_dir_with_events() -> tempfile::TempDir {
        let dir = make_state_dir();
        let events = "{\"timestamp\":\"2026-01-01T00:00:00Z\",\"resource\":\"f\",\"action\":\"apply\"}\n{\"timestamp\":\"2026-02-01T00:00:00Z\",\"resource\":\"g\",\"action\":\"converge\"}\n";
        write_yaml(dir.path(), "web/events.jsonl", events);
        write_yaml(dir.path(), "web.events.jsonl", events);
        dir
    }

    // ========================================================================
    // 43. destroy: cmd_destroy
    // ========================================================================

    #[test]
    fn test_cov_destroy_no_yes_flag() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", basic_config());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_destroy(&cfg, &state, None, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_destroy_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", "NOT VALID YAML");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_destroy(&cfg, &state, None, true, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_destroy_verbose_flag() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("destroy-test.txt");
        let yaml = format!(
            "version: \"1.0\"\nname: destroy-test\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: local\n    path: {}\n    content: \"test\"\n",
            target.display()
        );
        let cfg = write_yaml(dir.path(), "forjar.yaml", &yaml);
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        // First apply to create the file
        let _ = super::super::apply::cmd_apply(
            &cfg,
            &state,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true,
            false,
            None,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            None, // telemetry_endpoint
        );
        let result = cmd_destroy(&cfg, &state, None, true, true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 44. destroy: compute_rollback_changes
    // ========================================================================

    #[test]
    fn test_cov_compute_rollback_changes_empty() {
        let prev: crate::core::types::ForjarConfig =
            serde_yaml_ng::from_str(empty_config()).unwrap();
        let curr: crate::core::types::ForjarConfig =
            serde_yaml_ng::from_str(empty_config()).unwrap();
        let changes = compute_rollback_changes(&prev, &curr, 1);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_cov_compute_rollback_changes_added_resource() {
        let prev: crate::core::types::ForjarConfig =
            serde_yaml_ng::from_str(empty_config()).unwrap();
        let curr: crate::core::types::ForjarConfig =
            serde_yaml_ng::from_str(basic_config()).unwrap();
        let changes = compute_rollback_changes(&prev, &curr, 1);
        // curr has resources that prev doesn't => "exists now but not in HEAD~1"
        assert!(!changes.is_empty());
    }

    #[test]
    fn test_cov_compute_rollback_changes_removed_resource() {
        let prev: crate::core::types::ForjarConfig =
            serde_yaml_ng::from_str(basic_config()).unwrap();
        let curr: crate::core::types::ForjarConfig =
            serde_yaml_ng::from_str(empty_config()).unwrap();
        let changes = compute_rollback_changes(&prev, &curr, 2);
        // prev has resources not in curr => "will be re-added from HEAD~2"
        assert!(!changes.is_empty());
    }

    // ========================================================================
    // 45. observe: cmd_anomaly
    // ========================================================================

    #[test]
    fn test_cov_anomaly_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_anomaly(dir.path(), None, 1, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_anomaly_with_failure_events_plain() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let events: Vec<String> = (0..10)
            .map(|i| {
                format!(
                    r#"{{"ts":"2026-02-{:02}T00:00:00Z","event":"resource_failed","machine":"web","resource":"pkg","error":"timeout"}}"#,
                    (i % 28) + 1
                )
            })
            .collect();
        std::fs::write(machine_dir.join("events.jsonl"), events.join("\n")).unwrap();
        let result = cmd_anomaly(dir.path(), None, 1, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_anomaly_with_failure_events_json() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let events: Vec<String> = (0..10)
            .map(|i| {
                format!(
                    r#"{{"ts":"2026-02-{:02}T00:00:00Z","event":"resource_failed","machine":"web","resource":"pkg","error":"timeout"}}"#,
                    (i % 28) + 1
                )
            })
            .collect();
        std::fs::write(machine_dir.join("events.jsonl"), events.join("\n")).unwrap();
        let result = cmd_anomaly(dir.path(), None, 1, true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 46. observe: cmd_trace
    // ========================================================================

    #[test]
    fn test_cov_trace_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_trace(dir.path(), None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_trace_empty_dir_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_trace(dir.path(), None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_trace_with_data_text() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = crate::tripwire::tracer::TraceSession::start("cov-trace");
        session.record_noop("r1", "file", "m1");
        session.record_span(
            "r2",
            "package",
            "m1",
            "create",
            std::time::Duration::from_millis(500),
            0,
            None,
        );
        session.record_span(
            "r3",
            "service",
            "m1",
            "update",
            std::time::Duration::from_secs(2),
            1,
            None,
        );
        crate::tripwire::tracer::write_trace(dir.path(), "m1", &session).unwrap();
        let result = cmd_trace(dir.path(), None, false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 47. observe: handle_watch_change
    // ========================================================================

    #[test]
    fn test_cov_handle_watch_change_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", basic_config());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        handle_watch_change(&cfg, &state, false);
    }

    #[test]
    fn test_cov_handle_watch_change_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", "NOT VALID");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        // Should print error but not panic
        handle_watch_change(&cfg, &state, false);
    }

    #[test]
    fn test_cov_handle_watch_change_auto_apply() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("watch-target.txt");
        let yaml = format!(
            "version: \"1.0\"\nname: watch\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: local\n    path: {}\n    content: \"watch-content\"\n",
            target.display()
        );
        let cfg = write_yaml(dir.path(), "forjar.yaml", &yaml);
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        // auto_apply=true should trigger apply
        handle_watch_change(&cfg, &state, true);
    }

    // ========================================================================
    // 48. validate_resources: print_resource_limits_text
    // ========================================================================

    #[test]
    fn test_cov_print_resource_limits_text_empty() {
        let counts = std::collections::HashMap::new();
        let violations: Vec<(String, usize)> = vec![];
        print_resource_limits_text(&counts, &violations, 100);
    }

    #[test]
    fn test_cov_print_resource_limits_text_with_data() {
        let mut counts = std::collections::HashMap::new();
        counts.insert("web".to_string(), 5);
        counts.insert("db".to_string(), 3);
        let violations: Vec<(String, usize)> = vec![];
        print_resource_limits_text(&counts, &violations, 100);
    }

    #[test]
    fn test_cov_print_resource_limits_text_with_violations() {
        let mut counts = std::collections::HashMap::new();
        counts.insert("web".to_string(), 150);
        let violations = vec![("web".to_string(), 150_usize)];
        print_resource_limits_text(&counts, &violations, 100);
    }

    // ========================================================================
    // 49. validate_structural: secrets with commented lines
    // ========================================================================

    #[test]
    fn test_cov_check_secrets_commented_lines() {
        let yaml = "# This is a comment with password: in it\nversion: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_secrets(f.path(), false);
        // Comment lines with # should be skipped
        assert!(result.is_ok());
    }

    // ========================================================================
    // 50. validate_compliance: portability with service type
    // ========================================================================

    #[test]
    fn test_cov_check_portability_service_type() {
        let f = write_temp_config(basic_config()); // has a service type
        let result = cmd_validate_check_portability(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_portability_service_type_json() {
        let f = write_temp_config(basic_config());
        let result = cmd_validate_check_portability(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 51. lock_ops: cmd_lock_export csv
    // ========================================================================

    #[test]
    fn test_cov_lock_export_csv_no_filter() {
        let dir = make_state_dir();
        let result = cmd_lock_export(dir.path(), "csv", None);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 52. lock_security: cmd_lock_audit_trail with machine filter
    // ========================================================================

    #[test]
    fn test_cov_lock_audit_trail_nonexistent_machine() {
        let dir = make_state_dir_with_events();
        let result = cmd_lock_audit_trail(dir.path(), Some("nonexistent"), false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 53. lock_core: cmd_lock with workspace
    // ========================================================================

    #[test]
    fn test_cov_lock_with_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", basic_config());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_lock(&cfg, &state_dir, None, Some("staging"), false, false, false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 54. observe: cmd_anomaly with drift events
    // ========================================================================

    #[test]
    fn test_cov_anomaly_with_drift_events() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let events: Vec<String> = (0..5)
            .map(|i| {
                format!(
                    r#"{{"ts":"2026-02-{:02}T00:00:00Z","event":"drift_detected","machine":"web","resource":"cfg","expected_hash":"abc","actual_hash":"def"}}"#,
                    (i % 28) + 1
                )
            })
            .collect();
        std::fs::write(machine_dir.join("events.jsonl"), events.join("\n")).unwrap();
        let result = cmd_anomaly(dir.path(), None, 1, false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 55. observe: cmd_trace with machine filter matching
    // ========================================================================

    #[test]
    fn test_cov_trace_machine_filter_match() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = crate::tripwire::tracer::TraceSession::start("cov-filter");
        session.record_noop("r1", "file", "web");
        crate::tripwire::tracer::write_trace(dir.path(), "web", &session).unwrap();
        let result = cmd_trace(dir.path(), Some("web"), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_trace_machine_filter_match_json() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = crate::tripwire::tracer::TraceSession::start("cov-filter-j");
        session.record_noop("r1", "file", "web");
        crate::tripwire::tracer::write_trace(dir.path(), "web", &session).unwrap();
        let result = cmd_trace(dir.path(), Some("web"), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 56. lock_security: cmd_lock_stats with no lock files
    // ========================================================================

    #[test]
    fn test_cov_lock_stats_no_lock_files() {
        let dir = tempfile::tempdir().unwrap();
        // Create a machine dir but no lock file
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let result = cmd_lock_stats(dir.path(), false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 57. validate_compliance: cmd_validate_check_drift_risk with many dependents
    // ========================================================================

    #[test]
    fn test_cov_check_drift_risk_many_dependents() {
        let yaml = "version: \"1.0\"\nname: deps-test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  base:\n    type: file\n    machine: m\n    path: /tmp/base\n    content: base\n  d1:\n    type: file\n    machine: m\n    path: /tmp/d1\n    content: d1\n    depends_on: [base]\n  d2:\n    type: file\n    machine: m\n    path: /tmp/d2\n    content: d2\n    depends_on: [base]\n  d3:\n    type: file\n    machine: m\n    path: /tmp/d3\n    content: d3\n    depends_on: [base]\n";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_drift_risk(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_drift_risk_many_dependents_json() {
        let yaml = "version: \"1.0\"\nname: deps-test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  base:\n    type: file\n    machine: m\n    path: /tmp/base\n    content: base\n  d1:\n    type: file\n    machine: m\n    path: /tmp/d1\n    content: d1\n    depends_on: [base]\n  d2:\n    type: file\n    machine: m\n    path: /tmp/d2\n    content: d2\n    depends_on: [base]\n  d3:\n    type: file\n    machine: m\n    path: /tmp/d3\n    content: d3\n    depends_on: [base]\n";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_drift_risk(f.path(), true);
        assert!(result.is_ok());
    }
}
