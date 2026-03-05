//! FJ-2003/FJ-2005: Active undo and undo-destroy commands.

use super::apply::*;
use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// Print generation metadata summary.
fn print_undo_meta(meta: &types::GenerationMeta) {
    println!("  target created: {}", meta.created_at);
    println!("  target action: {}", meta.action);
    if let Some(ref gr) = meta.git_ref {
        println!("  target git ref: {gr}");
    }
}

/// Compute resource diff for a single machine between current and target locks.
fn diff_machine_locks(
    machine: &str,
    current_lock: Option<&types::StateLock>,
    target_lock: &types::StateLock,
) -> Vec<String> {
    let mut changes = Vec::new();
    for (rid, rl) in &target_lock.resources {
        match current_lock.and_then(|l| l.resources.get(rid)) {
            None => changes.push(format!("  + {rid} ({machine}): will be created")),
            Some(crl) if crl.hash != rl.hash => {
                changes.push(format!("  ~ {rid} ({machine}): will be updated"))
            }
            _ => {}
        }
    }
    if let Some(cl) = current_lock {
        for rid in cl.resources.keys().filter(|r| !target_lock.resources.contains_key(*r)) {
            changes.push(format!("  - {rid} ({machine}): will be destroyed"));
        }
    }
    changes
}

/// Compute resource diff between current locks and target generation locks.
fn compute_undo_diff(
    current_locks: &std::collections::HashMap<String, types::StateLock>,
    target_locks: &std::collections::HashMap<String, types::StateLock>,
) -> Vec<String> {
    target_locks.iter().flat_map(|(machine, target_lock)| {
        diff_machine_locks(machine, current_locks.get(machine), target_lock)
    }).collect()
}

/// FJ-2003: Active undo — revert to a previous generation by re-applying its config.
pub(crate) fn cmd_undo(
    file: &Path,
    state_dir: &Path,
    generations: u32,
    machine_filter: Option<&str>,
    dry_run: bool,
    yes: bool,
) -> Result<(), String> {
    let gen_dir = state_dir.join("generations");
    let current = super::generation::current_generation(&gen_dir)
        .ok_or("no generations found — run `forjar apply` first")?;

    if current < generations {
        return Err(format!("cannot undo {generations} generation(s): only {current} exist"));
    }
    let target = current - generations;

    let current_config = parse_and_validate(file)?;
    let target_gen_dir = gen_dir.join(target.to_string());
    if !target_gen_dir.exists() {
        return Err(format!("generation {target} does not exist"));
    }

    let meta_content = std::fs::read_to_string(target_gen_dir.join(".generation.yaml"))
        .unwrap_or_default();
    println!("Undo: generation {current} → {target}");
    if let Ok(meta) = types::GenerationMeta::from_yaml(&meta_content) {
        print_undo_meta(&meta);
    }

    let current_locks = super::helpers_state::load_machine_locks(
        &current_config, state_dir, machine_filter,
    ).unwrap_or_default();
    let target_locks = load_generation_locks(&target_gen_dir, machine_filter);
    let changes = compute_undo_diff(&current_locks, &target_locks);

    if changes.is_empty() {
        println!("\nNo changes between generation {current} and {target}.");
        return Ok(());
    }
    println!("\nChanges ({} resource(s)):", changes.len());
    for c in &changes { println!("{c}"); }

    if dry_run {
        println!("\nDry run: {} change(s) would be applied.", changes.len());
        return Ok(());
    }
    if !yes {
        return Err("undo requires --yes to confirm".to_string());
    }

    super::generation::rollback_to_generation(state_dir, target, true)?;
    println!("\nRe-applying config to converge to generation {target}...");
    cmd_apply(
        file, state_dir, machine_filter, None, None, None,
        true, false, false, &[], false, None, false, false,
        None, None, false, false, None, false, false, 0, true,
        false, None, false, None, None, None, false, None, false,
    )
}

/// FJ-2005: Undo-destroy — replay from destroy-log.jsonl.
pub(crate) fn cmd_undo_destroy(
    state_dir: &Path,
    machine_filter: Option<&str>,
    force: bool,
    dry_run: bool,
) -> Result<(), String> {
    let log_path = state_dir.join("destroy-log.jsonl");
    let content = std::fs::read_to_string(&log_path)
        .map_err(|_| "no destroy-log.jsonl found — nothing to undo")?;

    let entries: Vec<types::DestroyLogEntry> = content.lines()
        .filter_map(|line| types::DestroyLogEntry::from_jsonl(line).ok())
        .filter(|e| machine_filter.is_none_or(|m| e.machine == m))
        .collect();

    if entries.is_empty() {
        return Err("no matching entries in destroy-log.jsonl".to_string());
    }

    let reliable: Vec<_> = entries.iter().filter(|e| e.reliable_recreate).collect();
    let unreliable: Vec<_> = entries.iter().filter(|e| !e.reliable_recreate).collect();

    println!("Undo-destroy: {} entries ({} reliable, {} best-effort)",
        entries.len(), reliable.len(), unreliable.len());

    for e in &reliable {
        println!("  + {} ({}, {})", e.resource_id, e.resource_type, e.machine);
    }
    for e in &unreliable {
        let marker = if force { "+" } else { "?" };
        println!("  {marker} {} ({}, {}) — unreliable recreate", e.resource_id, e.resource_type, e.machine);
    }

    if !unreliable.is_empty() && !force {
        println!("\n{} unreliable resources skipped. Use --force to attempt.", unreliable.len());
    }

    if dry_run {
        let count = if force { entries.len() } else { reliable.len() };
        println!("\nDry run: {count} resource(s) would be recreated.");
        return Ok(());
    }

    println!("\nReplay not yet implemented — use `forjar apply` with the original config.");
    Ok(())
}

/// Load lock files from a generation directory.
fn load_generation_locks(
    gen_dir: &Path,
    machine_filter: Option<&str>,
) -> std::collections::HashMap<String, types::StateLock> {
    let mut locks = std::collections::HashMap::new();
    let Ok(entries) = std::fs::read_dir(gen_dir) else { return locks };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        if let Some(filter) = machine_filter {
            if name != filter { continue; }
        }
        let lock_path = entry.path().join("state.lock.yaml");
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
                locks.insert(name, lock);
            }
        }
    }
    locks
}
