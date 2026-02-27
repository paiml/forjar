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
use super::apply::*;
use super::diff_cmd::*;
use super::drift::*;
use super::test_fixtures::*;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_auto_commit_in_git_repo() {
        // auto_commit=true in a temp dir that IS a git repo
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        // Init git repo in temp dir
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        // Initial commit so the repo is in a valid state
        std::fs::write(dir.path().join(".gitkeep"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", ".gitkeep"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let target = dir.path().join("auto-commit.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: autocommit-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: "auto commit test"
"#,
                target.display()
            ),
        )
        .unwrap();

        // auto_commit=true (second to last arg)
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
            true,
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

        // Verify git committed the state
        let output = std::process::Command::new("git")
            .args(["log", "--oneline", "-1"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&output.stdout);
        assert!(log.contains("forjar:"));
    }


    #[test]
    fn test_drift_alert_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let test_file = dir.path().join("drift-alert.txt");
        std::fs::write(&test_file, "current").unwrap();

        let alert_marker = dir.path().join("alert-fired");

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(test_file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:wrong_hash".to_string()),
        );
        resources.insert(
            "drifted-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: "blake3:x".to_string(),
                details,
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "alertbox".to_string(),
            hostname: "alertbox".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // alert_cmd touches a file when drift detected
        let alert_cmd = format!("touch {}", alert_marker.display());
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            Some(&alert_cmd),
            false,
            false, // dry_run
            false,
            false,
            None, // no env_file
        )
        .unwrap();

        // Verify alert command ran
        assert!(alert_marker.exists());
    }


    #[test]
    fn test_drift_alert_cmd_not_fired_when_no_drift() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let alert_marker = dir.path().join("should-not-exist");
        let alert_cmd = format!("touch {}", alert_marker.display());

        // Empty state dir — no drift
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            Some(&alert_cmd),
            false,
            false, // dry_run
            false,
            false,
            None, // no env_file
        )
        .unwrap();

        // Alert should NOT have fired
        assert!(!alert_marker.exists());
    }


    #[test]
    fn test_drift_auto_remediate() {
        // Create a file resource, apply, tamper, then drift --auto-remediate
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir
            .path()
            .join("auto-remediate-test.txt")
            .to_string_lossy()
            .to_string();
        std::fs::write(
            &config,
            format!(
                r#"version: "1.0"
name: remediation-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {}
    content: "original content"
    mode: "0644"
"#,
                target
            ),
        )
        .unwrap();

        // Apply to create the file
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
        assert!(std::path::Path::new(&target).exists());

        // Tamper with the file
        std::fs::write(&target, "tampered content").unwrap();

        // Drift with auto-remediate should detect and fix
        cmd_drift(
            &config, &state, None, false, None, true, // auto_remediate
            false, false, false, None, // no env_file
        )
        .unwrap();

        // File should be restored to original content
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content.trim(), "original content");

        // Clean up
        let _ = std::fs::remove_file(&target);
    }


    #[test]
    fn test_drift_dry_run_lists_resources() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        // Create a lock with two resources
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "web-config".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "abc123".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        resources.insert(
            "db-config".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "def456".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "local".to_string(),
            hostname: "local".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // Dry-run should succeed without connecting to any machine
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            None,
            false,
            true, // dry_run
            false,
            false,
            None, // no env_file
        )
        .unwrap();
    }


    #[test]
    fn test_diff_added_resource() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![
                ("pkg", "blake3:aaa", types::ResourceStatus::Converged),
                ("conf", "blake3:bbb", types::ResourceStatus::Converged),
            ],
        );
        cmd_diff(from_dir.path(), to_dir.path(), None, None, false).unwrap();
    }


    #[test]
    fn test_diff_removed_resource() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![
                ("pkg", "blake3:aaa", types::ResourceStatus::Converged),
                ("conf", "blake3:bbb", types::ResourceStatus::Converged),
            ],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        cmd_diff(from_dir.path(), to_dir.path(), None, None, false).unwrap();
    }


    #[test]
    fn test_diff_changed_hash() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![("pkg", "blake3:bbb", types::ResourceStatus::Converged)],
        );
        cmd_diff(from_dir.path(), to_dir.path(), None, None, false).unwrap();
    }


    #[test]
    fn test_diff_no_changes() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        cmd_diff(from_dir.path(), to_dir.path(), None, None, false).unwrap();
    }


    #[test]
    fn test_diff_json_output() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![
                ("pkg", "blake3:bbb", types::ResourceStatus::Converged),
                ("svc", "blake3:ccc", types::ResourceStatus::Converged),
            ],
        );
        cmd_diff(from_dir.path(), to_dir.path(), None, None, true).unwrap();
    }

}
