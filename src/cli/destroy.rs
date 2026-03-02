//! Destroy and rollback.

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
            eprintln!("  SKIP {}: codegen error: {}", resource_id, e);
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
            eprintln!("  FAIL {}: {}", resource_id, e);
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
    let mut destroyed = 0u32;
    let mut failed = 0u32;

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
                eprintln!(
                    "  SKIP {}: machine '{}' not found",
                    resource_id, machine_name
                );
                failed += 1;
                continue;
            }
        };

        if destroy_single_resource(resource_id, resource, machine) {
            destroyed += 1;
        } else {
            failed += 1;
        }
    }

    if failed == 0 {
        cleanup_state_files(state_dir, &all_machines, machine_filter);
    }

    println!();
    if failed > 0 {
        println!(
            "Destroy completed with errors: {} destroyed, {} failed",
            destroyed, failed
        );
        return Err(format!("{} resource(s) failed to destroy", failed));
    }

    println!("Destroy complete: {} resources removed.", destroyed);
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
    let git_ref = format!("HEAD~{}:{}", revision, file_str);
    let output = std::process::Command::new("git")
        .args(["show", &git_ref])
        .output()
        .map_err(|e| format!("git show failed: {}", e))?;

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
        .map_err(|e| format!("cannot parse previous config (HEAD~{}): {}", revision, e))?;
    let current_config = parse_and_validate(file)?;

    let changes = compute_rollback_changes(&previous_config, &current_config, revision);

    if changes.is_empty() {
        println!(
            "No config changes between HEAD and HEAD~{}. Nothing to rollback.",
            revision
        );
        return Ok(());
    }

    println!("Rollback to HEAD~{} ({}):", revision, previous_config.name);
    for c in &changes {
        println!("{}", c);
    }
    println!();

    if dry_run {
        println!("Dry run: {} change(s) would be applied.", changes.len());
        return Ok(());
    }

    let temp_config = std::env::temp_dir().join("forjar-rollback.yaml");
    std::fs::write(&temp_config, previous_yaml.as_bytes())
        .map_err(|e| format!("cannot write temp config: {}", e))?;

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
                changes.push(format!("  ~ {} (modified)", id));
            }
        } else {
            changes.push(format!(
                "  + {} (will be re-added from HEAD~{})",
                id, revision
            ));
        }
    }
    for id in current.resources.keys() {
        if !previous.resources.contains_key(id) {
            changes.push(format!(
                "  - {} (exists now but not in HEAD~{}, will remain)",
                id, revision
            ));
        }
    }
    changes
}
