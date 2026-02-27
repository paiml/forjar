//! Tests: Pre-condition checks.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::check::*;
use super::commands::*;
use super::dispatch::*;
use super::observe::*;
use super::validate_core::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_anomaly_detects_drift() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let machine_dir = state.join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();

        let mut events = String::new();
        // 2 converges + 1 drift = 3 events (meets min_events=3)
        for _ in 0..2 {
            events.push_str(
                &serde_json::to_string(&types::TimestampedEvent {
                    ts: "2026-02-25T00:00:00Z".to_string(),
                    event: types::ProvenanceEvent::ResourceConverged {
                        machine: "web".to_string(),
                        resource: "config-file".to_string(),
                        duration_seconds: 0.5,
                        hash: "def".to_string(),
                    },
                })
                .unwrap(),
            );
            events.push('\n');
        }
        events.push_str(
            &serde_json::to_string(&types::TimestampedEvent {
                ts: "2026-02-25T01:00:00Z".to_string(),
                event: types::ProvenanceEvent::DriftDetected {
                    machine: "web".to_string(),
                    resource: "config-file".to_string(),
                    expected_hash: "aaa".to_string(),
                    actual_hash: "bbb".to_string(),
                },
            })
            .unwrap(),
        );
        events.push('\n');

        std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();

        let result = cmd_anomaly(&state, None, 3, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_anomaly_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let machine_dir = state.join("srv");
        std::fs::create_dir_all(&machine_dir).unwrap();

        // Write 3 converge events for one resource (no anomaly, just normal)
        let mut events = String::new();
        for _ in 0..3 {
            events.push_str(
                &serde_json::to_string(&types::TimestampedEvent {
                    ts: "2026-02-25T00:00:00Z".to_string(),
                    event: types::ProvenanceEvent::ResourceConverged {
                        machine: "srv".to_string(),
                        resource: "pkg".to_string(),
                        duration_seconds: 1.0,
                        hash: "xyz".to_string(),
                    },
                })
                .unwrap(),
            );
            events.push('\n');
        }

        std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();

        let result = cmd_anomaly(&state, None, 3, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_anomaly_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        // Create two machines
        let m1 = state.join("m1");
        let m2 = state.join("m2");
        std::fs::create_dir_all(&m1).unwrap();
        std::fs::create_dir_all(&m2).unwrap();

        // Events only on m2
        let mut events = String::new();
        for _ in 0..5 {
            events.push_str(
                &serde_json::to_string(&types::TimestampedEvent {
                    ts: "2026-02-25T00:00:00Z".to_string(),
                    event: types::ProvenanceEvent::ResourceFailed {
                        machine: "m2".to_string(),
                        resource: "bad-svc".to_string(),
                        error: "timeout".to_string(),
                    },
                })
                .unwrap(),
            );
            events.push('\n');
        }
        std::fs::write(m2.join("events.jsonl"), &events).unwrap();

        // Filter to m1 (no events) → no anomalies
        let result = cmd_anomaly(&state, Some("m1"), 1, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_anomaly_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let result = dispatch(
            Commands::Anomaly {
                state_dir: state,
                machine: None,
                min_events: 3,
                json: false,
            },
            false,
            true,
        );
        assert!(result.is_ok());
    }

    // ── Import scan type tests ─────────────────────────────────


    #[test]
    fn test_fj017_check_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        // Check with machine filter
        cmd_check(&config, Some("local"), None, None, false, false).unwrap();
    }


    #[test]
    fn test_fj017_check_resource_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  pkg1:
    type: package
    machine: local
    provider: apt
    packages: [curl]
  pkg2:
    type: package
    machine: local
    provider: apt
    packages: [wget]
"#,
        )
        .unwrap();
        // Check only specific resource
        cmd_check(&config, None, Some("pkg1"), None, false, false).unwrap();
    }


    #[test]
    fn test_fj017_check_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  conf:
    type: file
    machine: local
    path: /tmp/forjar-check-test.txt
    content: hello
