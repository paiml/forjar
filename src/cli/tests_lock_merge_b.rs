//! Coverage tests for lock_merge.rs — merge, rebase, sign operations.

use super::lock_merge::*;
use crate::core::state;

// ── cmd_lock_merge ──────────────────────────────────────────────────

#[test]
fn lock_merge_both_empty_errors() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("nonexistent-from");
    let to = dir.path().join("nonexistent-to");
    let out = dir.path().join("output");
    let result = cmd_lock_merge(&from, &to, &out, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty"));
}

#[test]
fn lock_merge_only_left() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let to = dir.path().join("to-empty"); // doesn't exist
    let out = dir.path().join("output");

    // Create a lock in "from"
    let lock = state::new_lock("web", "web-host");
    state::save_lock(&from, &lock).unwrap();

    let result = cmd_lock_merge(&from, &to, &out, false);
    assert!(result.is_ok());
    // Output should have the web machine
    assert!(out.join("web").join("state.lock.yaml").exists());
}

#[test]
fn lock_merge_only_right() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from-empty"); // doesn't exist
    let to = dir.path().join("to");
    let out = dir.path().join("output");

    let lock = state::new_lock("db", "db-host");
    state::save_lock(&to, &lock).unwrap();

    let result = cmd_lock_merge(&from, &to, &out, false);
    assert!(result.is_ok());
    assert!(out.join("db").join("state.lock.yaml").exists());
}

#[test]
fn lock_merge_both_sides_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let to = dir.path().join("to");
    let out = dir.path().join("output");

    // Both have "web" machine — right takes precedence
    let lock_from = state::new_lock("web", "web-from");
    state::save_lock(&from, &lock_from).unwrap();

    let lock_to = state::new_lock("web", "web-to");
    state::save_lock(&to, &lock_to).unwrap();

    let result = cmd_lock_merge(&from, &to, &out, false);
    assert!(result.is_ok());

    // Verify right took precedence
    let merged = state::load_lock(&out, "web").unwrap().unwrap();
    assert_eq!(merged.hostname, "web-to");
}

#[test]
fn lock_merge_non_overlapping() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let to = dir.path().join("to");
    let out = dir.path().join("output");

    let lock1 = state::new_lock("web", "web-host");
    state::save_lock(&from, &lock1).unwrap();
    let lock2 = state::new_lock("db", "db-host");
    state::save_lock(&to, &lock2).unwrap();

    let result = cmd_lock_merge(&from, &to, &out, false);
    assert!(result.is_ok());
    assert!(out.join("web").join("state.lock.yaml").exists());
    assert!(out.join("db").join("state.lock.yaml").exists());
}

#[test]
fn lock_merge_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let to = dir.path().join("to");
    let out = dir.path().join("output");

    let lock = state::new_lock("app", "app-host");
    state::save_lock(&from, &lock).unwrap();

    let result = cmd_lock_merge(&from, &to, &out, true);
    assert!(result.is_ok());
}

#[test]
fn lock_merge_skips_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let out = dir.path().join("output");
    // Create a hidden dir in "from"
    std::fs::create_dir_all(from.join(".hidden")).unwrap();
    std::fs::write(
        from.join(".hidden").join("state.lock.yaml"),
        "schema: '1'",
    )
    .unwrap();
    // Create a visible dir
    let lock = state::new_lock("visible", "host");
    state::save_lock(&from, &lock).unwrap();

    let to = dir.path().join("to-empty");
    let result = cmd_lock_merge(&from, &to, &out, false);
    assert!(result.is_ok());
    // Only "visible" should be in output
    assert!(out.join("visible").exists());
    assert!(!out.join(".hidden").exists());
}

// ── cmd_lock_sign ───────────────────────────────────────────────────

#[test]
fn lock_sign_signs_existing_locks() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "web-host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_sign(dir.path(), "test-key", false);
    assert!(result.is_ok());
    // Signature file should exist
    assert!(dir.path().join("web").join("lock.sig").exists());
}

#[test]
fn lock_sign_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("app", "app-host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_sign(dir.path(), "key", true);
    assert!(result.is_ok());
}

#[test]
fn lock_sign_empty_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_lock_sign(dir.path(), "key", false);
    assert!(result.is_ok()); // Signs 0 files successfully
}

#[test]
fn lock_sign_nonexistent_dir() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nonexistent");
    let result = cmd_lock_sign(&missing, "key", false);
    assert!(result.is_ok()); // No dir → 0 signed, still ok
}

#[test]
fn lock_sign_multiple_machines() {
    let dir = tempfile::tempdir().unwrap();
    for name in &["web", "db", "cache"] {
        let lock = state::new_lock(name, &format!("{name}-host"));
        state::save_lock(dir.path(), &lock).unwrap();
    }

    let result = cmd_lock_sign(dir.path(), "multi-key", false);
    assert!(result.is_ok());
    for name in &["web", "db", "cache"] {
        assert!(dir.path().join(name).join("lock.sig").exists());
    }
}

#[test]
fn lock_sign_skips_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".hidden")).unwrap();
    std::fs::write(
        dir.path().join(".hidden").join("state.lock.yaml"),
        "schema: '1'",
    )
    .unwrap();

    let lock = state::new_lock("web", "host");
    state::save_lock(dir.path(), &lock).unwrap();

    let result = cmd_lock_sign(dir.path(), "key", false);
    assert!(result.is_ok());
    assert!(!dir.path().join(".hidden").join("lock.sig").exists());
    assert!(dir.path().join("web").join("lock.sig").exists());
}

#[test]
fn lock_sign_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "host");
    state::save_lock(dir.path(), &lock).unwrap();

    cmd_lock_sign(dir.path(), "key-a", false).unwrap();
    let sig1 = std::fs::read_to_string(dir.path().join("web").join("lock.sig")).unwrap();

    cmd_lock_sign(dir.path(), "key-a", false).unwrap();
    let sig2 = std::fs::read_to_string(dir.path().join("web").join("lock.sig")).unwrap();

    assert_eq!(sig1, sig2, "same key should produce same signature");
}

#[test]
fn lock_sign_different_keys_different_sigs() {
    let dir = tempfile::tempdir().unwrap();
    let lock = state::new_lock("web", "host");
    state::save_lock(dir.path(), &lock).unwrap();

    cmd_lock_sign(dir.path(), "key-a", false).unwrap();
    let sig1 = std::fs::read_to_string(dir.path().join("web").join("lock.sig")).unwrap();

    cmd_lock_sign(dir.path(), "key-b", false).unwrap();
    let sig2 = std::fs::read_to_string(dir.path().join("web").join("lock.sig")).unwrap();

    assert_ne!(sig1, sig2, "different keys should produce different signatures");
}
