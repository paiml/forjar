use crate::core::types;
use std::path::Path;

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
