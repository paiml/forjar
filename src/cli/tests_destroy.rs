//! Tests: Destroy and rollback.

#![allow(unused_imports)]
use super::apply::*;
use super::commands::*;
use super::destroy::*;
use super::dispatch::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

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
            0,
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

    // ── compute_rollback_changes ─────────────────────────────

    fn minimal_config(name: &str, resources: Vec<(&str, &str)>) -> types::ForjarConfig {
        let mut yaml = format!(
            "version: \"1.0\"\nname: {name}\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n    user: root\n    arch: x86_64\nresources:\n"
        );
        for (id, content) in resources {
            yaml.push_str(&format!(
                "  {id}:\n    type: file\n    machine: local\n    path: /tmp/{id}\n    content: \"{content}\"\n"
            ));
        }
        serde_yaml_ng::from_str(&yaml).unwrap()
    }

    #[test]
    fn rollback_changes_no_diff() {
        let a = minimal_config("test", vec![("f1", "hello")]);
        let b = minimal_config("test", vec![("f1", "hello")]);
        let changes = compute_rollback_changes(&a, &b, 1);
        assert!(changes.is_empty());
    }

    #[test]
    fn rollback_changes_modified_resource() {
        let prev = minimal_config("test", vec![("f1", "old")]);
        let curr = minimal_config("test", vec![("f1", "new")]);
        let changes = compute_rollback_changes(&prev, &curr, 1);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].contains("modified"));
    }

    #[test]
    fn rollback_changes_resource_added_in_current() {
        let prev = minimal_config("test", vec![("f1", "hello")]);
        let curr = minimal_config("test", vec![("f1", "hello"), ("f2", "world")]);
        let changes = compute_rollback_changes(&prev, &curr, 1);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].contains("exists now"));
    }

    #[test]
    fn rollback_changes_resource_removed_in_current() {
        let prev = minimal_config("test", vec![("f1", "hello"), ("f2", "world")]);
        let curr = minimal_config("test", vec![("f1", "hello")]);
        let changes = compute_rollback_changes(&prev, &curr, 2);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].contains("re-added"));
        assert!(changes[0].contains("HEAD~2"));
    }

    #[test]
    fn rollback_changes_mixed() {
        let prev = minimal_config("v1", vec![("f1", "old"), ("f2", "removed")]);
        let curr = minimal_config("v2", vec![("f1", "new"), ("f3", "added")]);
        let changes = compute_rollback_changes(&prev, &curr, 3);
        // f1 modified, f2 will be re-added, f3 exists now
        assert_eq!(changes.len(), 3);
    }

    /// FJ-2005: Verify destroy-log.jsonl is written and parseable.
    #[test]
    fn destroy_log_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let log_path = state_dir.join("destroy-log.jsonl");
        let resource = types::Resource {
            resource_type: types::ResourceType::File,
            machine: types::MachineTarget::Single("local".into()),
            path: Some("/tmp/test.txt".into()),
            content: Some("hello".into()),
            ..Default::default()
        };
        write_destroy_log_entry(&log_path, "f1", &resource, "local", &Default::default());
        let content = std::fs::read_to_string(&log_path).unwrap();
        let entry = types::DestroyLogEntry::from_jsonl(content.lines().next().unwrap()).unwrap();
        assert_eq!(entry.resource_id, "f1");
        assert_eq!(entry.resource_type, "file");
        assert!(entry.reliable_recreate);
        assert!(entry.config_fragment.is_some());
    }

    /// FJ-2005: cleanup_succeeded_entries removes only specified entries.
    #[test]
    fn cleanup_succeeded_entries_partial() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path();
        let machine_dir = state_dir.join("m1");
        std::fs::create_dir_all(&machine_dir).unwrap();

        let rl = |hash: &str| types::ResourceLock {
            resource_type: types::ResourceType::File,
            status: types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: hash.into(),
            details: std::collections::HashMap::new(),
        };
        let mut resources = indexmap::IndexMap::new();
        resources.insert("r1".into(), rl("h1"));
        resources.insert("r2".into(), rl("h2"));
        let lock = types::StateLock {
            schema: "1.0".into(),
            machine: "m1".into(),
            hostname: "m1".into(),
            generated_at: "now".into(),
            generator: "forjar".into(),
            blake3_version: "1.8".into(),
            resources,
        };
        let yaml = serde_yaml_ng::to_string(&lock).unwrap();
        std::fs::write(machine_dir.join("state.lock.yaml"), yaml).unwrap();

        let mut succeeded = std::collections::HashMap::new();
        succeeded.insert("m1".to_string(), vec!["r1".to_string()]);
        cleanup_succeeded_entries(state_dir, &succeeded);

        let remaining = std::fs::read_to_string(machine_dir.join("state.lock.yaml")).unwrap();
        let remaining_lock: types::StateLock = serde_yaml_ng::from_str(&remaining).unwrap();
        assert!(!remaining_lock.resources.contains_key("r1"));
        assert!(remaining_lock.resources.contains_key("r2"));
    }
}
