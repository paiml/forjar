//! Workspace management.

use crate::core::types;
use std::path::{Path, PathBuf};

pub(crate) fn cmd_workspace_new(name: &str) -> Result<(), String> {
    workspace_new_in(Path::new("."), Path::new("state"), name)
}

pub(crate) fn cmd_workspace_list() -> Result<(), String> {
    workspace_list_in(Path::new("."), Path::new("state"))
}

pub(crate) fn cmd_workspace_select(name: &str) -> Result<(), String> {
    workspace_select_in(Path::new("."), Path::new("state"), name)
}

pub(crate) fn cmd_workspace_delete(name: &str, yes: bool) -> Result<(), String> {
    workspace_delete_in(Path::new("."), Path::new("state"), name, yes)
}

pub(crate) fn cmd_workspace_current() -> Result<(), String> {
    match current_workspace() {
        Some(ws) => println!("{ws}"),
        None => println!("(default — no workspace selected)"),
    }
    Ok(())
}

/// GH-208: a workspace name is a DIRECTORY NAME, never a path.
///
/// `state_base.join(name)` was used unvalidated and fed straight to
/// `remove_dir_all`, so on the published 1.12.3 binary:
///
/// ```text
///   $ forjar workspace delete ../../victim --yes
///   Deleted workspace '../../victim'          rc=0
///   $ ls ../victim
///   *** the entire tree, outside the project, was gone ***
/// ```
///
/// `workspace new ../escaped` likewise wrote state outside `state/`, where
/// `list` could not see it while `current` reported it — the two commands
/// disagreeing about the same workspace. An empty name "succeeded" while
/// creating nothing.
///
/// A name must therefore be a single, ordinary path component. This is checked
/// BEFORE any filesystem access, so a malicious name cannot even be probed for
/// existence.
///
/// `pub(crate)` because `mcp::paths` joins the same marker file onto a state
/// base (#367) and must apply the same check — a second, hand-rolled copy of
/// this predicate is exactly how the first one came to be needed.
pub(crate) fn validate_workspace_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("workspace name must not be empty".to_string());
    }
    if name == "." || name == ".." {
        return Err(format!("invalid workspace name '{name}'"));
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(format!(
            "invalid workspace name '{name}': a workspace name is a directory \
             name, not a path (no '/', '\\' or NUL)"
        ));
    }
    // Belt and braces: reject anything the OS would not treat as one component.
    let mut it = Path::new(name).components();
    match (it.next(), it.next()) {
        (Some(std::path::Component::Normal(_)), None) => Ok(()),
        _ => Err(format!(
            "invalid workspace name '{name}': must be a single path component"
        )),
    }
}

/// Resolve a workspace directory, refusing anything that escapes `state_base`.
///
/// Validation above already rejects separators; this is the second, independent
/// check that the resolved directory really is a direct child of the state base,
/// so a symlink cannot be used to escape either. Destructive operations must go
/// through this, never through a bare `join`.
fn workspace_dir(state_base: &Path, name: &str) -> Result<std::path::PathBuf, String> {
    validate_workspace_name(name)?;
    // NB: a plain join is correct HERE — this function is the one place allowed
    // to build the path; every caller goes through it. (An earlier pass rewrote
    // this line to call workspace_dir() and produced infinite recursion, which
    // surfaced as a stack overflow on `workspace new`.)
    let ws_dir = state_base.join(name);
    if let (Ok(base), Ok(target)) = (state_base.canonicalize(), ws_dir.canonicalize()) {
        if target.parent() != Some(base.as_path()) {
            return Err(format!(
                "refusing to operate on '{}': it is not a direct child of {}",
                ws_dir.display(),
                base.display()
            ));
        }
    }
    Ok(ws_dir)
}

