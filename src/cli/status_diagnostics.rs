//! Status diagnostics — resource duration, machine-resource map.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::*;
use super::helpers_state::*;


/// FJ-762: Show last apply duration per resource.
pub(crate) fn cmd_status_resource_duration(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let entries = collect_durations(state_dir, &targets);
    if json {
        let items: Vec<String> = entries.iter()
            .map(|(m, r, d)| format!("{{\"machine\":\"{}\",\"resource\":\"{}\",\"duration_s\":{:.2}}}", m, r, d))
            .collect();
        println!("{{\"resource_durations\":[{}]}}", items.join(","));
    } else if entries.is_empty() {
        println!("No apply duration data found.");
    } else {
        println!("Resource apply durations:");
        for (m, r, d) in &entries { println!("  {} / {} — {:.2}s", m, r, d); }
    }
    Ok(())
}

/// Collect duration data from lock files.
fn collect_durations(sd: &Path, targets: &[&String]) -> Vec<(String, String, f64)> {
    let mut entries = Vec::new();
    for m in targets {
        let lock_path = sd.join(format!("{}.lock.yaml", m));
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
                for (name, rl) in &lock.resources {
                    let d = rl.duration_seconds.unwrap_or(0.0);
                    entries.push((m.to_string(), name.clone(), d));
                }
            }
        }
    }
    entries
}


/// FJ-764: Show which resources target each machine.
pub(crate) fn cmd_status_machine_resource_map(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let map = build_machine_resource_map(&config);
    if json {
        let items: Vec<String> = map.iter()
            .map(|(m, rs)| format!("{{\"machine\":\"{}\",\"resources\":{:?}}}", m, rs))
            .collect();
        println!("{{\"machine_resource_map\":[{}]}}", items.join(","));
    } else if map.is_empty() {
        println!("No machine-resource mappings found.");
    } else {
        println!("Machine → Resource map:");
        for (m, rs) in &map {
            println!("  {} ({} resources):", m, rs.len());
            for r in rs { println!("    {}", r); }
        }
    }
    Ok(())
}

/// Build mapping from machine name to list of resources targeting it.
fn build_machine_resource_map(config: &types::ForjarConfig) -> Vec<(String, Vec<String>)> {
    let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (name, resource) in &config.resources {
        for m in resource.machine.to_vec() {
            map.entry(m).or_default().push(name.clone());
        }
    }
    let mut result: Vec<(String, Vec<String>)> = map.into_iter().collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, rs) in &mut result { rs.sort(); }
    result
}
