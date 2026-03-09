//! Additional coverage tests for workspace.rs — edge cases and helpers.

use super::workspace::*;

// ── workspace_list_in edge cases ─────────────────────────────────────

#[test]
fn list_no_state_base() {
    let dir = tempfile::tempdir().unwrap();
    let state_base = dir.path().join("nonexistent-state");
    assert!(workspace_list_in(dir.path(), &state_base).is_ok());
}

#[test]
fn list_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let state_base = dir.path().join("state");
    std::fs::create_dir_all(&state_base).unwrap();
    assert!(workspace_list_in(dir.path(), &state_base).is_ok());
}

#[test]
fn list_with_active_marker() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state_base = root.join("state");
    std::fs::create_dir_all(&state_base).unwrap();
    workspace_new_in(root, &state_base, "active").unwrap();
    workspace_new_in(root, &state_base, "inactive").unwrap();
    workspace_select_in(root, &state_base, "active").unwrap();
    assert!(workspace_list_in(root, &state_base).is_ok());
}

// ── workspace_delete_in edge cases ───────────────────────────────────

#[test]
fn delete_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    let state_base = dir.path().join("state");
    std::fs::create_dir_all(&state_base).unwrap();
    let result = workspace_delete_in(dir.path(), &state_base, "ghost", true);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

#[test]
fn delete_non_active_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state_base = root.join("state");
    std::fs::create_dir_all(&state_base).unwrap();
    workspace_new_in(root, &state_base, "keep").unwrap();
    workspace_new_in(root, &state_base, "delete-me").unwrap();
    workspace_select_in(root, &state_base, "keep").unwrap();
    // Delete non-active workspace shouldn't clear workspace file
    workspace_delete_in(root, &state_base, "delete-me", true).unwrap();
    assert_eq!(current_workspace_in(root).as_deref(), Some("keep"));
}

// ── current_workspace_in edge cases ──────────────────────────────────

#[test]
fn current_workspace_empty_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".forjar")).unwrap();
    std::fs::write(root.join(".forjar/workspace"), "").unwrap();
    assert!(current_workspace_in(root).is_none());
}

#[test]
fn current_workspace_whitespace_only() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".forjar")).unwrap();
    std::fs::write(root.join(".forjar/workspace"), "  \n  ").unwrap();
    assert!(current_workspace_in(root).is_none());
}

#[test]
fn current_workspace_with_newline() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".forjar")).unwrap();
    std::fs::write(root.join(".forjar/workspace"), "production\n").unwrap();
    assert_eq!(current_workspace_in(root).as_deref(), Some("production"));
}

// ── resolve_state_dir ────────────────────────────────────────────────

#[test]
fn resolve_with_workspace_flag() {
    let dir = std::path::Path::new("/tmp/state");
    let resolved = resolve_state_dir(dir, Some("staging"));
    assert_eq!(resolved, std::path::PathBuf::from("/tmp/state/staging"));
}

// ── inject_workspace_param ───────────────────────────────────────────

#[test]
fn inject_with_explicit_flag() {
    let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources: {}\n";
    let mut config: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    inject_workspace_param(&mut config, Some("ci-staging"));
    assert_eq!(
        config.params.get("workspace").unwrap(),
        &serde_yaml_ng::Value::String("ci-staging".to_string())
    );
}
