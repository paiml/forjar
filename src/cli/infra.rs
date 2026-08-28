//! Infrastructure utilities.

use super::helpers::*;
use super::helpers_state::*;
pub(crate) use super::infra_bench::cmd_bench;
use crate::core::{migrate, types};
use std::path::Path;

pub(crate) fn cmd_migrate(file: &Path, output: Option<&Path>) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Count docker resources
    let docker_count = config
        .resources
        .values()
        .filter(|r| r.resource_type == types::ResourceType::Docker)
        .count();

    if docker_count == 0 {
        println!("No Docker resources found in {}", file.display());
        return Ok(());
    }

    let (migrated, warnings) = migrate::migrate_config(&config);

    // Print warnings
    if !warnings.is_empty() {
        eprintln!("Migration warnings:");
        for w in &warnings {
            eprintln!("  ⚠ {w}");
        }
        eprintln!();
    }

    // Serialize migrated config
    let yaml = serde_yaml_ng::to_string(&migrated)
        .map_err(|e| format!("Failed to serialize migrated config: {e}"))?;

    if let Some(out_path) = output {
        std::fs::write(out_path, &yaml)
            .map_err(|e| format!("Failed to write {}: {}", out_path.display(), e))?;
        println!(
            "Migrated {} Docker resource(s) → pepita in {}",
            docker_count,
            out_path.display()
        );
    } else {
        print!("{yaml}");
    }

    println!(
        "Migration complete: {} resource(s) converted, {} warning(s)",
        docker_count,
        warnings.len()
    );
    Ok(())
}

pub(crate) fn cmd_mcp() -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create tokio runtime: {e}"))?;
    rt.block_on(crate::mcp::serve())
}

pub(crate) fn cmd_mcp_schema() -> Result<(), String> {
    let schema = crate::mcp::export_schema();
    let json = serde_json::to_string_pretty(&schema).map_err(|e| format!("JSON error: {e}"))?;
    println!("{json}");
    Ok(())
}

/// Locks of the machines under `state_dir` that pass the optional filter, in
/// listing order. A machine whose lock is missing or unreadable is skipped:
/// every `state` sub-command treats that as "nothing recorded here" rather than
/// as an error.
fn filtered_machine_locks(
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<Vec<types::StateLock>, String> {
    let mut locks = Vec::new();
    for machine_name in list_state_machines(state_dir)? {
        if machine_filter.is_some_and(|filter| machine_name != filter) {
            continue;
        }
        if let Ok(Some(lock)) = crate::core::state::load_lock(state_dir, &machine_name) {
            locks.push(lock);
        }
    }
    Ok(locks)
}

/// Appends one row per resource recorded in `lock`, in lock order.
fn push_state_rows(lock: &types::StateLock, rows: &mut Vec<serde_json::Value>) {
    for (res_id, res_lock) in &lock.resources {
        rows.push(serde_json::json!({
            "machine": lock.machine,
            "resource": res_id,
            "type": res_lock.resource_type.to_string(),
            "status": format!("{:?}", res_lock.status).to_lowercase(),
            "hash": &res_lock.hash[..12.min(res_lock.hash.len())],
            "applied_at": res_lock.applied_at.as_deref().unwrap_or("-"),
        }));
    }
}

/// Prints the collected rows as an aligned table with a machine-count footer.
fn print_state_table(rows: &[serde_json::Value]) {
    println!(
        "{:<15} {:<25} {:<10} {:<10} {:<14} APPLIED AT",
        "MACHINE", "RESOURCE", "TYPE", "STATUS", "HASH"
    );
    for row in rows {
        println!(
            "{:<15} {:<25} {:<10} {:<10} {:<14} {}",
            row["machine"].as_str().unwrap_or("-"),
            row["resource"].as_str().unwrap_or("-"),
            row["type"].as_str().unwrap_or("-"),
            row["status"].as_str().unwrap_or("-"),
            row["hash"].as_str().unwrap_or("-"),
            row["applied_at"].as_str().unwrap_or("-"),
        );
    }
    println!(
        "\n{} resources across {} machines.",
        rows.len(),
        rows.iter()
            .map(|r| r["machine"].as_str().unwrap_or(""))
            .collect::<std::collections::HashSet<_>>()
            .len()
    );
}

/// Renders the collected rows as JSON, as a table, or as the "nothing
/// recorded" line.
fn print_state_rows(rows: &[serde_json::Value], json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(rows).unwrap_or_else(|_| "[]".to_string())
        );
    } else if rows.is_empty() {
        println!("No resources in state.");
    } else {
        print_state_table(rows);
    }
}

