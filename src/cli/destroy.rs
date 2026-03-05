//! Destroy, rollback, and undo.

use super::apply::*;
use super::helpers::*;
use crate::core::{codegen, executor, resolver, types};
use crate::transport;
use std::path::Path;

/// Destroy a single resource on its machine. Returns true on success.
fn destroy_single_resource(
    resource_id: &str,
    resource: &types::Resource,
    machine: &types::Machine,
) -> bool {
    let mut destroy_resource = resource.clone();
    destroy_resource.state = Some("absent".to_string());

    let script = match codegen::apply_script(&destroy_resource) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  SKIP {resource_id}: codegen error: {e}");
            return false;
        }
    };

    if machine.is_container_transport() {
        let _ = crate::transport::container::ensure_container(machine);
    }

    match transport::exec_script(machine, &script) {
        Ok(out) if out.success() => {
            println!("  - {} ({})", resource_id, resource.resource_type);
            true
        }
        Ok(out) => {
            eprintln!(
                "  FAIL {}: exit {}: {}",
                resource_id,
                out.exit_code,
                out.stderr.trim()
            );
            false
        }
        Err(e) => {
            eprintln!("  FAIL {resource_id}: {e}");
            false
        }
    }
}

/// Clean up state lock files for the given machines.
fn cleanup_state_files(state_dir: &Path, machines: &[String], machine_filter: Option<&str>) {
    for machine_name in machines {
        if let Some(filter) = machine_filter {
            if machine_name != filter {
                continue;
            }
        }
        let lock_path = state_dir.join(machine_name).join("state.lock.yaml");
        if lock_path.exists() {
            let _ = std::fs::remove_file(&lock_path);
        }
    }
}

/// FJ-2005: Remove only succeeded resource entries from lock files on partial failure.
pub(crate) fn cleanup_succeeded_entries(
    state_dir: &Path,
    succeeded: &std::collections::HashMap<String, Vec<String>>,
) {
    for (machine_name, resource_ids) in succeeded {
        let lock_path = state_dir.join(machine_name).join("state.lock.yaml");
        let Ok(content) = std::fs::read_to_string(&lock_path) else {
            continue;
        };
        let Ok(mut lock) =
            serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content)
        else {
            continue;
        };
        for rid in resource_ids {
            lock.resources.shift_remove(rid);
        }
        if lock.resources.is_empty() {
            let _ = std::fs::remove_file(&lock_path);
        } else if let Ok(yaml) = serde_yaml_ng::to_string(&lock) {
            let _ = std::fs::write(&lock_path, yaml);
        }
    }
}

