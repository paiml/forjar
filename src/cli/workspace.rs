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
        Some(ws) => println!("{}", ws),
        None => println!("(default — no workspace selected)"),
    }
    Ok(())
}

/// Testable core: create workspace in given root + state base.
pub(crate) fn workspace_new_in(root: &Path, state_base: &Path, name: &str) -> Result<(), String> {
    let meta = root.join(".forjar");
    std::fs::create_dir_all(&meta)
        .map_err(|e| format!("cannot create workspace metadata: {}", e))?;
    let ws_dir = state_base.join(name);
    if ws_dir.exists() {
        return Err(format!("workspace '{}' already exists", name));
    }
    std::fs::create_dir_all(&ws_dir)
        .map_err(|e| format!("cannot create workspace dir {}: {}", ws_dir.display(), e))?;
    std::fs::write(meta.join("workspace"), name)
        .map_err(|e| format!("cannot write workspace file: {}", e))?;
    println!("Created and selected workspace '{}'", name);
    Ok(())
}

/// Testable core: list workspaces.
pub(crate) fn workspace_list_in(root: &Path, state_base: &Path) -> Result<(), String> {
    let active = current_workspace_in(root);
    if !state_base.exists() {
        println!("No workspaces (state/ does not exist)");
        return Ok(());
    }
    let mut found = false;
    let entries =
        std::fs::read_dir(state_base).map_err(|e| format!("cannot read state dir: {}", e))?;
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            let marker = if active.as_deref() == Some(&name) {
                " *"
            } else {
                ""
            };
            println!("  {}{}", name, marker);
            found = true;
        }
    }
    if !found {
        println!("No workspaces found");
    }
    Ok(())
}

/// Testable core: select workspace.
pub(crate) fn workspace_select_in(
    root: &Path,
    state_base: &Path,
    name: &str,
) -> Result<(), String> {
    let ws_dir = state_base.join(name);
    if !ws_dir.exists() {
        return Err(format!(
            "workspace '{}' does not exist (no state/{}/)",
            name, name
        ));
    }
    let meta = root.join(".forjar");
    std::fs::create_dir_all(&meta)
        .map_err(|e| format!("cannot create workspace metadata: {}", e))?;
    std::fs::write(meta.join("workspace"), name)
        .map_err(|e| format!("cannot write workspace file: {}", e))?;
    println!("Selected workspace '{}'", name);
    Ok(())
}

/// Testable core: delete workspace.
pub(crate) fn workspace_delete_in(
    root: &Path,
    state_base: &Path,
    name: &str,
    yes: bool,
) -> Result<(), String> {
    let ws_dir = state_base.join(name);
    if !ws_dir.exists() {
        return Err(format!("workspace '{}' does not exist", name));
    }
    if !yes {
        println!(
            "This will delete workspace '{}' and all its state. Use --yes to confirm.",
            name
        );
        return Ok(());
    }
    std::fs::remove_dir_all(&ws_dir).map_err(|e| format!("cannot delete workspace dir: {}", e))?;
    if current_workspace_in(root).as_deref() == Some(name) {
        let _ = std::fs::remove_file(root.join(".forjar").join("workspace"));
    }
    println!("Deleted workspace '{}'", name);
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
pub(crate) fn inject_workspace_param(
    config: &mut types::ForjarConfig,
    workspace_flag: Option<&str>,
) {
    let ws = workspace_flag
        .map(|s| s.to_string())
        .or_else(current_workspace)
        .unwrap_or_else(|| "default".to_string());
    config
        .params
        .insert("workspace".to_string(), serde_yaml_ng::Value::String(ws));
}
