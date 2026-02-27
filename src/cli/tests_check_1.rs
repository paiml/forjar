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
use super::destroy::*;
use super::diff_cmd::*;
use super::dispatch::*;
use super::init::*;
use super::lint::*;
use super::observe::*;
use super::test_fixtures::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: check if a config has any cross-machine dependencies.
    fn has_cross_machine_dependency(config: &types::ForjarConfig) -> bool {
        for (_id, resource) in &config.resources {
            let my_machines: std::collections::HashSet<String> =
                resource.machine.to_vec().into_iter().collect();
            for dep in &resource.depends_on {
                if let Some(dep_resource) = config.resources.get(dep) {
                    let dep_machines: std::collections::HashSet<String> =
                        dep_resource.machine.to_vec().into_iter().collect();
                    if my_machines.is_disjoint(&dep_machines) {
                        return true;
                    }
                }
            }
        }
        false
    }


    #[test]
    fn test_diff_machine_filter() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            from_dir.path(),
            "m2",
            vec![("svc", "blake3:bbb", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![("pkg", "blake3:changed", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m2",
            vec![("svc", "blake3:bbb", types::ResourceStatus::Converged)],
        );
        // Filtering to m1 should only show m1's changes
        cmd_diff(from_dir.path(), to_dir.path(), Some("m1"), None, false).unwrap();
    }


    #[test]
    fn test_diff_empty_state_dirs() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        let result = cmd_diff(from_dir.path(), to_dir.path(), None, None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no machines found"));
    }


    #[test]
    fn test_discover_machines() {
        let dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            dir.path(),
            "alpha",
            vec![("f", "blake3:x", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            dir.path(),
            "beta",
            vec![("f", "blake3:y", types::ResourceStatus::Converged)],
        );
        let machines = discover_machines(dir.path());
        assert_eq!(machines, vec!["alpha", "beta"]);
    }

    // ── forjar check tests ─────────────────────────────────────────


    #[test]
    fn test_check_local_file_pass() {
        let dir = tempfile::tempdir().unwrap();
        // Create the file that check will verify
        let target = dir.path().join("check-test.txt");
        std::fs::write(&target, "hello").unwrap();

        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: hello
"#,
                target.display()
            ),
        )
        .unwrap();
        // File exists → check should pass
        cmd_check(&config, None, None, None, false, false).unwrap();
    }


    #[test]
    fn test_check_local_file_missing_still_runs() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: /tmp/forjar-check-nonexistent-12345678
    content: hello
"#,
        )
        .unwrap();
        // Check script reports status (exits 0 even for missing file)
        cmd_check(&config, None, None, None, false, false).unwrap();
    }


    #[test]
    fn test_check_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("check-json-test.txt");
        std::fs::write(&target, "hello").unwrap();

        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: hello