/// FJ-2005: Write a destroy log entry with pre-state for undo-destroy recovery.
pub(crate) fn write_destroy_log_entry(
    log_path: &Path,
    resource_id: &str,
    resource: &types::Resource,
    machine_name: &str,
    locks: &std::collections::HashMap<String, types::StateLock>,
) {
    let pre_hash = locks
        .get(machine_name)
        .and_then(|l| l.resources.get(resource_id))
        .map(|rl| rl.hash.clone())
        .unwrap_or_default();

    let entry = types::DestroyLogEntry {
        timestamp: crate::tripwire::eventlog::now_iso8601(),
        machine: machine_name.to_string(),
        resource_id: resource_id.to_string(),
        resource_type: resource.resource_type.to_string(),
        pre_hash,
        generation: 0, // filled by caller if known
        config_fragment: serde_yaml_ng::to_string(resource).ok(),
        reliable_recreate: resource.content.is_some(),
    };
    if let Ok(line) = entry.to_jsonl() {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

pub(crate) fn cmd_destroy(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    yes: bool,
    verbose: bool,
) -> Result<(), String> {
    if !yes {
        return Err(
            "destroy requires --yes flag to confirm removal of all managed resources".to_string(),
        );
    }

    let config = parse_and_validate(file)?;
    let execution_order = resolver::build_execution_order(&config)?;
    let reverse_order: Vec<String> = execution_order.into_iter().rev().collect();

    if verbose {
        eprintln!(
            "Destroying {} resources in reverse order",
            reverse_order.len()
        );
    }

    let all_machines = executor::collect_machines(&config);
    // FJ-2005: Load locks to capture pre-hash for destroy log
    let locks = super::helpers_state::load_machine_locks(&config, state_dir, machine_filter)
        .unwrap_or_default();
    let destroy_log_path = state_dir.join("destroy-log.jsonl");
    let mut destroyed = 0u32;
    let mut failed = 0u32;
    let mut succeeded_resources: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for resource_id in &reverse_order {
        let resource = match config.resources.get(resource_id) {
            Some(r) => r,
            None => continue,
        };

        let machine_name = match &resource.machine {
            types::MachineTarget::Single(m) => m.as_str(),
            types::MachineTarget::Multiple(ms) => {
                if ms.is_empty() {
                    continue;
                }
                ms[0].as_str()
            }
        };

        if let Some(filter) = machine_filter {
            if machine_name != filter {
                continue;
            }
        }

        let machine = match config.machines.get(machine_name) {
            Some(m) => m,
            None => {
                eprintln!("  SKIP {resource_id}: machine '{machine_name}' not found");
                failed += 1;
                continue;
            }
        };

        if destroy_single_resource(resource_id, resource, machine) {
            destroyed += 1;
            succeeded_resources
                .entry(machine_name.to_string())
                .or_default()
                .push(resource_id.clone());
            // FJ-2005: Write pre-state to destroy-log.jsonl
            write_destroy_log_entry(
                &destroy_log_path,
                resource_id,
                resource,
                machine_name,
                &locks,
            );
        } else {
            failed += 1;
        }
    }

    if failed == 0 {
        // All succeeded — remove entire lock files
        cleanup_state_files(state_dir, &all_machines, machine_filter);
    } else {
        // FJ-2005: Partial failure — only remove lock entries for succeeded resources
        cleanup_succeeded_entries(state_dir, &succeeded_resources);
    }

    println!();
    if failed > 0 {
        println!("Destroy completed with errors: {destroyed} destroyed, {failed} failed");
        return Err(format!("{failed} resource(s) failed to destroy"));
    }

    println!("Destroy complete: {destroyed} resources removed.");
    Ok(())
}

/// Rollback to a previous config revision from git history.
pub(crate) fn cmd_rollback(
    file: &Path,
    state_dir: &Path,
    revision: u32,
    machine_filter: Option<&str>,
    dry_run: bool,
    verbose: bool,
) -> Result<(), String> {
    let file_str = file.to_string_lossy();
    let git_ref = format!("HEAD~{revision}:{file_str}");
    let output = std::process::Command::new("git")
        .args(["show", &git_ref])
        .output()
        .map_err(|e| format!("git show failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "cannot read {} from git history (HEAD~{}): {}",
            file_str,
            revision,
            stderr.trim()
        ));
    }

    let previous_yaml = String::from_utf8_lossy(&output.stdout);
    let previous_config: types::ForjarConfig = serde_yaml_ng::from_str(&previous_yaml)
        .map_err(|e| format!("cannot parse previous config (HEAD~{revision}): {e}"))?;
    let current_config = parse_and_validate(file)?;

    let changes = compute_rollback_changes(&previous_config, &current_config, revision);

    if changes.is_empty() {
        println!("No config changes between HEAD and HEAD~{revision}. Nothing to rollback.");
        return Ok(());
    }

    println!("Rollback to HEAD~{} ({}):", revision, previous_config.name);
    for c in &changes {
        println!("{c}");
    }
    println!();

    if dry_run {
        println!("Dry run: {} change(s) would be applied.", changes.len());
        return Ok(());
    }

    let temp_config = std::env::temp_dir().join("forjar-rollback.yaml");
    std::fs::write(&temp_config, previous_yaml.as_bytes())
        .map_err(|e| format!("cannot write temp config: {e}"))?;

    println!("Applying previous config with --force...");
    cmd_apply(
        &temp_config,
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
        verbose,
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

/// Compare previous and current configs to find rollback changes.
pub(crate) fn compute_rollback_changes(
    previous: &types::ForjarConfig,
    current: &types::ForjarConfig,
    revision: u32,
) -> Vec<String> {
    let mut changes = Vec::new();
    for (id, prev_resource) in &previous.resources {
        if let Some(cur_resource) = current.resources.get(id) {
            let prev_yaml = serde_yaml_ng::to_string(prev_resource).unwrap_or_default();
            let cur_yaml = serde_yaml_ng::to_string(cur_resource).unwrap_or_default();
            if prev_yaml != cur_yaml {
                changes.push(format!("  ~ {id} (modified)"));
            }
        } else {
            changes.push(format!("  + {id} (will be re-added from HEAD~{revision})"));
        }
    }
    for id in current.resources.keys() {
        if !previous.resources.contains_key(id) {
            changes.push(format!(
                "  - {id} (exists now but not in HEAD~{revision}, will remain)"
            ));
        }
    }
    changes
}

/// FJ-2003: Active undo — revert to a previous generation by re-applying its config.
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
