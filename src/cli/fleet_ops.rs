//! Fleet operations.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::apply::*;


/// FJ-327: Re-run only previously failed resources.
pub(crate) fn cmd_retry_failed(
    file: &Path,
    state_dir: &Path,
    param_overrides: &[String],
    timeout: Option<u64>,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Scan event logs for the most recent ResourceFailed events per machine
    let mut failed_resources: Vec<(String, String)> = Vec::new(); // (machine, resource)

    for (name, _machine) in &config.machines {
        let log_path = eventlog::event_log_path(state_dir, name);
        if !log_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&log_path)
            .map_err(|e| format!("cannot read {}: {}", log_path.display(), e))?;

        // Find the last ApplyCompleted to mark the boundary, then collect
        // ResourceFailed events after the last ApplyCompleted
        let mut last_apply_line = 0usize;
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains("ApplyCompleted") {
                last_apply_line = i;
            }
        }

        // Collect ResourceFailed events from the last apply run
        // We scan backwards from the last ApplyCompleted to find ResourceFailed in that run
        for line in &lines[..=last_apply_line] {
            if let Ok(event) = serde_json::from_str::<types::TimestampedEvent>(line) {
                if let types::ProvenanceEvent::ResourceFailed {
                    ref machine,
                    ref resource,
                    ..
                } = event.event
                {
                    // Check if this is from the most recent run (same machine)
                    if machine == name {
                        failed_resources.push((name.clone(), resource.clone()));
                    }
                }
            }
        }
    }

    if failed_resources.is_empty() {
        println!("No failed resources found in event logs. Nothing to retry.");
        return Ok(());
    }

    println!("Retrying {} failed resource(s):", failed_resources.len());
    for (machine, resource) in &failed_resources {
        println!("  {} → {}", machine, resource);
    }
    println!();

    // Apply each failed resource individually
    for (machine, resource) in &failed_resources {
        println!("Retrying {} on {}...", resource, machine);
        cmd_apply(
            file,
            state_dir,
            Some(machine),
            Some(resource),
            None,  // tag_filter
            None,  // group_filter
            true,  // force — re-apply regardless of hash
            false, // dry_run
            false, // no_tripwire
            param_overrides,
            false, // auto_commit
            timeout,
            false, // json
            false, // verbose
            None,  // env_file
            None,  // workspace
            false, // report
            false, // force_unlock
            None,  // output_mode
            false, // progress
            false, // timing
            0,     // retry
            true,  // yes — no confirmation
            false, // parallel
            None,  // resource_timeout
            false, // rollback_on_failure
            None,  // max_parallel
            None,  // notify,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )?;
    }

    println!(
        "\n{} Retried {} resource(s) successfully.",
        green("✓"),
        failed_resources.len()
    );
    Ok(())
}


/// FJ-324: Rolling deployment — apply N machines at a time, stop on failure.
pub(crate) fn cmd_rolling(
    file: &Path,
    state_dir: &Path,
    batch_size: usize,
    param_overrides: &[String],
    timeout: Option<u64>,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let machine_names: Vec<String> = config.machines.keys().cloned().collect();

    if machine_names.is_empty() {
        return Err("no machines defined in config".to_string());
    }

    let batches: Vec<Vec<String>> = machine_names
        .chunks(batch_size)
        .map(|chunk| chunk.to_vec())
        .collect();

    println!(
        "Rolling deploy: {} machines in {} batch(es) of {}",
        machine_names.len(),
        batches.len(),
        batch_size,
    );

    for (i, batch) in batches.iter().enumerate() {
        println!(
            "\n--- Batch {}/{}: {} ---",
            i + 1,
            batches.len(),
            batch.join(", ")
        );

        for machine in batch {
            cmd_apply(
                file,
                state_dir,
                Some(machine),
                None,  // resource_filter
                None,  // tag_filter
                None,  // group_filter
                false, // force
                false, // dry_run
                false, // no_tripwire
                param_overrides,
                false, // auto_commit
                timeout,
                false, // json
                false, // verbose
                None,  // env_file
                None,  // workspace
                false, // report
                false, // force_unlock
                None,  // output_mode
                false, // progress
                false, // timing
                0,     // retry
                true,  // yes
                false, // parallel
                None,  // resource_timeout
                false, // rollback_on_failure
                None,  // max_parallel
                None,  // notify,
                None,  // subset
                false, // confirm_destructive
                None,  // exclude
                false, // sequential
            )?;
        }

        println!("Batch {}/{} complete.", i + 1, batches.len());
    }

    println!(
        "\n{} Rolling deploy complete: {} machines converged.",
        green("✓"),
        machine_names.len()
    );
    Ok(())
}


