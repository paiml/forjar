//! Diff and env commands.

use crate::core::{state, types};
use std::path::Path;
use super::helpers::*;


#[derive(Debug)]
enum DiffChange {
    Added,
    Removed,
    Changed,
}

struct ResourceDiff {
    resource: String,
    change: DiffChange,
    from_hash: Option<String>,
    to_hash: Option<String>,
    from_status: Option<String>,
    to_status: Option<String>,
}

/// Compute diffs between two resource maps for a single machine.
fn compute_machine_diffs(
    from_resources: &indexmap::IndexMap<String, types::ResourceLock>,
    to_resources: &indexmap::IndexMap<String, types::ResourceLock>,
) -> (Vec<ResourceDiff>, usize, usize, usize, usize) {
    let mut diffs = Vec::new();
    let mut added = 0;
    let mut removed = 0;
    let mut changed = 0;
    let mut unchanged = 0;

    for (id, to_res) in to_resources {
        if !from_resources.contains_key(id) {
            diffs.push(ResourceDiff {
                resource: id.clone(),
                change: DiffChange::Added,
                from_hash: None,
                to_hash: Some(to_res.hash.clone()),
                from_status: None,
                to_status: Some(format!("{:?}", to_res.status)),
            });
            added += 1;
        }
    }

    for (id, from_res) in from_resources {
        if !to_resources.contains_key(id) {
            diffs.push(ResourceDiff {
                resource: id.clone(),
                change: DiffChange::Removed,
                from_hash: Some(from_res.hash.clone()),
                to_hash: None,
                from_status: Some(format!("{:?}", from_res.status)),
                to_status: None,
            });
            removed += 1;
        } else if let Some(to_res) = to_resources.get(id) {
            if from_res.hash != to_res.hash || from_res.status != to_res.status {
                diffs.push(ResourceDiff {
                    resource: id.clone(),
                    change: DiffChange::Changed,
                    from_hash: Some(from_res.hash.clone()),
                    to_hash: Some(to_res.hash.clone()),
                    from_status: Some(format!("{:?}", from_res.status)),
                    to_status: Some(format!("{:?}", to_res.status)),
                });
                changed += 1;
            } else {
                unchanged += 1;
            }
        }
    }

    diffs.sort_by(|a, b| a.resource.cmp(&b.resource));
    (diffs, added, removed, changed, unchanged)
}

/// Print a single diff entry in text mode.
fn print_diff_entry(d: &ResourceDiff) {
    let symbol = match d.change {
        DiffChange::Added => "+",
        DiffChange::Removed => "-",
        DiffChange::Changed => "~",
    };
    match d.change {
        DiffChange::Added => {
            println!("  {} {} ({})", symbol, d.resource, d.to_status.as_deref().unwrap_or("?"));
        }
        DiffChange::Removed => {
            println!("  {} {} (was {})", symbol, d.resource, d.from_status.as_deref().unwrap_or("?"));
        }
        DiffChange::Changed => {
            println!(
                "  {} {} ({} → {})", symbol, d.resource,
                d.from_status.as_deref().unwrap_or("?"),
                d.to_status.as_deref().unwrap_or("?")
            );
        }
    }
}

/// Convert diffs to JSON value.
fn diffs_to_json(diffs: &[ResourceDiff]) -> Vec<serde_json::Value> {
    diffs.iter().map(|d| serde_json::json!({
        "resource": d.resource,
        "change": format!("{:?}", d.change).to_lowercase(),
        "from_hash": d.from_hash,
        "to_hash": d.to_hash,
        "from_status": d.from_status,
        "to_status": d.to_status,
    })).collect()
}

pub(crate) fn cmd_diff(
    from: &Path,
    to: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let from_machines = discover_machines(from);
    let to_machines = discover_machines(to);
    let mut all_machines: Vec<String> = from_machines
        .iter().chain(to_machines.iter()).cloned().collect();
    all_machines.sort();
    all_machines.dedup();

    if let Some(filter) = machine_filter {
        all_machines.retain(|m| m == filter);
    }

    if all_machines.is_empty() {
        return Err("no machines found in either state directory".to_string());
    }

    let mut total_added = 0usize;
    let mut total_removed = 0usize;
    let mut total_changed = 0usize;
    let mut total_unchanged = 0usize;
    let mut json_machines = Vec::new();

    for machine_name in &all_machines {
        let from_lock = state::load_lock(from, machine_name)?;
        let to_lock = state::load_lock(to, machine_name)?;

        let from_resources = from_lock.as_ref().map(|l| &l.resources).cloned().unwrap_or_default();
        let to_resources = to_lock.as_ref().map(|l| &l.resources).cloned().unwrap_or_default();

        let (mut diffs, added, removed, changed, unchanged) =
            compute_machine_diffs(&from_resources, &to_resources);

        total_added += added;
        total_removed += removed;
        total_changed += changed;
        total_unchanged += unchanged;

        if let Some(res_filter) = resource_filter {
            diffs.retain(|d| d.resource == res_filter);
        }

        if json {
            json_machines.push(serde_json::json!({
                "machine": machine_name,
                "diffs": diffs_to_json(&diffs),
            }));
        } else if !diffs.is_empty() {
            println!("Machine: {}", machine_name);
            for d in &diffs {
                print_diff_entry(d);
            }
            println!();
        }
    }

    if json {
        let report = serde_json::json!({
            "from": from.display().to_string(),
            "to": to.display().to_string(),
            "summary": {
                "added": total_added, "removed": total_removed,
                "changed": total_changed, "unchanged": total_unchanged,
            },
            "machines": json_machines,
        });
        println!("{}", serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {}", e))?);
    } else {
        println!(
            "Diff: {} added, {} removed, {} changed, {} unchanged",
            total_added, total_removed, total_changed, total_unchanged
        );
    }

    Ok(())
}


/// Load all resource hashes from an environment directory.
fn load_env_resources(dir: &Path) -> Result<std::collections::HashMap<String, String>, String> {
    let mut resources = std::collections::HashMap::new();
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        if entry.path().is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(lock) = state::load_lock(dir, &name)? {
                for (id, rl) in &lock.resources {
                    resources.insert(format!("{}:{}", name, id), rl.hash.clone());
                }
            }
        }
    }
    Ok(resources)
}

