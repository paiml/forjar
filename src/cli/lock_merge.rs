//! Lock merge and rebase.

use super::helpers::*;
use crate::core::state;
use std::path::Path;

// ── FJ-445: lock merge ──

/// Collect all machine directory names from a state directory.
fn collect_machine_names_from_dir(dir: &Path) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    if !dir.exists() {
        return names;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if !name.starts_with('.') {
                    names.insert(name);
                }
            }
        }
    }
    names
}

pub(crate) fn cmd_lock_merge(
    from: &Path,
    to: &Path,
    output: &Path,
    json: bool,
) -> Result<(), String> {
    let mut merged_count = 0usize;
    let mut conflict_count = 0usize;

    if !from.exists() && !to.exists() {
        return Err("Both state directories are empty".to_string());
    }

    let mut machines = collect_machine_names_from_dir(from);
    machines.extend(collect_machine_names_from_dir(to));

    std::fs::create_dir_all(output).map_err(|e| e.to_string())?;

    for m_name in &machines {
        let left = state::load_lock(from, m_name).ok().flatten();
        let right = state::load_lock(to, m_name).ok().flatten();

        match (left, right) {
            (Some(_left_lock), Some(right_lock)) => {
                // Right takes precedence on conflicts
                let out_dir = output.join(m_name);
                std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
                let lock_path = out_dir.join("state.lock.yaml");
                let yaml = serde_yaml_ng::to_string(&right_lock).map_err(|e| e.to_string())?;
                std::fs::write(&lock_path, yaml).map_err(|e| e.to_string())?;
                conflict_count += 1;
                merged_count += 1;
            }
            (Some(lock), None) | (None, Some(lock)) => {
                let out_dir = output.join(m_name);
                std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
                let lock_path = out_dir.join("state.lock.yaml");
                let yaml = serde_yaml_ng::to_string(&lock).map_err(|e| e.to_string())?;
                std::fs::write(&lock_path, yaml).map_err(|e| e.to_string())?;
                merged_count += 1;
            }
            (None, None) => {}
        }
    }

    if json {
        println!(
            "{{\"merged\":{},\"conflicts\":{},\"output\":\"{}\"}}",
            merged_count,
            conflict_count,
            output.display()
        );
    } else {
        println!(
            "{} Merged {} machine(s) ({} conflicts, right takes precedence) → {}",
            green("✓"),
            merged_count,
            conflict_count,
            output.display()
        );
    }
    Ok(())
}

// ── FJ-455: lock rebase ──

pub(crate) fn cmd_lock_rebase(
    from: &Path,
    config_file: &Path,
    output: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(config_file)?;
    let config_resources: std::collections::HashSet<String> =
        config.resources.keys().cloned().collect();

    std::fs::create_dir_all(output).map_err(|e| e.to_string())?;

    let mut kept = 0usize;
    let mut dropped = 0usize;

    if from.exists() {
        let entries = std::fs::read_dir(from).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let m_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if m_name.starts_with('.') {
                    continue;
                }
                if let Ok(Some(mut lock)) = state::load_lock(from, &m_name) {
                    let before = lock.resources.len();
                    lock.resources
                        .retain(|name, _| config_resources.contains(name));
                    let after = lock.resources.len();
                    kept += after;
                    dropped += before - after;

                    let out_dir = output.join(&m_name);
                    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
                    let lock_path = out_dir.join("state.lock.yaml");
                    let yaml = serde_yaml_ng::to_string(&lock).map_err(|e| e.to_string())?;
                    std::fs::write(&lock_path, yaml).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    if json {
        println!(
            "{{\"kept\":{},\"dropped\":{},\"output\":\"{}\"}}",
            kept,
            dropped,
            output.display()
        );
    } else {
        println!(
            "{} Rebased: {} resources kept, {} dropped → {}",
            green("✓"),
            kept,
            dropped,
            output.display()
        );
    }
    Ok(())
}

// ── FJ-465: lock sign ──

pub(crate) fn cmd_lock_sign(state_dir: &Path, key: &str, json: bool) -> Result<(), String> {
    use crate::tripwire::hasher;

    let mut signed = 0usize;
    if state_dir.exists() {
        let entries = std::fs::read_dir(state_dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let m_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if m_name.starts_with('.') {
                    continue;
                }
                let lock_path = path.join("state.lock.yaml");
                if lock_path.exists() {
                    let content = std::fs::read_to_string(&lock_path).map_err(|e| e.to_string())?;
                    let hash = hasher::hash_string(&format!("{content}{key}"));
                    let sig_path = path.join("lock.sig");
                    std::fs::write(&sig_path, &hash).map_err(|e| e.to_string())?;
                    signed += 1;
                }
            }
        }
    }

    if json {
        println!(
            "{{\"signed\":{},\"state_dir\":\"{}\"}}",
            signed,
            state_dir.display()
        );
    } else {
        println!(
            "{} Signed {} lock file(s) in {}",
            green("✓"),
            signed,
            state_dir.display()
        );
    }
    Ok(())
}
