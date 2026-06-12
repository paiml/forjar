use crate::core::types;
use std::path::Path;

/// FJ-2005: Undo-destroy — replay from destroy-log.jsonl.
pub(crate) fn cmd_undo_destroy(
    state_dir: &Path,
    machine_filter: Option<&str>,
    force: bool,
    dry_run: bool,
) -> Result<(), String> {
    let entries = load_destroy_entries(state_dir, machine_filter)?;

    let reliable: Vec<_> = entries.iter().filter(|e| e.reliable_recreate).collect();
    let unreliable: Vec<_> = entries.iter().filter(|e| !e.reliable_recreate).collect();
    print_undo_plan(&entries, &reliable, &unreliable, force);

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
        match replay_entry(entry) {
            Ok(()) => {
                println!("  + {} ({})", entry.resource_id, entry.resource_type);
                replayed += 1;
            }
            Err(msg) => {
                eprintln!("{msg}");
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

/// Load destroy-log entries, filtered by machine when requested.
fn load_destroy_entries(
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<Vec<types::DestroyLogEntry>, String> {
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
    Ok(entries)
}

/// Print the undo plan: entry counts, per-entry lines, and the skip warning.
fn print_undo_plan(
    entries: &[types::DestroyLogEntry],
    reliable: &[&types::DestroyLogEntry],
    unreliable: &[&types::DestroyLogEntry],
    force: bool,
) {
    println!(
        "Undo-destroy: {} entries ({} reliable, {} best-effort)",
        entries.len(),
        reliable.len(),
        unreliable.len()
    );

    for e in reliable {
        println!("  + {} ({}, {})", e.resource_id, e.resource_type, e.machine);
    }
    for e in unreliable {
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
}

/// Replay one destroy-log entry: parse its config fragment, generate the
/// apply script, and execute it. Returns the failure message on error.
fn replay_entry(entry: &types::DestroyLogEntry) -> Result<(), String> {
    let resource = parse_replay_resource(entry)?;
    let config = build_replay_config(entry)?;
    let script = crate::core::codegen::apply_script(&resource)
        .map_err(|e| format!("  FAIL {}: codegen error: {e}", entry.resource_id))?;
    let machine = config.machines.get(&entry.machine).ok_or_else(|| {
        format!(
            "  SKIP {}: machine '{}' not in config",
            entry.resource_id, entry.machine
        )
    })?;
    run_replay_script(entry, machine, &script)
}

/// Parse the stored config fragment back into a Resource.
fn parse_replay_resource(entry: &types::DestroyLogEntry) -> Result<types::Resource, String> {
    let fragment = entry.config_fragment.as_ref().ok_or_else(|| {
        format!(
            "  SKIP {}: no config_fragment in destroy log",
            entry.resource_id
        )
    })?;
    serde_yaml_ng::from_str(fragment).map_err(|e| {
        format!(
            "  SKIP {}: cannot parse config_fragment: {e}",
            entry.resource_id
        )
    })
}

/// Build a minimal single-machine config for replay execution.
fn build_replay_config(entry: &types::DestroyLogEntry) -> Result<types::ForjarConfig, String> {
    let machine_name = &entry.machine;
    let machine_config = format!(
        "version: '1.0'\nname: undo-destroy-replay\nmachines:\n  {machine_name}:\n    hostname: {machine_name}\n    addr: 127.0.0.1\nresources: {{}}\n"
    );
    crate::core::parser::parse_config(&machine_config)
        .map_err(|e| format!("  SKIP {}: config error: {e}", entry.resource_id))
}

/// Execute the replay script on the machine; map failures to messages.
fn run_replay_script(
    entry: &types::DestroyLogEntry,
    machine: &types::Machine,
    script: &str,
) -> Result<(), String> {
    match crate::transport::exec_script(machine, script) {
        Ok(out) if out.success() => Ok(()),
        Ok(out) => Err(format!(
            "  FAIL {}: exit {}: {}",
            entry.resource_id,
            out.exit_code,
            out.stderr.trim()
        )),
        Err(e) => Err(format!("  FAIL {}: {e}", entry.resource_id)),
    }
}
