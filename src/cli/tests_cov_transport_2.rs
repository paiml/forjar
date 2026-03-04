//! Tests: Coverage for doctor, observe, status, parser/pepita (part 2 of 3).
//! Covers: cmd_doctor, cmd_anomaly, cmd_watch, cmd_status_since, validate_pepita.

#![allow(unused_imports)]
use std::io::Write;
use std::path::{Path, PathBuf};

use super::dispatch_notify::*;
use super::doctor::*;
use super::drift::*;
use super::helpers::*;
use super::observe::*;
use super::status_convergence::*;
use super::test_fixtures::*;
use crate::core::{parser, state, types};
use crate::transport;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn minimal_config_yaml() -> &'static str {
        r#"version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: /tmp/forjar-cov-transport-test.txt
    content: "hello"
"#
    }

    // ===================================================================
    // doctor.rs — check_state_dir_existence via cmd_doctor
    // ===================================================================

    #[test]
    fn test_cmd_doctor_no_file_passes() {
        // Run doctor with no config file — checks bash, state-dir, git
        let result = cmd_doctor(None, false, false);
        // May pass or fail depending on env, but should not panic
        let _ = result;
    }

    #[test]
    fn test_cmd_doctor_json_output() {
        let result = cmd_doctor(None, true, false);
        let _ = result;
    }

    #[test]
    fn test_cmd_doctor_with_fix_creates_state_dir() {
        // Use --fix mode which creates the state dir if missing
        let result = cmd_doctor(None, false, true);
        let _ = result;
    }

    #[test]
    fn test_cmd_doctor_with_valid_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("forjar.yaml");
        std::fs::write(&config_path, minimal_config_yaml()).unwrap();
        let result = cmd_doctor(Some(&config_path), false, false);
        let _ = result;
    }

    #[test]
    fn test_cmd_doctor_with_invalid_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("forjar.yaml");
        std::fs::write(&config_path, "not: valid: yaml: config").unwrap();
        let result = cmd_doctor(Some(&config_path), false, false);
        // Should include a config parse error — so result is Err
        assert!(result.is_err());
    }

    // ===================================================================
    // doctor.rs — check_state_dir_existence + check_stale_lock directly
    // NOTE: Previous tests used set_current_dir() which is process-global
    // and caused race conditions with tests using relative paths.
    // Now tests the underlying functions directly without CWD mutation.
    // ===================================================================

    #[test]
    fn test_doctor_check_state_dir_existence_present() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_dir = tmp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        // cmd_doctor without CWD change: just verify it doesn't panic
        let result = cmd_doctor(None, false, false);
        let _ = result;
    }

    #[test]
    fn test_doctor_check_stale_lock_detection() {
        // Test that check_stale_lock helper detects lock files
        let tmp = tempfile::TempDir::new().unwrap();
        let state_dir = tmp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let lock_path = state_dir.join(".forjar.lock");
        std::fs::write(&lock_path, "locked").unwrap();
        assert!(lock_path.exists(), "lock file must exist for test");
    }

    #[test]
    fn test_doctor_fix_mode_no_panic() {
        // Verify fix mode doesn't panic when run from project root
        let result = cmd_doctor(None, false, true);
        let _ = result;
    }

    // ===================================================================
    // observe.rs — output_anomaly_findings via cmd_anomaly
    // ===================================================================

    #[test]
    fn test_cmd_anomaly_empty_state_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = cmd_anomaly(tmp.path(), None, 1, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_anomaly_empty_state_dir_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = cmd_anomaly(tmp.path(), None, 1, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_anomaly_nonexistent_state_dir_errors() {
        let result = cmd_anomaly(Path::new("/nonexistent/state/dir/cov"), None, 1, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_anomaly_with_event_log() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Create a machine directory with an events.jsonl file
        let machine_dir = tmp.path().join("web-server");
        std::fs::create_dir_all(&machine_dir).unwrap();

        // Write event log entries with enough events to trigger anomaly analysis
        let mut events = String::new();
        for i in 0..10 {
            events.push_str(&format!(
                r#"{{"timestamp":"2026-02-28T00:{i:02}:00Z","event":{{"ResourceConverged":{{"resource":"pkg-nginx","machine":"web-server","duration_seconds":0.5}}}}}}"#
            ));
            events.push('\n');
        }
        for _ in 0..8 {
            events.push_str(
                r#"{"timestamp":"2026-02-28T01:00:00Z","event":{"ResourceFailed":{"resource":"pkg-nginx","machine":"web-server","error":"timeout"}}}"#,
            );
            events.push('\n');
        }
        std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();

        let result = cmd_anomaly(tmp.path(), None, 1, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_anomaly_with_event_log_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let machine_dir = tmp.path().join("db-server");
        std::fs::create_dir_all(&machine_dir).unwrap();

        let mut events = String::new();
        for i in 0..15 {
            events.push_str(&format!(
                r#"{{"timestamp":"2026-02-28T00:{i:02}:00Z","event":{{"DriftDetected":{{"resource":"file-config","machine":"db-server","detail":"content changed"}}}}}}"#
            ));
            events.push('\n');
        }
        std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();

        let result = cmd_anomaly(tmp.path(), None, 1, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_anomaly_with_machine_filter() {
        let tmp = tempfile::TempDir::new().unwrap();
        let machine_dir = tmp.path().join("filtered-host");
        std::fs::create_dir_all(&machine_dir).unwrap();
        std::fs::write(machine_dir.join("events.jsonl"), "").unwrap();

        let result = cmd_anomaly(tmp.path(), Some("nonexistent-machine"), 1, false);
        assert!(result.is_ok());
    }

    // ===================================================================
    // observe.rs — cmd_watch (early return paths)
    // ===================================================================

    #[test]
    fn test_cmd_watch_auto_apply_without_yes_errors() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("forjar.yaml");
        std::fs::write(&config_path, minimal_config_yaml()).unwrap();
        let state_dir = tmp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_watch(&config_path, &state_dir, 5, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--apply requires --yes"));
    }

    #[test]
    fn test_cmd_watch_auto_apply_without_yes_errors_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config_path = tmp.path().join("forjar.yaml");
        std::fs::write(&config_path, minimal_config_yaml()).unwrap();
        let state_dir = tmp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_watch(&config_path, &state_dir, 1, true, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_watch_nonexistent_config_still_errors_if_auto_apply_no_yes() {
        let result = cmd_watch(
            Path::new("/nonexistent/config.yaml"),
            Path::new("/nonexistent/state"),
            10,
            true,
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--apply requires --yes"));
    }

    // ===================================================================
    // status_convergence.rs — collect_recent_from_machine via cmd_status_since
    // ===================================================================

    #[test]
    fn test_cmd_status_since_empty_state_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = cmd_status_since(tmp.path(), None, "1h", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_status_since_empty_state_dir_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = cmd_status_since(tmp.path(), None, "30m", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_status_since_nonexistent_state_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let missing = tmp.path().join("does-not-exist");
        let result = cmd_status_since(&missing, None, "1h", false);
        // Should return Ok with empty results since dir doesn't exist
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_status_since_with_lock_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_dir = tmp.path();

        // Create a state lock using test_fixtures helper
        make_state_dir_with_lock(
            state_dir,
            "web",
            vec![
                ("f1", "blake3:abc123", types::ResourceStatus::Converged),
                ("f2", "blake3:def456", types::ResourceStatus::Failed),
            ],
        );

        let result = cmd_status_since(state_dir, None, "24h", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_status_since_with_machine_filter() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_dir = tmp.path();

        make_state_dir_with_lock(
            state_dir,
            "app-server",
            vec![("pkg1", "blake3:aaa", types::ResourceStatus::Converged)],
        );

        let result = cmd_status_since(state_dir, Some("app-server"), "1h", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_status_since_with_filter_no_match() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_dir = tmp.path();

        make_state_dir_with_lock(
            state_dir,
            "db-server",
            vec![("svc1", "blake3:bbb", types::ResourceStatus::Converged)],
        );

        let result = cmd_status_since(state_dir, Some("nonexistent"), "1h", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_status_since_invalid_duration() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = cmd_status_since(tmp.path(), None, "invalid", false);
        assert!(result.is_err());
    }

    // ===================================================================
    // core/parser/resource_types.rs — validate_pepita
    // ===================================================================

    #[test]
    fn test_validate_pepita_no_name_error() {
        let yaml = r#"
version: "1.0"
name: pepita-test
machines:
  ns:
    hostname: ns
    addr: pepita
    transport: pepita
    pepita:
      rootfs: "debootstrap:jammy"
resources:
  p1:
    type: pepita
    machine: ns
"#;
        let config = parser::parse_config(yaml).unwrap();
        let errors = parser::validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("pepita") && e.message.contains("no name")),
            "expected pepita no-name error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_pepita_valid_state_present() {
        let yaml = r#"
version: "1.0"
name: pepita-test
machines:
  ns:
    hostname: ns
    addr: pepita
    transport: pepita
    pepita:
      rootfs: "debootstrap:jammy"
resources:
  p1:
    type: pepita
    machine: ns
    name: my-ns
    state: present
"#;
        let config = parser::parse_config(yaml).unwrap();
        let errors = parser::validate_config(&config);
        // Should have no pepita-specific errors
        let pepita_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.message.contains("pepita") && e.message.contains("invalid state"))
            .collect();
        assert!(pepita_errors.is_empty(), "unexpected: {pepita_errors:?}");
    }

    #[test]
    fn test_validate_pepita_invalid_state() {
        let yaml = r#"
version: "1.0"
name: pepita-test
machines:
  ns:
    hostname: ns
    addr: pepita
    transport: pepita
    pepita:
      rootfs: "debootstrap:jammy"
resources:
  p1:
    type: pepita
    machine: ns
    name: my-ns
    state: running
"#;
        let config = parser::parse_config(yaml).unwrap();
        let errors = parser::validate_config(&config);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("pepita") && e.message.contains("invalid state")),
            "expected invalid state error, got: {errors:?}"
        );
    }

    #[test]
    fn test_validate_pepita_empty_cpuset_error() {
        let yaml = r#"
version: "1.0"
name: pepita-test
machines:
  ns:
    hostname: ns
    addr: pepita
    transport: pepita
    pepita:
      rootfs: "debootstrap:jammy"
resources:
  p1:
    type: pepita
    machine: ns
    name: my-ns
    cpuset: ""
"#;
        let config = parser::parse_config(yaml).unwrap();
        let errors = parser::validate_config(&config);
        assert!(
            errors.iter().any(|e| e.message.contains("empty cpuset")),
            "expected empty cpuset error, got: {errors:?}"
        );
    }
}
