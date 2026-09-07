//! Core status command.

use crate::core::{parser, state, types};
use std::path::Path;

/// Build a JSON resource entry, enriched with config if available.
fn build_json_resource_entry(
    id: &str,
    rl: &types::ResourceLock,
    config: &Option<types::ForjarConfig>,
) -> (String, serde_json::Value) {
    let mut entry = serde_json::json!({
        "type": rl.resource_type,
        "status": rl.status,
        "hash": &rl.hash,
    });
    if let Some(ref at) = rl.applied_at {
        entry["applied_at"] = serde_json::json!(at);
    }
    if let Some(dur) = rl.duration_seconds {
        entry["duration_seconds"] = serde_json::json!(dur);
    }
    if !rl.details.is_empty() {
        entry["details"] = serde_json::json!(rl.details);
    }
    enrich_json_entry(&mut entry, id, config);
    (id.to_string(), entry)
}

/// Enrich a JSON entry with config metadata (group, tags, depends_on).
fn enrich_json_entry(
    entry: &mut serde_json::Value,
    id: &str,
    config: &Option<types::ForjarConfig>,
) {
    if let Some(ref cfg) = config {
        if let Some(res) = cfg.resources.get(id) {
            if let Some(ref rg) = res.resource_group {
                entry["resource_group"] = serde_json::json!(rg);
            }
            if !res.tags.is_empty() {
                entry["tags"] = serde_json::json!(res.tags);
            }
            if !res.depends_on.is_empty() {
                entry["depends_on"] = serde_json::json!(res.depends_on);
            }
        }
    }
}

/// Print JSON output for the status command.
fn print_status_json(
    global: &Option<types::GlobalLock>,
    machines: &[types::StateLock],
    config: &Option<types::ForjarConfig>,
) -> Result<(), String> {
    let machine_values: Vec<serde_json::Value> = machines
        .iter()
        .map(|lock| {
            let resources: serde_json::Map<String, serde_json::Value> = lock
                .resources
                .iter()
                .map(|(id, rl)| build_json_resource_entry(id, rl, config))
                .collect();
            serde_json::json!({
                "machine": lock.machine,
                "hostname": lock.hostname,
                "generated_at": lock.generated_at,
                "generator": lock.generator,
                "blake3_version": lock.blake3_version,
                "resource_count": lock.resources.len(),
                "resources": resources,
            })
        })
        .collect();

    // Rebuild global summary from live lock data to avoid stale counts
    let live_global = if let Some(ref g) = global {
        let mut g2 = g.clone();
        for lock in machines {
            if let Some(ms) = g2.machines.get_mut(&lock.machine) {
                ms.resources = lock.resources.len();
            }
        }
        Some(g2)
    } else {
        None
    };
    let output = serde_json::json!({
        "global": live_global,
        "machines": machine_values,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output)
            .map_err(|e| format!("JSON serialization error: {e}"))?
    );
    Ok(())
}

/// Build the extras string for a resource from config metadata.
fn build_resource_extras(id: &str, config: &Option<types::ForjarConfig>) -> String {
    if let Some(ref cfg) = config {
        if let Some(res) = cfg.resources.get(id) {
            let mut parts = Vec::new();
            if let Some(ref rg) = res.resource_group {
                parts.push(format!("group={rg}"));
            }
            if !res.tags.is_empty() {
                parts.push(format!("tags={}", res.tags.join(",")));
            }
            if parts.is_empty() {
                return String::new();
            }
            return format!(" ({})", parts.join(", "));
        }
    }
    String::new()
}

/// forjar#469: the stack that WROTE this machine, read from the per-name stamp
/// map — not the `name` at the top of the lock, which is only whichever stack
/// applied last.
///
/// `None` when the dir holds fewer than two stacks, so the overwhelmingly
/// common one-config-one-state-dir output is byte-identical to what it has
/// always been. Attribution is a fact about a SHARED dir; printing it for a
/// single stack would be noise on every status in the wild.
fn machine_owner<'a>(global: &'a Option<types::GlobalLock>, machine: &str) -> Option<&'a str> {
    let g = global.as_ref()?;
    if g.stacks.len() < 2 {
        return None;
    }
    g.stacks
        .iter()
        .find(|(_, stamp)| stamp.machines.iter().any(|m| m == machine))
        .map(|(name, _)| name.as_str())
}