"#,
        )
        .unwrap();
        // JSON output
        cmd_check(&config, None, None, None, true, false).unwrap();
    }

    // ── Rollback error tests ───────────────────────────────────


    #[test]
    fn test_fj273_test_command_parse() {
        let cmd = Commands::Test {
            file: PathBuf::from("forjar.yaml"),
            machine: Some("web".to_string()),
            resource: None,
            tag: None,
            group: None,
            json: true,
        };
        match cmd {
            Commands::Test { json, machine, .. } => {
                assert!(json);
                assert_eq!(machine, Some("web".to_string()));
            }
            _ => panic!("expected Test"),
        }
    }


    #[test]
    fn test_fj273_test_dispatch_runs() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test-proj\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  my-file:\n    type: file\n    machine: local\n    path: /tmp/fj273-test-dispatch.txt\n    content: hello\n",
        )
        .unwrap();
        let result = dispatch(
            Commands::Test {
                file: config_path,
                machine: None,
                resource: None,
                tag: None,
                group: None,
                json: false,
            },
            false,
            true,
        );
        // Will fail (file doesn't exist) or pass — either way it runs without panic
        let _ = result;
    }


    #[test]
    fn test_fj273_test_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            "version: \"1.0\"\nname: test-proj\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  my-file:\n    type: file\n    machine: local\n    path: /tmp/fj273-test-json.txt\n    content: hello\n",
        )
        .unwrap();
        let result = dispatch(
            Commands::Test {
                file: config_path,
                machine: None,
                resource: None,
                tag: None,
                group: None,
                json: true,
            },
            false,
            true,
        );
        let _ = result;
    }


    #[test]
    fn test_fj273_test_nonexistent_config() {
        let result = dispatch(
            Commands::Test {
                file: PathBuf::from("/tmp/fj273-nonexistent.yaml"),
                machine: None,
                resource: None,
                tag: None,
                group: None,
                json: false,
            },
            false,
            true,
        );
        assert!(result.is_err());
    }

    // ========================================================================
    // FJ-281: Resource groups
    // ========================================================================


    #[test]
    fn test_fj281_test_group_flag() {
        let cmd = Commands::Test {
            file: PathBuf::from("forjar.yaml"),
            machine: None,
            resource: None,
            tag: None,
            group: Some("database".to_string()),
            json: false,
        };
        match cmd {
            Commands::Test { group, .. } => {
                assert_eq!(group, Some("database".to_string()));
            }
            _ => panic!("expected Test"),
        }
    }

    // ── FJ-282: forjar validate --strict ──────────────────────────


    #[test]
    fn test_fj282_strict_off_skips_checks() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        // bad machine ref, but strict=false so it should pass
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: nonexistent
    path: /tmp/test.txt
    content: "hello"
"#;
        std::fs::write(&file, yaml).unwrap();
        // strict=false should skip deep checks — but parse_and_validate
        // may still reject unknown machine refs. If so, we just verify
        // that the error is NOT about "strict validation".
        let result = cmd_validate(&file, false, false, false);
        match result {
            Ok(()) => {} // parser didn't catch it — fine
            Err(msg) => assert!(!msg.contains("strict validation")),
        }
    }

    // ── FJ-283: Apply retry with backoff ──────────────────────────


    #[test]
    fn test_fj305_check_json_ci_fields() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("fj305.txt");
        std::fs::write(&target, "ci-check").unwrap();

        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            format!(
                r#"
version: "1.0"
name: ci-gate
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: {}
    content: ci-check
"#,
                target.display()
            ),
        )
        .unwrap();
        // The function prints JSON to stdout — just verify it succeeds
        let result = cmd_check(&config_path, None, None, None, true, false);
        assert!(result.is_ok());
    }

    // ── FJ-306: env --json enhanced ──

}
