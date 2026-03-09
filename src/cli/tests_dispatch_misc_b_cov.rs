//! Coverage tests for dispatch_misc_b.rs — cmd_oci_pack, which_runtime, open_state_conn.

use super::dispatch_misc_b::*;

// ── cmd_oci_pack ────────────────────────────────────────────────────

#[test]
fn oci_pack_text_output() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output.oci");
    let result = cmd_oci_pack(dir.path(), "myapp:v1", &out, false);
    assert!(result.is_ok());
}

#[test]
fn oci_pack_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("output.oci");
    let result = cmd_oci_pack(dir.path(), "myapp:latest", &out, true);
    assert!(result.is_ok());
}

#[test]
fn oci_pack_nonexistent_dir() {
    let out = std::path::Path::new("/tmp/output.oci");
    let result = cmd_oci_pack(std::path::Path::new("/nonexistent/dir"), "tag:v1", out, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

// ── which_runtime ───────────────────────────────────────────────────

#[test]
fn which_runtime_bash_exists() {
    // bash should always exist on Linux
    assert!(which_runtime("bash"));
}

#[test]
fn which_runtime_nonexistent() {
    assert!(!which_runtime("definitely-not-a-real-binary-xyz123"));
}

// ── open_state_conn ─────────────────────────────────────────────────

#[test]
fn open_state_conn_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    // Should fall back to :memory: or succeed with empty ingest
    let conn = open_state_conn(dir.path());
    assert!(conn.is_ok());
}

#[test]
fn open_state_conn_with_lock_files() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();
    // Create a machine dir with a lock file for ingest
    let machine_dir = state_dir.join("web");
    std::fs::create_dir_all(&machine_dir).unwrap();
    let lock = crate::core::state::new_lock("web", "web.local");
    crate::core::state::save_lock(state_dir, &lock).unwrap();

    let conn = open_state_conn(state_dir);
    assert!(conn.is_ok());
}

#[test]
fn open_state_conn_nonexistent_dir() {
    // Should fall back to :memory:
    let conn = open_state_conn(std::path::Path::new("/nonexistent/state"));
    assert!(conn.is_ok());
}