/// Testable core: create workspace in given root + state base.
pub(crate) fn workspace_new_in(root: &Path, state_base: &Path, name: &str) -> Result<(), String> {
    let meta = root.join(".forjar");
    std::fs::create_dir_all(&meta).map_err(|e| format!("cannot create workspace metadata: {e}"))?;
    let ws_dir = workspace_dir(state_base, name)?;
    if ws_dir.exists() {
        return Err(format!("workspace '{name}' already exists"));
    }
    std::fs::create_dir_all(&ws_dir)
        .map_err(|e| format!("cannot create workspace dir {}: {}", ws_dir.display(), e))?;
    std::fs::write(meta.join("workspace"), name)
        .map_err(|e| format!("cannot write workspace file: {e}"))?;
    println!("Created and selected workspace '{name}'");
    Ok(())
}

/// One workspace directory under the state base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceEntry {
    /// Directory name.
    pub(crate) name: String,
    /// Whether `.forjar/workspace` names this one.
    pub(crate) active: bool,
}

/// Enumerate the workspaces under `state_base`, sorted by name.
///
/// FVS (#356): this was inlined in `workspace_list_in`, which printed as it
/// went — so the workspace an agent's state reads and writes through was
/// knowable only by a human looking at a terminal. Sorting is part of the
/// extraction rather than incidental to it: `read_dir` returns entries in no
/// defined order, so the previous listing could reorder itself between two
/// invocations over an unchanged directory.
///
/// A missing state base is an empty list, not an error: it means nothing has
/// been applied yet, which is a fact rather than a failure.
pub(crate) fn list_workspaces_in(
    root: &Path,
    state_base: &Path,
) -> Result<Vec<WorkspaceEntry>, String> {
    let active = current_workspace_in(root);
    if !state_base.exists() {
        return Ok(Vec::new());
    }
    let entries =
        std::fs::read_dir(state_base).map_err(|e| format!("cannot read state dir: {e}"))?;
    let mut out: Vec<WorkspaceEntry> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let is_active = active.as_deref() == Some(name.as_str());
            WorkspaceEntry {
                name,
                active: is_active,
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Testable core: list workspaces.
pub(crate) fn workspace_list_in(root: &Path, state_base: &Path) -> Result<(), String> {
    if !state_base.exists() {
        println!("No workspaces (state/ does not exist)");
        return Ok(());
    }
    let found = list_workspaces_in(root, state_base)?;
    if found.is_empty() {
        println!("No workspaces found");
        return Ok(());
    }
    for ws in &found {
        let marker = if ws.active { " *" } else { "" };
        println!("  {}{}", ws.name, marker);
    }
    Ok(())
}

/// Testable core: select workspace.
pub(crate) fn workspace_select_in(
    root: &Path,
    state_base: &Path,
    name: &str,
) -> Result<(), String> {
    let ws_dir = workspace_dir(state_base, name)?;
    if !ws_dir.exists() {
        return Err(format!(
            "workspace '{name}' does not exist (no state/{name}/)"
        ));
    }
    let meta = root.join(".forjar");
    std::fs::create_dir_all(&meta).map_err(|e| format!("cannot create workspace metadata: {e}"))?;
    std::fs::write(meta.join("workspace"), name)
        .map_err(|e| format!("cannot write workspace file: {e}"))?;
    println!("Selected workspace '{name}'");
    Ok(())
}

/// Testable core: delete workspace.
pub(crate) fn workspace_delete_in(
    root: &Path,
    state_base: &Path,
    name: &str,
    yes: bool,
) -> Result<(), String> {
    let ws_dir = workspace_dir(state_base, name)?;
    if !ws_dir.exists() {
        return Err(format!("workspace '{name}' does not exist"));
    }
    if !yes {
        println!("This will delete workspace '{name}' and all its state. Use --yes to confirm.");
        return Ok(());
    }
    std::fs::remove_dir_all(&ws_dir).map_err(|e| format!("cannot delete workspace dir: {e}"))?;
    if current_workspace_in(root).as_deref() == Some(name) {
        let _ = std::fs::remove_file(root.join(".forjar").join("workspace"));
    }
    println!("Deleted workspace '{name}'");
    Ok(())
}

/// Read the current workspace from `.forjar/workspace` in the given root.
pub(crate) fn current_workspace_in(root: &Path) -> Option<String> {
    std::fs::read_to_string(root.join(".forjar").join("workspace"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read the current workspace from the current directory.
pub(crate) fn current_workspace() -> Option<String> {
    current_workspace_in(Path::new("."))
}

/// Resolve the effective state directory given a workspace flag.
pub(crate) fn resolve_state_dir(state_dir: &Path, workspace_flag: Option<&str>) -> PathBuf {
    if let Some(ws) = workspace_flag {
        return state_dir.join(ws);
    }
    if let Some(ws) = current_workspace() {
        return state_dir.join(ws);
    }
    state_dir.to_path_buf()
}

/// Inject `{{workspace}}` template variable into config params.
///
/// A user-defined `params.workspace` wins and is left untouched. Overwriting it
/// silently corrupted configs: a `workspace: /home/noah/workspace` param was
/// replaced by the workspace *name*, so `{{params.workspace}}` expanded to the
/// `--workspace` value (or `"default"`) and the resource still reported
/// `converged` — a file resource created `~/yoga` instead of `~/workspace`.
pub(crate) fn inject_workspace_param(
    config: &mut types::ForjarConfig,
    workspace_flag: Option<&str>,
) {
    if config.params.contains_key("workspace") {
        eprintln!(
            "warning: config defines param 'workspace' — keeping it; \
             the built-in {{{{workspace}}}} name is not injected"
        );
        return;
    }
    let ws = workspace_flag
        .map(|s| s.to_string())
        .or_else(current_workspace)
        .unwrap_or_else(|| "default".to_string());
    config
        .params
        .insert("workspace".to_string(), serde_yaml_ng::Value::String(ws));
}

#[cfg(test)]
mod tests_gh208_name_validation {
    use super::*;

    // GH-208: `workspace delete ../../victim --yes` recursively deleted a
    // directory OUTSIDE the project and reported success (rc=0) on the published
    // 1.12.3 binary. A workspace name is a directory name, never a path.

    #[test]
    fn traversal_names_are_rejected() {
        for bad in ["../victim", "../../victim", "a/b", "/etc", ".", "..", ""] {
            assert!(
                validate_workspace_name(bad).is_err(),
                "workspace name {bad:?} must be rejected — it can escape the state dir"
            );
        }
    }

    #[test]
    fn ordinary_names_are_accepted() {
        for ok in ["dev", "staging", "ws-1", "ws_1", "a.b", "PROD"] {
            assert!(
                validate_workspace_name(ok).is_ok(),
                "ordinary workspace name {ok:?} must still be accepted"
            );
        }
    }

    #[test]
    fn delete_refuses_to_escape_and_leaves_the_target_alone() {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let victim = root.path().join("victim");
        std::fs::create_dir_all(victim.join("deep")).unwrap();
        std::fs::write(victim.join("deep").join("data.txt"), "SECRET").unwrap();

        let err = workspace_delete_in(root.path(), &state, "../victim", true)
            .expect_err("must refuse to delete outside the state dir");
        assert!(err.contains("invalid workspace name"), "got: {err}");
        assert!(
            victim.join("deep").join("data.txt").exists(),
            "the directory outside the project must be untouched"
        );
    }

    #[test]
    fn delete_still_removes_a_real_workspace() {
        // The guard against "fixed" meaning "deletes nothing".
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        std::fs::create_dir_all(state.join("dev")).unwrap();
        std::fs::write(state.join("dev").join("x"), "y").unwrap();

        workspace_delete_in(root.path(), &state, "dev", true).expect("deletes a real workspace");
        assert!(
            !state.join("dev").exists(),
            "a legitimate workspace must still be deletable"
        );
    }
}
