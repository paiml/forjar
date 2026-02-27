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


/// FJ-766: Aggregate convergence across all machines.
pub(crate) fn cmd_status_fleet_convergence(
    state_dir: &Path, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let (mut total, mut converged) = (0usize, 0usize);
    for m in &machines {
        let (t, c, _, _) = super::status_resource_detail::tally_machine_health(state_dir, m);
        total += t;
        converged += c;
    }
    let pct = if total > 0 { converged * 100 / total } else { 100 };
    if json {
        println!("{{\"fleet_convergence_pct\":{},\"converged\":{},\"total\":{},\"machines\":{}}}", pct, converged, total, machines.len());
    } else {
        println!("Fleet convergence: {}% ({}/{} resources across {} machines)", pct, converged, total, machines.len());
    }
    Ok(())
}


/// FJ-770: Show BLAKE3 hash per resource from lock file.
pub(crate) fn cmd_status_resource_hash(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let entries = collect_hashes(state_dir, &targets);
    if json {
        let items: Vec<String> = entries.iter()
            .map(|(m, r, h)| format!("{{\"machine\":\"{}\",\"resource\":\"{}\",\"hash\":\"{}\"}}", m, r, h))
            .collect();
        println!("{{\"resource_hashes\":[{}]}}", items.join(","));
    } else if entries.is_empty() {
        println!("No resource hashes found.");
    } else {
        println!("Resource hashes:");
        for (m, r, h) in &entries { println!("  {} / {} — {}", m, r, h); }
    }
    Ok(())
}

/// Collect hash data from lock files.
fn collect_hashes(sd: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
    let mut entries = Vec::new();
    for m in targets {
        let lock_path = sd.join(format!("{}.lock.yaml", m));
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
                for (name, rl) in &lock.resources {
                    entries.push((m.to_string(), name.clone(), rl.hash.clone()));
                }
            }
        }
    }
    entries
}


/// FJ-772: Show drift percentage per machine.
pub(crate) fn cmd_status_machine_drift_summary(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let data: Vec<_> = targets.iter().map(|m| {
        let (t, _, _, d) = super::status_resource_detail::tally_machine_health(state_dir, m);
        let pct = if t > 0 { d * 100 / t } else { 0 };
        (m.to_string(), pct, d, t)
    }).collect();
    if json {
        let items: Vec<String> = data.iter()
            .map(|(m, p, d, t)| format!("{{\"machine\":\"{}\",\"drift_pct\":{},\"drifted\":{},\"total\":{}}}", m, p, d, t))
            .collect();
        println!("{{\"machine_drift\":[{}]}}", items.join(","));
    } else if data.is_empty() {
        println!("No machines found.");
    } else {
        println!("Machine drift summary:");
        for (m, pct, d, t) in &data { println!("  {} — {}% drifted ({}/{})", m, pct, d, t); }
    }
    Ok(())
}


/// FJ-774: Show total apply count per machine from event log.
pub(crate) fn cmd_status_apply_history_count(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let data: Vec<_> = targets.iter().map(|m| {
        let events_path = state_dir.join(m.as_str()).join("events.jsonl");
        let count = count_apply_events(&events_path);
        (m.to_string(), count)
    }).collect();
    if json {
        let items: Vec<String> = data.iter()
            .map(|(m, c)| format!("{{\"machine\":\"{}\",\"apply_count\":{}}}", m, c))
            .collect();
        println!("{{\"apply_history\":[{}]}}", items.join(","));
    } else if data.is_empty() {
        println!("No machines found.");
    } else {
        println!("Apply history counts:");
        for (m, c) in &data { println!("  {} — {} applies", m, c); }
    }
    Ok(())
}

/// Count apply_complete events in an events.jsonl file.
fn count_apply_events(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| l.contains("apply_complete"))
        .count()
}


/// FJ-778: Show number of lock files per machine.
pub(crate) fn cmd_status_lock_file_count(
    state_dir: &Path, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let count = machines.len();
    if json {
        println!("{{\"lock_file_count\":{},\"machines\":{:?}}}", count, machines);
    } else {
        println!("Lock files: {} ({} machines)", count, count);
        for m in &machines { println!("  {}.lock.yaml", m); }
    }
    Ok(())
}


/// FJ-780: Show resource type breakdown across fleet.
pub(crate) fn cmd_status_resource_type_distribution(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let dist = count_resource_types(&config);
    if json {
        let items: Vec<String> = dist.iter()
            .map(|(t, c)| format!("{{\"type\":\"{}\",\"count\":{}}}", t, c))
            .collect();
        println!("{{\"resource_types\":[{}]}}", items.join(","));
    } else if dist.is_empty() {
        println!("No resources.");
    } else {
        let total: usize = dist.iter().map(|(_, c)| c).sum();
        println!("Resource type distribution ({} total):", total);
        for (t, c) in &dist { println!("  {} — {}", t, c); }
    }
    Ok(())
}

/// Count resources by type.
fn count_resource_types(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for resource in config.resources.values() {
        *counts.entry(resource.resource_type.to_string()).or_default() += 1;
    }
    let mut result: Vec<(String, usize)> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result
}