pub(crate) fn cmd_state_list(
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    if !state_dir.exists() {
        if json {
            println!("[]");
        } else {
            println!("No state directory found.");
        }
        return Ok(());
    }

    let mut all_rows: Vec<serde_json::Value> = Vec::new();
    for lock in filtered_machine_locks(state_dir, machine_filter)? {
        push_state_rows(&lock, &mut all_rows);
    }

    print_state_rows(&all_rows, json);
    Ok(())
}

/// Renames `old_id` to `new_id` in one machine's lock and saves it, reporting
/// whether this machine recorded `old_id` at all. Refuses when `new_id` is
/// already recorded here, since that would silently drop an entry.
fn rename_in_machine_lock(
    state_dir: &Path,
    lock: &mut types::StateLock,
    old_id: &str,
    new_id: &str,
) -> Result<bool, String> {
    if !lock.resources.contains_key(old_id) {
        return Ok(false);
    }

    if lock.resources.contains_key(new_id) {
        return Err(format!(
            "resource '{}' already exists on machine '{}'",
            new_id, lock.machine
        ));
    }

    // Move the resource entry
    if let Some(resource_lock) = lock.resources.swap_remove(old_id) {
        lock.resources.insert(new_id.to_string(), resource_lock);
    }

    crate::core::state::save_lock(state_dir, lock)
        .map_err(|e| format!("failed to save lock: {e}"))?;

    println!(
        "Renamed '{}' → '{}' on machine '{}'",
        old_id, new_id, lock.machine
    );
    Ok(true)
}

pub(crate) fn cmd_state_mv(
    state_dir: &Path,
    old_id: &str,
    new_id: &str,
    machine_filter: Option<&str>,
) -> Result<(), String> {
    if old_id == new_id {
        return Err("old and new resource IDs are the same".to_string());
    }

    if !state_dir.exists() {
        return Err("state directory does not exist".to_string());
    }

    let mut moved = false;
    for mut lock in filtered_machine_locks(state_dir, machine_filter)? {
        moved |= rename_in_machine_lock(state_dir, &mut lock, old_id, new_id)?;
    }

    if !moved {
        return Err(format!("resource '{old_id}' not found in state"));
    }

    Ok(())
}

// ============================================================================
// FJ-213: state-rm — remove a resource from state
// ============================================================================

/// Other resources in `lock` whose recorded details mention `resource_id`.
/// This is a textual, best-effort guard: state details are untyped, so the only
/// evidence of a reference is the id appearing in a string value.
fn state_dependents_of(lock: &types::StateLock, resource_id: &str) -> Vec<String> {
    lock.resources
        .keys()
        .filter(|k| *k != resource_id)
        .filter(|k| {
            lock.resources[*k]
                .details
                .values()
                .any(|v| v.as_str().map(|s| s.contains(resource_id)).unwrap_or(false))
        })
        .cloned()
        .collect()
}

/// Drops `resource_id` from one machine's lock and saves it, reporting whether
/// this machine recorded it at all. Without `force`, refuses while other
/// entries still appear to reference it.
fn remove_from_machine_lock(
    state_dir: &Path,
    lock: &mut types::StateLock,
    resource_id: &str,
    force: bool,
) -> Result<bool, String> {
    if !lock.resources.contains_key(resource_id) {
        return Ok(false);
    }

    if !force {
        let dependents = state_dependents_of(lock, resource_id);
        if !dependents.is_empty() {
            return Err(format!(
                "resource '{}' may be referenced by: {}. Use --force to skip this check.",
                resource_id,
                dependents.join(", ")
            ));
        }
    }

    lock.resources.swap_remove(resource_id);

    crate::core::state::save_lock(state_dir, lock)
        .map_err(|e| format!("failed to save lock: {e}"))?;

    println!(
        "Removed '{}' from state on machine '{}' (resource still exists on machine)",
        resource_id, lock.machine
    );
    Ok(true)
}

pub(crate) fn cmd_state_rm(
    state_dir: &Path,
    resource_id: &str,
    machine_filter: Option<&str>,
    force: bool,
) -> Result<(), String> {
    if !state_dir.exists() {
        return Err("state directory does not exist".to_string());
    }

    let mut removed = false;
    for mut lock in filtered_machine_locks(state_dir, machine_filter)? {
        removed |= remove_from_machine_lock(state_dir, &mut lock, resource_id, force)?;
    }

    if !removed {
        return Err(format!("resource '{resource_id}' not found in state"));
    }

    Ok(())
}

// ============================================================================
// FJ-215: output — resolve and display output values
// ============================================================================
