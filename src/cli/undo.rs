//! FJ-2003/FJ-2005: Active undo and undo-destroy commands.

use super::apply::*;
use super::helpers::*;
use super::helpers_state::load_generation_locks;
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
pub(super) fn diff_machine_locks(
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
        for rid in cl
            .resources
            .keys()
            .filter(|r| !target_lock.resources.contains_key(*r))
        {
            changes.push(format!("  - {rid} ({machine}): will be destroyed"));
        }
    }
    changes
}

/// Compute resource diff between current locks and target generation locks.
pub(super) fn compute_undo_diff(
    current_locks: &std::collections::HashMap<String, types::StateLock>,
    target_locks: &std::collections::HashMap<String, types::StateLock>,
) -> Vec<String> {
    target_locks
        .iter()
        .flat_map(|(machine, target_lock)| {
            diff_machine_locks(machine, current_locks.get(machine), target_lock)
        })
        .collect()
}

/// FJ-2003: Pre-flight SSH connectivity check for multi-machine undo.
///
/// Verifies all target machines are reachable before making any changes.
/// Returns Err if any machine is unreachable (fail fast).
fn preflight_ssh_check(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
) -> Result<(), String> {
    let machines: Vec<(&String, &types::Machine)> = config
        .machines
        .iter()
        .filter(|(name, _)| machine_filter.is_none_or(|f| name.as_str() == f))
        .collect();

    let mut unreachable = Vec::new();
    for (name, machine) in &machines {
        let is_local = machine.addr == "localhost"
            || machine.addr == "127.0.0.1"
            || machine.transport.as_deref() == Some("local");
        if is_local || machine.is_container_transport() {
            println!("  ✓ {name}: local/container (skip SSH)");
            continue;
        }
        let host = &machine.addr;
        let status = std::process::Command::new("ssh")
            .args([
                "-o",
                "ConnectTimeout=5",
                "-o",
                "BatchMode=yes",
                host,
                "true",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => println!("  ✓ {name}: {host} reachable"),
            _ => {
                eprintln!("  ✗ {name}: {host} unreachable");
                unreachable.push(name.as_str());
            }
        }
    }
    if unreachable.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "pre-flight failed: {} machine(s) unreachable: {}",
            unreachable.len(),
            unreachable.join(", ")
        ))
    }
}

/// Write undo progress to `undo-progress.yaml` in the machine's state directory.
pub(super) fn write_undo_progress(state_dir: &Path, machine: &str, progress: &types::UndoProgress) {
    let dir = state_dir.join(machine);
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("undo-progress.yaml");
    if let Ok(yaml) = serde_yaml_ng::to_string(progress) {
        let _ = std::fs::write(path, yaml);
    }
}

/// Read undo progress from a machine's state directory.
pub(super) fn read_undo_progress(state_dir: &Path, machine: &str) -> Option<types::UndoProgress> {
    let path = state_dir.join(machine).join("undo-progress.yaml");
    let content = std::fs::read_to_string(path).ok()?;
    serde_yaml_ng::from_str(&content).ok()
}

