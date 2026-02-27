//! Tests: Destroy and rollback.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::destroy::*;
use super::apply::*;
use super::commands::*;
use super::dispatch::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj061_destroy_requires_yes() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: /tmp/forjar-destroy-test.txt
    content: "x"
"#,
        )
        .unwrap();
        let result = cmd_destroy(&config, &state, None, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--yes"));
    }


    #[test]
    fn test_fj061_destroy_local_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir.path().join("destroy-me.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: destroy-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  victim:
    type: file
    machine: local
    path: {}
    content: "will be destroyed"
"#,
                target.display()
            ),
        )
        .unwrap();

        // First, apply so the file exists and state is saved
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )
        .unwrap();
        assert!(target.exists());
        assert!(state.join("local").join("state.lock.yaml").exists());

        // Now destroy
        cmd_destroy(&config, &state, None, true, false).unwrap();

        // File should be removed
        assert!(!target.exists());

        // State lock should be cleaned up
        assert!(!state.join("local").join("state.lock.yaml").exists());
    }


    #[test]
    fn test_fj061_destroy_verbose() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir.path().join("destroy-verbose.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: verbose-destroy
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: "verbose test"
"#,
                target.display()
            ),
        )
        .unwrap();

        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )
        .unwrap();
        cmd_destroy(&config, &state, None, true, true).unwrap();
        assert!(!target.exists());
    }


    #[test]
    fn test_fj061_destroy_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target_a = dir.path().join("file-a.txt");
        let target_b = dir.path().join("file-b.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: filter-test
machines:
  local-a:
    hostname: localhost
    addr: 127.0.0.1
  local-b:
    hostname: localhost
    addr: 127.0.0.1
resources:
  fa:
    type: file
    machine: local-a
    path: {}
    content: "a"
  fb:
    type: file
    machine: local-b
    path: {}
    content: "b"
"#,
                target_a.display(),
                target_b.display()
            ),
        )
        .unwrap();

        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )
        .unwrap();
        assert!(target_a.exists());
        assert!(target_b.exists());

        // Only destroy machine local-a
        cmd_destroy(&config, &state, Some("local-a"), true, false).unwrap();
        assert!(!target_a.exists());
        assert!(target_b.exists()); // b should still exist
    }


    #[test]
    fn test_fj061_dispatch_destroy() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir.path().join("dispatch-destroy.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: dispatch-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: "dispatch"
"#,
                target.display()
            ),
        )
        .unwrap();

        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )
        .unwrap();
        dispatch(
            Commands::Destroy(DestroyArgs {
                file: config,
                machine: None,
                yes: true,
                state_dir: state,
            }),
            false,
            true,
        )
        .unwrap();
        assert!(!target.exists());
    }


    #[test]
    fn test_fj017_rollback_invalid_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nonexistent.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        // Rollback with nonexistent config should fail
        let result = cmd_rollback(&config, &state, 1, None, true, false);
        assert!(result.is_err());
    }

    // ── Apply with param overrides ─────────────────────────────

}
