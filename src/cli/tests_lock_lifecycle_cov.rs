//! Coverage tests for lock_lifecycle.rs — compress, archive, snapshot, defrag.

use super::lock_lifecycle::*;
use crate::core::state;

fn create_state_with_lock(state_dir: &std::path::Path, machine: &str) {
    let mut lock = state::new_lock(machine, &format!("{machine}.local"));
    lock.resources.insert(
        "zz-cfg".to_string(),
        crate::core::types::ResourceLock {
            resource_type: crate::core::types::ResourceType::File,
            status: crate::core::types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:abc".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    lock.resources.insert(
        "aa-pkg".to_string(),
        crate::core::types::ResourceLock {
            resource_type: crate::core::types::ResourceType::Package,
            status: crate::core::types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:def".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    state::save_lock(state_dir, &lock).unwrap();
}

// ── cmd_lock_compress ───────────────────────────────────────────────

#[test]
fn compress_with_lock_file() {
    let dir = tempfile::tempdir().unwrap();
    create_state_with_lock(dir.path(), "web");
    // Add a comment to make compression save bytes
    let lock_path = dir.path().join("web").join("state.lock.yaml");
    let mut content = std::fs::read_to_string(&lock_path).unwrap();
    content.push_str("\n# this is a comment\n\n# another comment\n");
    std::fs::write(&lock_path, content).unwrap();

    let result = cmd_lock_compress(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn compress_json_output() {
    let dir = tempfile::tempdir().unwrap();
    create_state_with_lock(dir.path(), "web");
    let result = cmd_lock_compress(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn compress_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_compress(dir.path(), false);
    assert!(result.is_ok());
}

// ── cmd_lock_archive ────────────────────────────────────────────────

#[test]
fn archive_with_event_log() {
    let dir = tempfile::tempdir().unwrap();
    create_state_with_lock(dir.path(), "web");
    // Create an events file at the expected location
    let events_path = dir.path().join("web.events.jsonl");
    std::fs::write(&events_path, "{\"event\":\"test\"}\n").unwrap();

    let result = cmd_lock_archive(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn archive_json_output() {
    let dir = tempfile::tempdir().unwrap();
    create_state_with_lock(dir.path(), "web");
    let result = cmd_lock_archive(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn archive_no_event_logs() {
    let dir = tempfile::tempdir().unwrap();
    create_state_with_lock(dir.path(), "web");
    let result = cmd_lock_archive(dir.path(), false);
    assert!(result.is_ok());
}

// ── cmd_lock_snapshot ───────────────────────────────────────────────

#[test]
fn snapshot_creates_copy() {
    let dir = tempfile::tempdir().unwrap();
    create_state_with_lock(dir.path(), "web");
    let result = cmd_lock_snapshot(dir.path(), false);
    assert!(result.is_ok());
    // Snapshot dir should exist
    assert!(dir.path().join("snapshots").exists());
}

#[test]
fn snapshot_json_output() {
    let dir = tempfile::tempdir().unwrap();
    create_state_with_lock(dir.path(), "web");
    let result = cmd_lock_snapshot(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn snapshot_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_snapshot(dir.path(), false);
    assert!(result.is_ok());
}

// ── cmd_lock_defrag ─────────────────────────────────────────────────

#[test]
fn defrag_reorders_resources() {
    let dir = tempfile::tempdir().unwrap();
    create_state_with_lock(dir.path(), "web");
    let result = cmd_lock_defrag(dir.path(), false);
    assert!(result.is_ok());

    // Verify resources are now alphabetically ordered
    let lock = state::load_lock(dir.path(), "web").unwrap().unwrap();
    let keys: Vec<&String> = lock.resources.keys().collect();
    assert_eq!(keys[0], "aa-pkg");
    assert_eq!(keys[1], "zz-cfg");
}

#[test]
fn defrag_json_output() {
    let dir = tempfile::tempdir().unwrap();
    create_state_with_lock(dir.path(), "web");
    let result = cmd_lock_defrag(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn defrag_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_defrag(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn defrag_multiple_machines() {
    let dir = tempfile::tempdir().unwrap();
    create_state_with_lock(dir.path(), "web");
    create_state_with_lock(dir.path(), "db");
    let result = cmd_lock_defrag(dir.path(), false);
    assert!(result.is_ok());
}