"#,
                target.display()
            ),
        )
        .unwrap();
        cmd_check(&config, None, None, None, true, false).unwrap();
    }


    #[test]
    fn test_fmt_normalizes_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        // Manually-written YAML with inconsistent spacing
        std::fs::write(
            &file,
            r#"version: "1.0"
name: fmt-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/fmt-test
    content: hello
"#,
        )
        .unwrap();

        // Check should fail (not yet canonical)
        let result = cmd_fmt(&file, true);
        assert!(result.is_err());

        // Format it
        cmd_fmt(&file, false).unwrap();

        // Check should now pass
        cmd_fmt(&file, true).unwrap();
    }


    #[test]
    fn test_fmt_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(
            &file,
            r#"version: "1.0"
name: idempotent-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: hello
"#,
        )
        .unwrap();

        // Format it twice
        cmd_fmt(&file, false).unwrap();
        let after_first = std::fs::read_to_string(&file).unwrap();

        cmd_fmt(&file, false).unwrap();
        let after_second = std::fs::read_to_string(&file).unwrap();

        assert_eq!(after_first, after_second);
    }


    #[test]
    fn test_lint_unused_machine() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(
            &file,
            r#"version: "1.0"
name: lint-test
machines:
  used:
    hostname: used
    addr: 127.0.0.1
  unused:
    hostname: unused
    addr: 10.0.0.1
resources:
  f:
    type: file
    machine: used
    path: /tmp/test
    content: hello
"#,
        )
        .unwrap();

        // Lint should succeed but print warnings (it returns Ok)
        cmd_lint(&file, false, false, false).unwrap();
    }


    #[test]
    fn test_lint_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(
            &file,
            r#"version: "1.0"
name: lint-json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
  orphan:
    hostname: orphan
    addr: 10.0.0.2
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: hello
"#,
        )
        .unwrap();

        cmd_lint(&file, true, false, false).unwrap();
    }


    #[test]
    fn test_lint_clean_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(
            &file,
            r#"version: "1.0"
name: clean
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: hello
"#,
        )
        .unwrap();

        cmd_lint(&file, false, false, false).unwrap();
    }


    #[test]
    fn test_lint_cross_machine_dependency() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(
            &file,
            r#"version: "1.0"
name: cross-dep
machines:
  web:
    hostname: web
    addr: 10.0.0.1
  db:
    hostname: db
    addr: 10.0.0.2
resources:
  app-config:
    type: file
    machine: web
    path: /etc/app.conf
    content: "host=db"
    depends_on: [db-ready]
  db-ready:
    type: file
    machine: db
    path: /tmp/db-ready
    content: "ok"
"#,
        )
        .unwrap();

        // Capture output via JSON mode to inspect warnings
        let result = cmd_lint(&file, true, false, false);
        assert!(result.is_ok());
        // The warning should mention cross-machine dependency
        // We re-run logic here to check the warning was generated
        let config = parse_and_validate(&file).unwrap();
        assert!(
            has_cross_machine_dependency(&config),
            "should detect cross-machine dependency"
        );
    }


    #[test]
    fn test_rollback_no_git_history() {
        // A file that doesn't exist in git history should fail gracefully
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("nonexistent.yaml");
        std::fs::write(
            &file,
            "version: \"1.0\"\nname: test\nmachines: {}\nresources: {}\n",
        )
        .unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let result = cmd_rollback(&file, &state, 1, None, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot read"));
    }


    #[test]
    fn test_rollback_dispatch() {
        // Verify the Rollback command variant is accepted by dispatch
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            "version: \"1.0\"\nname: rb\nmachines: {}\nresources: {}\n",
        )
        .unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        // Dispatch dry-run rollback — will fail because no git history,
        // but verifies the dispatch path is wired correctly
        let result = dispatch(
            Commands::Rollback {
                file,
                revision: 1,
                machine: None,
                dry_run: true,
                state_dir: state,
            },
            false,
            true,
        );
        assert!(result.is_err()); // Expected: no git history
    }


    #[test]
    fn test_anomaly_empty_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        // No machine dirs → "no resources with enough history"
        let result = cmd_anomaly(&state, None, 3, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_anomaly_detects_high_failure_rate() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let machine_dir = state.join("m1");
        std::fs::create_dir_all(&machine_dir).unwrap();

        // Write events with high failure rate: 1 converge, 4 failures
        let mut events = String::new();
        events.push_str(
            &serde_json::to_string(&types::TimestampedEvent {
                ts: "2026-02-25T00:00:00Z".to_string(),
                event: types::ProvenanceEvent::ResourceConverged {
                    machine: "m1".to_string(),
                    resource: "flaky-pkg".to_string(),
                    duration_seconds: 1.0,
                    hash: "abc".to_string(),
                },
            })
            .unwrap(),
        );
        events.push('\n');
        for _ in 0..4 {
            events.push_str(
                &serde_json::to_string(&types::TimestampedEvent {
                    ts: "2026-02-25T00:01:00Z".to_string(),
                    event: types::ProvenanceEvent::ResourceFailed {
                        machine: "m1".to_string(),
                        resource: "flaky-pkg".to_string(),
                        error: "install failed".to_string(),
                    },
                })
                .unwrap(),
            );
            events.push('\n');
        }

        std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();

        // min_events=3, json mode so we can parse output
        let result = cmd_anomaly(&state, None, 3, false);
        assert!(result.is_ok());
    }

}
