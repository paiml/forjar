//! Phase 96 — Transport Diagnostics & Recipe Governance: status commands.

use std::collections::BTreeMap;
use std::path::Path;

/// The value of a top-level `key: value` line in a lock file, unquoted.
/// `None` when the lock does not carry that key at all.
fn lock_scalar(content: &str, key: &str) -> Option<String> {
    content.lines().find(|l| l.starts_with(key)).map(|l| {
        l.trim_start_matches(key)
            .trim()
            .trim_matches('"')
            .to_string()
    })
}

/// What every lock walk prints when the state directory cannot be read at all —
/// an empty JSON object, or the plain "nothing here" line.
fn print_no_machine_state(json: bool) {
    if json {
        println!("{{}}");
    } else {
        println!("  No machine state found.");
    }
}

/// Walks the state directory, honouring `--machine`, and summarises every
/// machine that has a `state.lock.yaml` with `summarise`, which is handed the
/// lock's path and its contents. `None` when the state directory itself cannot
/// be read — distinct from an empty map, which means "readable, but no locks".
fn summarise_machine_locks(
    state_dir: &Path,
    machine: Option<&str>,
    summarise: impl Fn(&Path, &str) -> serde_json::Value,
) -> Option<BTreeMap<String, serde_json::Value>> {
    let entries = std::fs::read_dir(state_dir).ok()?;

    let mut results: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if machine.is_some_and(|filter| name != filter) {
            continue;
        }
        let lock_path = entry.path().join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        results.insert(name, summarise(&lock_path, &content));
    }
    Some(results)
}

/// One machine's SSH connection health, inferred from its lock file: a lock
/// that was generated at all proves the machine was reachable.
fn ssh_connection_health(content: &str) -> serde_json::Value {
    let connected = content.contains("generated_at:");
    let generator = lock_scalar(content, "generator:").unwrap_or_else(|| "unknown".to_string());
    let transport = if generator.contains("ssh") || content.contains("ssh") {
        "ssh"
    } else {
        "local"
    };
    serde_json::json!({
        "connected": connected,
        "transport": transport,
        "healthy": connected,
    })
}

/// The human rendering of the SSH connection health report.
fn print_ssh_health_text(results: &BTreeMap<String, serde_json::Value>) {
    println!("=== Machine SSH Connection Health ===");
    if results.is_empty() {
        println!("  No machine state found.");
    }
    for (m, info) in results {
        let healthy = info["healthy"].as_bool().unwrap_or(false);
        let transport = info["transport"].as_str().unwrap_or("unknown");
        let symbol = if healthy { "✓" } else { "✗" };
        println!("  {symbol} {m}: transport={transport}, healthy={healthy}");
    }
}

/// FJ-1029: `status --machine-ssh-connection-health`
pub(crate) fn cmd_status_machine_ssh_connection_health(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let Some(results) = summarise_machine_locks(state_dir, machine, |_lock_path, content| {
        ssh_connection_health(content)
    }) else {
        print_no_machine_state(json);
        return Ok(());
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "machine_ssh_connection_health": results
            }))
            .unwrap_or_default()
        );
    } else {
        print_ssh_health_text(&results);
    }
    Ok(())
}

/// One lock file's staleness facts: when it was generated, how much it records,
/// and how big it is. A lock with no `generated_at` is treated as stale.
fn lock_staleness(lock_path: &Path, content: &str) -> serde_json::Value {
    let generated_at = lock_scalar(content, "generated_at:").unwrap_or_default();
    let resource_count = content
        .lines()
        .filter(|l| l.starts_with("  ") && l.contains("type:"))
        .count();
    let file_size = std::fs::metadata(lock_path).map(|m| m.len()).unwrap_or(0);
    serde_json::json!({
        "generated_at": generated_at,
        "resource_count": resource_count,
        "file_size_bytes": file_size,
        "stale": generated_at.is_empty(),
    })
}

/// The human rendering of the lock file staleness report.
fn print_staleness_text(results: &BTreeMap<String, serde_json::Value>) {
    println!("=== Lock File Staleness Report ===");
    if results.is_empty() {
        println!("  No lock files found.");
    }
    for (m, info) in results {
        let generated = info["generated_at"].as_str().unwrap_or("unknown");
        let count = info["resource_count"].as_u64().unwrap_or(0);
        let size = info["file_size_bytes"].as_u64().unwrap_or(0);
        let stale = info["stale"].as_bool().unwrap_or(true);
        let marker = if stale { " [STALE]" } else { "" };
        println!("  {m}: generated={generated}, resources={count}, size={size}B{marker}");
    }
}

/// FJ-1032: `status --lock-file-staleness-report`
pub(crate) fn cmd_status_lock_file_staleness_report(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let Some(results) = summarise_machine_locks(state_dir, machine, lock_staleness) else {
        print_no_machine_state(json);
        return Ok(());
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "lock_file_staleness_report": results
            }))
            .unwrap_or_default()
        );
    } else {
        print_staleness_text(&results);
    }
    Ok(())
}

/// FJ-1035: `status --fleet-transport-method-summary`
pub(crate) fn cmd_status_fleet_transport_method_summary(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let mut local_count = 0u64;
    let mut ssh_count = 0u64;
    let mut machines_local: Vec<String> = Vec::new();
    let mut machines_ssh: Vec<String> = Vec::new();

    let Ok(entries) = std::fs::read_dir(state_dir) else {
        if json {
            println!("{{}}");
        } else {
            println!("  No machine state found.");
        }
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine {
            if name != filter {
                continue;
            }
        }
        let lock_path = entry.path().join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let is_local = content.contains("addr: 127.0.0.1")
            || content.contains("addr: localhost")
            || content.contains("transport: local");
        if is_local {
            local_count += 1;
            machines_local.push(name);
        } else {
            ssh_count += 1;
            machines_ssh.push(name);
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "fleet_transport_method_summary": {
                    "local": { "count": local_count, "machines": machines_local },
                    "ssh": { "count": ssh_count, "machines": machines_ssh },
                }
            }))
            .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Transport Method Summary ===");
        println!("  Local: {local_count} machines {machines_local:?}");
        println!("  SSH:   {ssh_count} machines {machines_ssh:?}");
    }
    Ok(())
}