/// Initialize undo progress for all affected resources.
pub(super) fn init_undo_progress(
    current: u32,
    target: u32,
    changes: &[String],
) -> types::UndoProgress {
    let mut resources = std::collections::HashMap::new();
    for c in changes {
        let rid = c.split_whitespace().nth(1).unwrap_or("unknown");
        resources.insert(
            rid.to_string(),
            types::ResourceProgress {
                status: types::ResourceProgressStatus::Pending,
                at: None,
            },
        );
    }
    types::UndoProgress {
        generation_from: current,
        generation_to: target,
        started_at: crate::tripwire::eventlog::now_iso8601(),
        status: types::UndoStatus::InProgress,
        resources,
    }
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
        return Err(format!(
            "cannot undo {generations} generation(s): only {current} exist"
        ));
    }
    let target = current - generations;

    let current_config = parse_and_validate(file)?;
    let target_gen_dir = gen_dir.join(target.to_string());
    if !target_gen_dir.exists() {
        return Err(format!("generation {target} does not exist"));
    }

    let meta_content =
        std::fs::read_to_string(target_gen_dir.join(".generation.yaml")).unwrap_or_default();
    println!("Undo: generation {current} → {target}");
    if let Ok(meta) = types::GenerationMeta::from_yaml(&meta_content) {
        print_undo_meta(&meta);
    }

    let current_locks =
        super::helpers_state::load_machine_locks(&current_config, state_dir, machine_filter)
            .unwrap_or_default();
    let target_locks = load_generation_locks(&target_gen_dir, machine_filter);
    let changes = compute_undo_diff(&current_locks, &target_locks);

    if changes.is_empty() {
        println!("\nNo changes between generation {current} and {target}.");
        return Ok(());
    }
    println!("\nChanges ({} resource(s)):", changes.len());
    for c in &changes {
        println!("{c}");
    }

    if dry_run {
        println!("\nDry run: {} change(s) would be applied.", changes.len());
        return Ok(());
    }
    if !yes {
        return Err("undo requires --yes to confirm".to_string());
    }

    // Phase 1: Pre-flight SSH check (multi-machine coordination)
    println!("\nPre-flight check:");
    preflight_ssh_check(&current_config, machine_filter)?;

    // Write undo-progress.yaml for resume support
    let progress = init_undo_progress(current, target, &changes);
    for machine in target_locks.keys() {
        write_undo_progress(state_dir, machine, &progress);
    }

    super::generation::rollback_to_generation(state_dir, target, true)?;
    println!("\nRe-applying config to converge to generation {target}...");
    let result = cmd_apply(
        file,
        state_dir,
        machine_filter,
        None,
        None,
        None,
        true,
        false,
        false,
        &[],
        false,
        None,
        false,
        false,
        None,
        None,
        false,
        false,
        None,
        false,
        false,
        0,
        true,
        false,
        None,
        false,
        None,
        None,
        None,
        false,
        None,
        false,
    );

    // Mark progress completed or partial
    let final_status = if result.is_ok() {
        types::UndoStatus::Completed
    } else {
        types::UndoStatus::Partial
    };
    for machine in target_locks.keys() {
        if let Some(mut p) = read_undo_progress(state_dir, machine) {
            p.status = final_status;
            write_undo_progress(state_dir, machine, &p);
        }
    }
    result
}