/// FJ-325: Canary deployment — apply to one machine first, then rest.
pub(crate) fn cmd_canary(
    file: &Path,
    state_dir: &Path,
    canary_machine: &str,
    auto_proceed: bool,
    param_overrides: &[String],
    timeout: Option<u64>,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    if !config.machines.contains_key(canary_machine) {
        return Err(format!(
            "canary machine '{}' not found in config (available: {})",
            canary_machine,
            config
                .machines
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Phase 1: Apply to canary
    println!("=== Canary Phase: applying to '{}' ===\n", canary_machine);

    cmd_apply(
        file,
        state_dir,
        Some(canary_machine),
        None,
        None,
        None,
        false,
        false,
        false,
        param_overrides,
        false,
        timeout,
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
        None,  // subset
        false, // confirm_destructive
        None,  // exclude
        false, // sequential
    )?;

    println!("\n{} Canary '{}' succeeded.", green("✓"), canary_machine);

    // Phase 2: Apply to remaining machines
    let remaining: Vec<String> = config
        .machines
        .keys()
        .filter(|k| *k != canary_machine)
        .cloned()
        .collect();

    if remaining.is_empty() {
        println!("No remaining machines. Canary deploy complete.");
        return Ok(());
    }

    if !auto_proceed {
        println!(
            "\nCanary succeeded. Remaining machines: {}",
            remaining.join(", ")
        );
        println!("Use --auto-proceed to skip this confirmation in CI.");
        println!("Proceeding to remaining machines...");
    }

    println!(
        "\n=== Fleet Phase: applying to {} remaining machine(s) ===\n",
        remaining.len()
    );

    for machine in &remaining {
        cmd_apply(
            file,
            state_dir,
            Some(machine),
            None,
            None,
            None,
            false,
            false,
            false,
            param_overrides,
            false,
            timeout,
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
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )?;
    }

    println!(
        "\n{} Canary deploy complete: canary + {} machine(s) converged.",
        green("✓"),
        remaining.len()
    );
    Ok(())
}


/// FJ-326: List all machines with connection status.
pub(crate) fn cmd_inventory(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let mut results: Vec<serde_json::Value> = Vec::new();

    for (name, machine) in &config.machines {
        let is_local = machine.addr == "127.0.0.1" || machine.addr == "localhost";
        let is_container =
            machine.addr == "container" || machine.transport.as_deref() == Some("container");

        let (status, transport_type) = if is_local {
            ("reachable".to_string(), "local")
        } else if is_container {
            ("container".to_string(), "container")
        } else {
            // Try SSH connection test: ssh -o BatchMode=yes -o ConnectTimeout=5
            let user_host = format!("{}@{}", machine.user, machine.addr);
            let mut ssh_args = vec!["-o", "BatchMode=yes", "-o", "ConnectTimeout=5"];
            if let Some(ref key) = machine.ssh_key {
                ssh_args.push("-i");
                ssh_args.push(key);
            }
            ssh_args.push(&user_host);
            ssh_args.push("echo");
            ssh_args.push("ok");
            let result = std::process::Command::new("ssh")
                .args(&ssh_args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            match result {
                Ok(s) if s.success() => ("reachable".to_string(), "ssh"),
                _ => ("unreachable".to_string(), "ssh"),
            }
        };

        let resource_count = config
            .resources
            .values()
            .filter(|r| match &r.machine {
                types::MachineTarget::Single(m) => m == name,
                types::MachineTarget::Multiple(ms) => ms.contains(&name.to_string()),
            })
            .count();

        if json {
            results.push(serde_json::json!({
                "name": name,
                "hostname": machine.hostname,
                "addr": machine.addr,
                "user": machine.user,
                "arch": machine.arch,
                "transport": transport_type,
                "status": status,
                "roles": machine.roles,
                "resources": resource_count,
            }));
        } else {
            let status_icon = match status.as_str() {
                "reachable" => green("●"),
                "container" => dim("◆"),
                _ => red("✗"),
            };
            println!(
                "  {} {} ({}) [{}] — {} via {} ({} resources)",
                status_icon,
                name,
                machine.hostname,
                machine.addr,
                status,
                transport_type,
                resource_count,
            );
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&results).unwrap_or_default()
        );
    } else {
        println!("\n{} machines in inventory", config.machines.len());
    }

    Ok(())
}

