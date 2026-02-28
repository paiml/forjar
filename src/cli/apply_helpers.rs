//! Apply lifecycle helpers (hooks, notify, params, git).

use crate::core::types;
use std::path::Path;


/// Run a local shell hook command. Returns Ok if the command succeeds, Err if it fails.
pub(crate) fn run_hook(name: &str, command: &str, verbose: bool) -> Result<(), String> {
    if verbose {
        eprintln!("Running {} hook: {}", name, command);
    }
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| format!("{} hook failed to start: {}", name, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{} hook failed (exit {}): {}",
            name,
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        print!("{}", stdout);
    }
    Ok(())
}


/// FJ-225: Run a notification hook with template variable expansion.
pub(crate) fn run_notify(template: &str, vars: &[(&str, &str)]) {
    let mut cmd = template.to_string();
    for (key, value) in vars {
        cmd = cmd.replace(&format!("{{{{{}}}}}", key), value);
    }
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if !stdout.is_empty() {
                print!("{}", stdout);
            }
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                eprintln!(
                    "Warning: notify hook exited {}: {}",
                    out.status.code().unwrap_or(-1),
                    stderr.trim()
                );
            }
        }
        Err(e) => {
            eprintln!("Warning: notify hook failed to start: {}", e);
        }
    }
}


/// Parse KEY=VALUE param overrides and merge into config.
pub(crate) fn apply_param_overrides(
    config: &mut types::ForjarConfig,
    overrides: &[String],
) -> Result<(), String> {
    for kv in overrides {
        let (key, value) = kv
            .split_once('=')
            .ok_or_else(|| format!("invalid param '{}': expected KEY=VALUE", kv))?;
        config.params.insert(
            key.to_string(),
            serde_yaml_ng::Value::String(value.to_string()),
        );
    }
    Ok(())
}

// ========================================================================
// FJ-210: Workspace helpers
// ========================================================================


/// FJ-211: Load param overrides from an external YAML file.
/// The file must be a flat YAML mapping (key: value). Values are merged into
/// config.params, overriding any existing keys with the same name.
pub(crate) fn load_env_params(config: &mut types::ForjarConfig, path: &Path) -> Result<(), String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read env file {}: {}", path.display(), e))?;
    let mapping: indexmap::IndexMap<String, serde_yaml_ng::Value> =
        serde_yaml_ng::from_str(&content)
            .map_err(|e| format!("invalid YAML in env file {}: {}", path.display(), e))?;
    for (key, value) in mapping {
        config.params.insert(key, value);
    }
    Ok(())
}


/// Git commit state directory after successful apply.
pub(crate) fn git_commit_state(state_dir: &Path, config_name: &str, converged: u32) -> Result<(), String> {
    let msg = format!(
        "forjar: {} — {} resource(s) converged",
        config_name, converged
    );
    // Find the git repo root from state_dir's parent
    let repo_root = state_dir.parent().unwrap_or(Path::new("."));
    let status = std::process::Command::new("git")
        .current_dir(repo_root)
        .args(["add", "state"])
        .status()
        .map_err(|e| format!("git add failed: {}", e))?;
    if !status.success() {
        return Err("git add state/ failed".to_string());
    }
    let status = std::process::Command::new("git")
        .current_dir(repo_root)
        .args(["commit", "--no-verify", "-m", &msg])
        .status()
        .map_err(|e| format!("git commit failed: {}", e))?;
    if !status.success() {
        return Err("git commit failed".to_string());
    }
    println!("Auto-committed state: {}", msg);
    Ok(())
}

