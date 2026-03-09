//! Coverage tests for infra.rs — cmd_state_list, cmd_state_mv, cmd_state_rm.

use super::infra::*;
use crate::core::{state, types};

fn create_lock_with_resource(
    state_dir: &std::path::Path,
    machine: &str,
    resource_id: &str,
) {
    let mut lock = state::new_lock(machine, &format!("{machine}.local"));
    lock.resources.insert(
        resource_id.to_string(),
        types::ResourceLock {
            resource_type: types::ResourceType::File,
            status: types::ResourceStatus::Converged,
            applied_at: Some("2026-01-01T00:00:00Z".to_string()),
            duration_seconds: Some(1.0),
            hash: "blake3:abcdef123456".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    state::save_lock(state_dir, &lock).unwrap();
}

// ── cmd_state_list ──────────────────────────────────────────────────

#[test]
fn state_list_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_state_list(dir.path(), None, false);
    assert!(result.is_ok());
}

#[test]
fn state_list_nonexistent_dir() {
    let result = cmd_state_list(std::path::Path::new("/nonexistent/state"), None, false);
    assert!(result.is_ok());
}

#[test]
fn state_list_nonexistent_json() {
    let result = cmd_state_list(std::path::Path::new("/nonexistent/state"), None, true);
    assert!(result.is_ok());
}

#[test]
fn state_list_with_resources() {
    let dir = tempfile::tempdir().unwrap();
    create_lock_with_resource(dir.path(), "web", "nginx-cfg");
    let result = cmd_state_list(dir.path(), None, false);
    assert!(result.is_ok());
}

#[test]
fn state_list_json() {
    let dir = tempfile::tempdir().unwrap();
    create_lock_with_resource(dir.path(), "web", "nginx-cfg");
    let result = cmd_state_list(dir.path(), None, true);
    assert!(result.is_ok());
}

#[test]
fn state_list_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    create_lock_with_resource(dir.path(), "web", "nginx-cfg");
    create_lock_with_resource(dir.path(), "db", "pg-cfg");
    let result = cmd_state_list(dir.path(), Some("web"), false);
    assert!(result.is_ok());
}

// ── cmd_state_mv ────────────────────────────────────────────────────

#[test]
fn state_mv_renames_resource() {
    let dir = tempfile::tempdir().unwrap();
    create_lock_with_resource(dir.path(), "web", "old-name");
    let result = cmd_state_mv(dir.path(), "old-name", "new-name", None);
    assert!(result.is_ok());
    let lock = state::load_lock(dir.path(), "web").unwrap().unwrap();
    assert!(!lock.resources.contains_key("old-name"));
    assert!(lock.resources.contains_key("new-name"));
}

#[test]
fn state_mv_same_id_error() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_state_mv(dir.path(), "same", "same", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("same"));
}

#[test]
fn state_mv_nonexistent_dir() {
    let result = cmd_state_mv(
        std::path::Path::new("/nonexistent/state"),
        "a",
        "b",
        None,
    );
    assert!(result.is_err());
}

#[test]
fn state_mv_resource_not_found() {
    let dir = tempfile::tempdir().unwrap();
    create_lock_with_resource(dir.path(), "web", "existing");
    let result = cmd_state_mv(dir.path(), "nonexistent", "new-name", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn state_mv_target_exists() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = state::new_lock("web", "web.local");
    lock.resources.insert(
        "r1".to_string(),
        types::ResourceLock {
            resource_type: types::ResourceType::File,
            status: types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:aaa".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    lock.resources.insert(
        "r2".to_string(),
        types::ResourceLock {
            resource_type: types::ResourceType::File,
            status: types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:bbb".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    state::save_lock(dir.path(), &lock).unwrap();
    let result = cmd_state_mv(dir.path(), "r1", "r2", None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}

#[test]
fn state_mv_with_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    create_lock_with_resource(dir.path(), "web", "cfg");
    create_lock_with_resource(dir.path(), "db", "cfg");
    let result = cmd_state_mv(dir.path(), "cfg", "new-cfg", Some("web"));
    assert!(result.is_ok());
    // web should be renamed
    let web_lock = state::load_lock(dir.path(), "web").unwrap().unwrap();
    assert!(web_lock.resources.contains_key("new-cfg"));
    // db should remain unchanged
    let db_lock = state::load_lock(dir.path(), "db").unwrap().unwrap();
    assert!(db_lock.resources.contains_key("cfg"));
}

// ── cmd_state_rm ────────────────────────────────────────────────────

#[test]
fn state_rm_removes_resource() {
    let dir = tempfile::tempdir().unwrap();
    create_lock_with_resource(dir.path(), "web", "cfg");
    let result = cmd_state_rm(dir.path(), "cfg", None, true);
    assert!(result.is_ok());
    let lock = state::load_lock(dir.path(), "web").unwrap().unwrap();
    assert!(!lock.resources.contains_key("cfg"));
}

#[test]
fn state_rm_nonexistent_dir() {
    let result = cmd_state_rm(
        std::path::Path::new("/nonexistent/state"),
        "r",
        None,
        true,
    );
    assert!(result.is_err());
}

#[test]
fn state_rm_resource_not_found() {
    let dir = tempfile::tempdir().unwrap();
    create_lock_with_resource(dir.path(), "web", "existing");
    let result = cmd_state_rm(dir.path(), "nonexistent", None, true);
    assert!(result.is_err());
}

#[test]
fn state_rm_with_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    create_lock_with_resource(dir.path(), "web", "cfg");
    create_lock_with_resource(dir.path(), "db", "cfg");
    let result = cmd_state_rm(dir.path(), "cfg", Some("web"), true);
    assert!(result.is_ok());
    // web cfg should be removed
    let web_lock = state::load_lock(dir.path(), "web").unwrap().unwrap();
    assert!(!web_lock.resources.contains_key("cfg"));
    // db cfg should remain
    let db_lock = state::load_lock(dir.path(), "db").unwrap().unwrap();
    assert!(db_lock.resources.contains_key("cfg"));
}