/// FJ-2003: Resume a partial undo from undo-progress.yaml.
pub(crate) fn cmd_undo_resume(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    dry_run: bool,
    yes: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let machines: Vec<String> = config
        .machines
        .keys()
        .filter(|&m| machine_filter.is_none_or(|f| m == f))
        .cloned()
        .collect();

    let mut found_partial = false;
    for machine in &machines {
        if let Some(p) = read_undo_progress(state_dir, machine) {
            if p.needs_resume() {
                found_partial = true;
                let pending = p.pending_count();
                let failed = p.failed_count();
                let done = p.completed_count();
                println!("Resume {machine}: gen {} → {} ({done} done, {failed} failed, {pending} pending)",
                    p.generation_from, p.generation_to);
            }
        }
    }
    if !found_partial {
        return Err("no partial undo found — nothing to resume".to_string());
    }
    if dry_run {
        println!("\nDry run: would resume partial undo.");
        return Ok(());
    }
    if !yes {
        return Err("undo --resume requires --yes to confirm".to_string());
    }

    println!("\nRe-applying config to complete undo...");
    cmd_apply(
        file,
        state_dir,
        machine_filter,
        None,
        None,
        None,
        true,
        false,
        false,
        &[],
        false,
        None,
        false,
        false,
        None,
        None,
        false,
        false,
        None,
        false,
        false,
        0,
        true,
        false,
        None,
        false,
        None,
        None,
        None,
        false,
        None,
        false,
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

    let entries: Vec<types::DestroyLogEntry> = content
        .lines()
        .filter_map(|line| types::DestroyLogEntry::from_jsonl(line).ok())
        .filter(|e| machine_filter.is_none_or(|m| e.machine == m))
        .collect();

    if entries.is_empty() {
        return Err("no matching entries in destroy-log.jsonl".to_string());
    }

    let reliable: Vec<_> = entries.iter().filter(|e| e.reliable_recreate).collect();
    let unreliable: Vec<_> = entries.iter().filter(|e| !e.reliable_recreate).collect();

    println!(
        "Undo-destroy: {} entries ({} reliable, {} best-effort)",
        entries.len(),
        reliable.len(),
        unreliable.len()
    );

    for e in &reliable {
        println!("  + {} ({}, {})", e.resource_id, e.resource_type, e.machine);
    }
    for e in &unreliable {
        let marker = if force { "+" } else { "?" };
        println!(
            "  {marker} {} ({}, {}) — unreliable recreate",
            e.resource_id, e.resource_type, e.machine
        );
    }

    if !unreliable.is_empty() && !force {
        println!(
            "\n{} unreliable resources skipped. Use --force to attempt.",
            unreliable.len()
        );
    }

    if dry_run {
        let count = if force { entries.len() } else { reliable.len() };
        println!("\nDry run: {count} resource(s) would be recreated.");
        return Ok(());
    }

    // FJ-2005: Replay — reconstruct resources from config_fragment and converge
    let replay_set: Vec<&types::DestroyLogEntry> = if force {
        entries.iter().collect()
    } else {
        reliable.clone()
    };

    let mut replayed = 0u32;
    let mut failed = 0u32;
    for entry in &replay_set {
        let Some(ref fragment) = entry.config_fragment else {
            eprintln!(
                "  SKIP {}: no config_fragment in destroy log",
                entry.resource_id
            );
            failed += 1;
            continue;
        };
        let resource: types::Resource = match serde_yaml_ng::from_str(fragment) {
            Ok(r) => r,
            Err(e) => {
                eprintln!(
                    "  SKIP {}: cannot parse config_fragment: {e}",
                    entry.resource_id
                );
                failed += 1;
                continue;
            }
        };

        let machine_name = &entry.machine;
        let machine_config = format!(
            "version: '1.0'\nname: undo-destroy-replay\nmachines:\n  {machine_name}:\n    hostname: {machine_name}\n    addr: 127.0.0.1\nresources: {{}}\n"
        );
        let mut config: types::ForjarConfig =
            match crate::core::parser::parse_config(&machine_config) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("  SKIP {}: config error: {e}", entry.resource_id);
                    failed += 1;
                    continue;
                }
            };
        config.resources.insert(entry.resource_id.clone(), resource);

        let script = match crate::core::codegen::apply_script(
            config.resources.get(&entry.resource_id).unwrap(),
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  FAIL {}: codegen error: {e}", entry.resource_id);
                failed += 1;
                continue;
            }
        };

        let Some(machine) = config.machines.get(machine_name) else {
            eprintln!(
                "  SKIP {}: machine '{machine_name}' not in config",
                entry.resource_id
            );
            failed += 1;
            continue;
        };

        match crate::transport::exec_script(machine, &script) {
            Ok(out) if out.success() => {
                println!("  + {} ({})", entry.resource_id, entry.resource_type);
                replayed += 1;
            }
            Ok(out) => {
                eprintln!(
                    "  FAIL {}: exit {}: {}",
                    entry.resource_id,
                    out.exit_code,
                    out.stderr.trim()
                );
                failed += 1;
            }
            Err(e) => {
                eprintln!("  FAIL {}: {e}", entry.resource_id);
                failed += 1;
            }
        }
    }

    println!("\nUndo-destroy: {replayed} replayed, {failed} failed.");
    if failed > 0 {
        Err(format!("{failed} resource(s) failed to recreate"))
    } else {
        Ok(())
    }
}
