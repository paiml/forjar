//! Coverage tests for lock_repair.rs — repair, rehash, normalize.

use super::lock_repair::*;
use crate::core::state;

fn create_valid_lock(state_dir: &std::path::Path, machine: &str) {
    let mut lock = state::new_lock(machine, &format!("{machine}.local"));
    lock.resources.insert(
        "cfg".to_string(),
        crate::core::types::ResourceLock {
            resource_type: crate::core::types::ResourceType::File,
            status: crate::core::types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:abc".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    state::save_lock(state_dir, &lock).unwrap();
}

fn create_corrupt_lock(state_dir: &std::path::Path, machine: &str) {
    let machine_dir = state_dir.join(machine);
    std::fs::create_dir_all(&machine_dir).unwrap();
    std::fs::write(machine_dir.join("state.lock.yaml"), "{{{{invalid yaml!!!!").unwrap();
}

// ── cmd_lock_repair ─────────────────────────────────────────────────

#[test]
fn repair_valid_lock_no_change() {
    let dir = tempfile::tempdir().unwrap();
    create_valid_lock(dir.path(), "web");
    let result = cmd_lock_repair(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn repair_corrupt_lock() {
    let dir = tempfile::tempdir().unwrap();
    create_corrupt_lock(dir.path(), "web");
    let result = cmd_lock_repair(dir.path(), false);
    assert!(result.is_ok());
    // Lock file should now be valid
    let content = std::fs::read_to_string(dir.path().join("web").join("state.lock.yaml")).unwrap();
    let _lock: crate::core::types::StateLock = serde_yaml_ng::from_str(&content).unwrap();
}

#[test]
fn repair_json_output() {
    let dir = tempfile::tempdir().unwrap();
    create_valid_lock(dir.path(), "web");
    let result = cmd_lock_repair(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn repair_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_repair(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn repair_mixed_valid_and_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    create_valid_lock(dir.path(), "web");
    create_corrupt_lock(dir.path(), "db");
    let result = cmd_lock_repair(dir.path(), false);
    assert!(result.is_ok());
}

// ── cmd_lock_rehash ─────────────────────────────────────────────────

#[test]
fn rehash_with_resources() {
    let dir = tempfile::tempdir().unwrap();
    create_valid_lock(dir.path(), "web");
    let result = cmd_lock_rehash(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn rehash_json_output() {
    let dir = tempfile::tempdir().unwrap();
    create_valid_lock(dir.path(), "web");
    let result = cmd_lock_rehash(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn rehash_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_rehash(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn rehash_corrupt_lock_skipped() {
    let dir = tempfile::tempdir().unwrap();
    create_corrupt_lock(dir.path(), "web");
    let result = cmd_lock_rehash(dir.path(), false);
    assert!(result.is_ok());
}

// ── cmd_lock_normalize ──────────────────────────────────────────────

#[test]
fn normalize_valid_lock() {
    let dir = tempfile::tempdir().unwrap();
    create_valid_lock(dir.path(), "web");
    let result = cmd_lock_normalize(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn normalize_json_output() {
    let dir = tempfile::tempdir().unwrap();
    create_valid_lock(dir.path(), "web");
    let result = cmd_lock_normalize(dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn normalize_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_normalize(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn normalize_corrupt_lock_skipped() {
    let dir = tempfile::tempdir().unwrap();
    create_corrupt_lock(dir.path(), "web");
    let result = cmd_lock_normalize(dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn normalize_multiple_machines() {
    let dir = tempfile::tempdir().unwrap();
    create_valid_lock(dir.path(), "web");
    create_valid_lock(dir.path(), "db");
    let result = cmd_lock_normalize(dir.path(), false);
    assert!(result.is_ok());
}