/// The `Project:`/`Generator:` block, plus the stack roll-call a shared dir
/// needs to be readable at all.
fn print_status_header(global: &Option<types::GlobalLock>) {
    let Some(g) = global.as_ref() else {
        return;
    };
    println!("Project: {} (last apply: {})", g.name, g.last_apply);
    println!("Generator: {}", g.generator);
    if g.stacks.len() > 1 {
        let names: Vec<&str> = g.stacks.keys().map(String::as_str).collect();
        println!("Stacks: {}", names.join(", "));
    }
    println!();
}

/// Print text output for the status command.
fn print_status_text(
    global: &Option<types::GlobalLock>,
    machines: &[types::StateLock],
    config: &Option<types::ForjarConfig>,
) {
    print_status_header(global);

    if machines.is_empty() {
        println!("No state found. Run `forjar apply` first.");
        return;
    }

    for lock in machines {
        println!("Machine: {} ({})", lock.machine, lock.hostname);
        if let Some(stack) = machine_owner(global, &lock.machine) {
            println!("  Stack: {stack}");
        }
        println!("  Generated: {}", lock.generated_at);
        println!("  Generator: {}", lock.generator);
        println!("  Resources: {}", lock.resources.len());

        for (id, rl) in &lock.resources {
            let duration = rl
                .duration_seconds
                .map(|d| format!(" ({d:.2}s)"))
                .unwrap_or_default();
            let extras = build_resource_extras(id, config);
            println!(
                "    {}: {} [{}]{}{}",
                id, rl.status, rl.resource_type, duration, extras
            );
        }
        println!();
    }
}

/// Converged / failed / drifted resource counts over a set of machine locks.
fn count_statuses<'a>(locks: impl Iterator<Item = &'a types::StateLock>) -> (u32, u32, u32) {
    let (mut converged, mut failed, mut drifted) = (0u32, 0u32, 0u32);
    for lock in locks {
        for (_, rl) in &lock.resources {
            match rl.status {
                types::ResourceStatus::Converged => converged += 1,
                types::ResourceStatus::Failed => failed += 1,
                types::ResourceStatus::Drifted => drifted += 1,
                types::ResourceStatus::Unknown => {}
            }
        }
    }
    (converged, failed, drifted)
}

/// Print summary mode output (FJ-303).
///
/// forjar#469: a dir with several stacks gets ONE LINE PER STACK, counting only
/// the machines that stack wrote. Aggregating every machine under the
/// last-applied `name` reported six machines' worth of failures against
/// whichever stack ran most recently.
fn print_status_summary(global: &Option<types::GlobalLock>, machines: &[types::StateLock]) {
    if let Some(g) = global.as_ref().filter(|g| g.stacks.len() > 1) {
        for (stack, stamp) in &g.stacks {
            let owned = machines
                .iter()
                .filter(|lock| stamp.machines.contains(&lock.machine));
            let (converged, failed, drifted) = count_statuses(owned);
            println!("{stack}: {converged} converged, {failed} failed, {drifted} drifted");
        }
        return;
    }
    let (converged, failed, drifted) = count_statuses(machines.iter());
    let name = global
        .as_ref()
        .map(|g| g.name.as_str())
        .unwrap_or("unknown");
    println!("{name}: {converged} converged, {failed} failed, {drifted} drifted");
}

pub(crate) fn cmd_status(
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
    config_file: Option<&Path>,
    summary: bool,
) -> Result<(), String> {
    let global = state::load_global_lock(state_dir)?;

    let config = if let Some(f) = config_file {
        Some(parser::parse_and_validate(f)?)
    } else {
        None
    };

    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut machines = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            machines.push(lock);
        }
    }

    if summary {
        print_status_summary(&global, &machines);
        return Ok(());
    }

    if json {
        print_status_json(&global, &machines, &config)?;
    } else {
        print_status_text(&global, &machines, &config);
    }

    Ok(())
}

// ============================================================================
// FJ-214: state-list -- tabular view of all resources in state
// ============================================================================