/// FJ-367: Compare environments (workspaces).
pub(crate) fn cmd_env_diff(env1: &str, env2: &str, state_dir: &Path, json: bool) -> Result<(), String> {
    let dir1 = state_dir.join(env1);
    let dir2 = state_dir.join(env2);

    if !dir1.exists() {
        return Err(format!("Workspace '{}' not found at {}", env1, dir1.display()));
    }
    if !dir2.exists() {
        return Err(format!("Workspace '{}' not found at {}", env2, dir2.display()));
    }

    let resources1 = load_env_resources(&dir1)?;
    let resources2 = load_env_resources(&dir2)?;

    let keys1: std::collections::HashSet<&String> = resources1.keys().collect();
    let keys2: std::collections::HashSet<&String> = resources2.keys().collect();

    let only1: Vec<&&String> = keys1.difference(&keys2).collect();
    let only2: Vec<&&String> = keys2.difference(&keys1).collect();
    let mut drifted = Vec::new();
    for key in keys1.intersection(&keys2) {
        if resources1[*key] != resources2[*key] {
            drifted.push(*key);
        }
    }

    if json {
        let result = serde_json::json!({
            "only_in_first": only1,
            "only_in_second": only2,
            "drifted": drifted,
        });
        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string()));
    } else {
        println!("Environment diff: {} vs {}\n", bold(env1), bold(env2));
        for k in &only1 { println!("  {} {} (only in {})", red("-"), k, env1); }
        for k in &only2 { println!("  {} {} (only in {})", green("+"), k, env2); }
        for k in &drifted { println!("  {} {} (hash differs)", yellow("~"), k); }
        if only1.is_empty() && only2.is_empty() && drifted.is_empty() {
            println!("  {} Environments are identical.", green("✓"));
        }
    }

    Ok(())
}


/// FJ-277: Show resolved environment info.
pub(crate) fn cmd_env(file: &Path, json: bool) -> Result<(), String> {
    let parsed_config = if file.exists() {
        parse_and_validate(file).ok()
    } else {
        None
    };

    if json {
        print_env_json(file, &parsed_config)
    } else {
        print_env_text(file, &parsed_config);
        Ok(())
    }
}

/// Print environment info as JSON.
fn print_env_json(file: &Path, config: &Option<crate::core::types::ForjarConfig>) -> Result<(), String> {
    let mut info = serde_json::json!({
        "forjar_version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "config_path": file.display().to_string(),
        "config_exists": file.exists(),
    });
    if let Some(ref config) = config {
        info["config_name"] = serde_json::json!(config.name);
        info["machines"] = serde_json::json!(config.machines.len());
        info["resources"] = serde_json::json!(config.resources.len());
        info["params"] = serde_json::json!(config.params.len());
        info["machine_names"] = serde_json::json!(config.machines.keys().collect::<Vec<_>>());
        info["resource_names"] = serde_json::json!(config.resources.keys().collect::<Vec<_>>());
        info["resolved_params"] = serde_json::json!(config.params);
    }
    println!("{}", serde_json::to_string_pretty(&info).map_err(|e| format!("JSON error: {}", e))?);
    Ok(())
}

/// Print environment info as text.
fn print_env_text(file: &Path, config: &Option<crate::core::types::ForjarConfig>) {
    println!("{:<20} {}", bold("forjar version:"), env!("CARGO_PKG_VERSION"));
    println!("{:<20} {}", bold("os:"), std::env::consts::OS);
    println!("{:<20} {}", bold("arch:"), std::env::consts::ARCH);
    println!("{:<20} {}", bold("config:"), file.display());
    if let Some(ref config) = config {
        println!("{:<20} {}", bold("project:"), config.name);
        println!("{:<20} {}", bold("machines:"), config.machines.len());
        println!("{:<20} {}", bold("resources:"), config.resources.len());
        println!("{:<20} {}", bold("params:"), config.params.len());
    } else if !file.exists() {
        println!("{:<20} {}", bold("status:"), dim("config not found"));
    } else {
        println!("{:<20} {}", bold("status:"), red("parse error"));
    }
}
