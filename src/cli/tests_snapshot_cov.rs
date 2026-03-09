//! Additional coverage tests for snapshot.rs — edge cases and helper functions.

use super::snapshot::*;

// ── snapshots_dir ────────────────────────────────────────────────────

#[test]
fn snapshots_dir_path() {
    let p = snapshots_dir(std::path::Path::new("/var/state"));
    assert_eq!(p, std::path::PathBuf::from("/var/state/snapshots"));
}

// ── copy_dir_recursive edge cases ────────────────────────────────────

#[test]
fn copy_dir_empty() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("empty-src");
    let dst = dir.path().join("empty-dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    assert!(copy_dir_recursive(&src, &dst, "").is_ok());
}

#[test]
fn copy_dir_nested_deep() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(src.join("a/b/c")).unwrap();
    std::fs::write(src.join("a/b/c/deep.txt"), "nested").unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    copy_dir_recursive(&src, &dst, "").unwrap();
    assert_eq!(
        std::fs::read_to_string(dst.join("a/b/c/deep.txt")).unwrap(),
        "nested"
    );
}

#[test]
fn copy_dir_skip_multiple_entries() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("keep.txt"), "keep").unwrap();
    std::fs::write(src.join("skip-me"), "skip").unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    copy_dir_recursive(&src, &dst, "skip-me").unwrap();
    assert!(dst.join("keep.txt").exists());
    assert!(!dst.join("skip-me").exists());
}

// ── cmd_snapshot_save edge cases ─────────────────────────────────────

#[test]
fn snapshot_save_empty_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    // Empty state dir — save should still work
    cmd_snapshot_save("empty-snap", &state_dir).unwrap();
    assert!(state_dir.join("snapshots/empty-snap/.snapshot.yaml").exists());
}

#[test]
fn snapshot_save_with_files_at_root() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(state_dir.join("config.yaml"), "version: 1").unwrap();
    cmd_snapshot_save("with-files", &state_dir).unwrap();
    assert!(state_dir.join("snapshots/with-files/config.yaml").exists());
}

// ── cmd_snapshot_list edge cases ─────────────────────────────────────

#[test]
fn snapshot_list_with_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    cmd_snapshot_save("first", &state_dir).unwrap();
    cmd_snapshot_save("second", &state_dir).unwrap();
    // Text output
    assert!(cmd_snapshot_list(&state_dir, false).is_ok());
    // JSON output
    assert!(cmd_snapshot_list(&state_dir, true).is_ok());
}

#[test]
fn snapshot_list_no_metadata_file() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let snap_dir = state_dir.join("snapshots/manual");
    std::fs::create_dir_all(&snap_dir).unwrap();
    // No .snapshot.yaml — created_at should be "unknown"
    assert!(cmd_snapshot_list(&state_dir, false).is_ok());
    assert!(cmd_snapshot_list(&state_dir, true).is_ok());
}

// ── cmd_snapshot_restore edge cases ──────────────────────────────────

#[test]
fn snapshot_restore_removes_new_files() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(state_dir.join("original.txt"), "v1").unwrap();
    cmd_snapshot_save("clean", &state_dir).unwrap();
    // Add new file after snapshot
    std::fs::write(state_dir.join("extra.txt"), "extra").unwrap();
    assert!(state_dir.join("extra.txt").exists());
    // Restore removes it
    cmd_snapshot_restore("clean", &state_dir, true).unwrap();
    assert!(!state_dir.join("extra.txt").exists());
    assert!(state_dir.join("original.txt").exists());
}

#[test]
fn snapshot_restore_removes_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    cmd_snapshot_save("bare", &state_dir).unwrap();
    // Add directory after snapshot
    std::fs::create_dir_all(state_dir.join("newdir")).unwrap();
    std::fs::write(state_dir.join("newdir/file.txt"), "data").unwrap();
    cmd_snapshot_restore("bare", &state_dir, true).unwrap();
    assert!(!state_dir.join("newdir").exists());
}

// ── cmd_snapshot_delete ──────────────────────────────────────────────

#[test]
fn snapshot_delete_cleans_up() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    cmd_snapshot_save("cleanup", &state_dir).unwrap();
    let snap = state_dir.join("snapshots/cleanup");
    assert!(snap.exists());
    cmd_snapshot_delete("cleanup", &state_dir).unwrap();
    assert!(!snap.exists());
    // Snapshots dir still exists but is empty
    assert!(state_dir.join("snapshots").exists());
}
