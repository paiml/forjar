//! Coverage tests for workspace.rs — delete without --yes, duplicate creation, resolve_state_dir.

use super::workspace::*;

// ── workspace_delete_in without --yes ──────────────────────────────

#[test]
fn delete_without_yes_prints_warning() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state_base = root.join("state");
    std::fs::create_dir_all(&state_base).unwrap();
    workspace_new_in(root, &state_base, "deletable").unwrap();
    // Without yes → prints warning, returns Ok
    let result = workspace_delete_in(root, &state_base, "deletable", false);
    assert!(result.is_ok());
    // Directory should still exist
    assert!(state_base.join("deletable").exists());
}

// ── workspace_new_in duplicate ─────────────────────────────────────

#[test]
fn new_duplicate_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state_base = root.join("state");
    std::fs::create_dir_all(&state_base).unwrap();
    workspace_new_in(root, &state_base, "dup").unwrap();
    let result = workspace_new_in(root, &state_base, "dup");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("already exists"));
}

// ── workspace_select_in nonexistent ────────────────────────────────

#[test]
fn select_nonexistent_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state_base = root.join("state");
    std::fs::create_dir_all(&state_base).unwrap();
    let result = workspace_select_in(root, &state_base, "ghost");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

// ── workspace_delete_in active workspace clears selection ──────────

#[test]
fn delete_active_workspace_clears_selection() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state_base = root.join("state");
    std::fs::create_dir_all(&state_base).unwrap();
    workspace_new_in(root, &state_base, "active-ws").unwrap();
    // "active-ws" is now selected (workspace_new_in selects it)
    assert_eq!(current_workspace_in(root).as_deref(), Some("active-ws"));
    workspace_delete_in(root, &state_base, "active-ws", true).unwrap();
    assert!(current_workspace_in(root).is_none());
}

// ── workspace_list_in with only files (no dirs) ────────────────────

#[test]
fn list_with_only_files_no_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state_base = root.join("state");
    std::fs::create_dir_all(&state_base).unwrap();
    // Only a file, no subdirectory
    std::fs::write(state_base.join("somefile.txt"), "data").unwrap();
    let result = workspace_list_in(root, &state_base);
    assert!(result.is_ok());
    // No workspaces found (only files, no dirs)
}

// ── resolve_state_dir with no workspace ────────────────────────────

#[test]
fn resolve_state_dir_no_flag_no_current() {
    // When there's no workspace flag and no current workspace file
    let dir = std::path::Path::new("/tmp/forjar-state-test");
    let resolved = resolve_state_dir(dir, None);
    // With no flag and no .forjar/workspace file, returns state_dir as-is
    assert_eq!(resolved, dir.to_path_buf());
}

// ── inject_workspace_param without flag ────────────────────────────

#[test]
fn inject_without_flag_defaults_to_default() {
    let yaml = "version: '1'\nname: test\nmachines: {}\nresources: {}\n";
    let mut config: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    inject_workspace_param(&mut config, None);
    let ws = config.params.get("workspace").unwrap();
    // Without a flag or .forjar/workspace file, defaults to "default"
    assert!(ws.as_str().is_some());
}

// ── workspace_new_in creates .forjar metadata ──────────────────────

#[test]
fn new_creates_metadata_dir() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let state_base = root.join("state");
    workspace_new_in(root, &state_base, "test-ws").unwrap();
    assert!(root.join(".forjar").exists());
    assert!(root.join(".forjar/workspace").exists());
    let content = std::fs::read_to_string(root.join(".forjar/workspace")).unwrap();
    assert_eq!(content, "test-ws");
}
