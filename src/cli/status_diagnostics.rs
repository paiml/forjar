//! Status diagnostics — resource duration, machine-resource map.

use super::helpers::*;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-762: Show last apply duration per resource.
pub(crate) fn cmd_status_resource_duration(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let entries = collect_durations(state_dir, &targets);
    if json {
        let items: Vec<String> = entries
            .iter()
            .map(|(m, r, d)| {
                format!(
                    "{{\"machine\":\"{}\",\"resource\":\"{}\",\"duration_s\":{:.2}}}",
                    m, r, d
                )
            })
            .collect();
        println!("{{\"resource_durations\":[{}]}}", items.join(","));
    } else if entries.is_empty() {
        println!("No apply duration data found.");
    } else {
        println!("Resource apply durations:");
        for (m, r, d) in &entries {
            println!("  {} / {} — {:.2}s", m, r, d);
        }
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
pub(crate) fn cmd_status_machine_resource_map(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let map = build_machine_resource_map(&config);
    if json {
        let items: Vec<String> = map
            .iter()
            .map(|(m, rs)| format!("{{\"machine\":\"{}\",\"resources\":{:?}}}", m, rs))
            .collect();
        println!("{{\"machine_resource_map\":[{}]}}", items.join(","));
    } else if map.is_empty() {
        println!("No machine-resource mappings found.");
    } else {
        println!("Machine → Resource map:");
        for (m, rs) in &map {
            println!("  {} ({} resources):", m, rs.len());
            for r in rs {
                println!("    {}", r);
            }
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
    for (_, rs) in &mut result {
        rs.sort();
    }
    result
}

/// FJ-766: Aggregate convergence across all machines.
pub(crate) fn cmd_status_fleet_convergence(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let (mut total, mut converged) = (0usize, 0usize);
    for m in &machines {
        let (t, c, _, _) = super::status_resource_detail::tally_machine_health(state_dir, m);
        total += t;
        converged += c;
    }
    let pct = if total > 0 {
        converged * 100 / total
    } else {
        100
    };
    if json {
        println!(
            "{{\"fleet_convergence_pct\":{},\"converged\":{},\"total\":{},\"machines\":{}}}",
            pct,
            converged,
            total,
            machines.len()
        );
    } else {
        println!(
            "Fleet convergence: {}% ({}/{} resources across {} machines)",
            pct,
            converged,
            total,
            machines.len()
        );
    }
    Ok(())
}

/// FJ-770: Show BLAKE3 hash per resource from lock file.
pub(crate) fn cmd_status_resource_hash(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let entries = collect_hashes(state_dir, &targets);
    if json {
        let items: Vec<String> = entries
            .iter()
            .map(|(m, r, h)| {
                format!(
                    "{{\"machine\":\"{}\",\"resource\":\"{}\",\"hash\":\"{}\"}}",
                    m, r, h
                )
            })
            .collect();
        println!("{{\"resource_hashes\":[{}]}}", items.join(","));
    } else if entries.is_empty() {
        println!("No resource hashes found.");
    } else {
        println!("Resource hashes:");
        for (m, r, h) in &entries {
            println!("  {} / {} — {}", m, r, h);
        }
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
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let data: Vec<_> = targets
        .iter()
        .map(|m| {
            let (t, _, _, d) = super::status_resource_detail::tally_machine_health(state_dir, m);
            let pct = if t > 0 { d * 100 / t } else { 0 };
            (m.to_string(), pct, d, t)
        })
        .collect();
    if json {
        let items: Vec<String> = data
            .iter()
            .map(|(m, p, d, t)| {
                format!(
                    "{{\"machine\":\"{}\",\"drift_pct\":{},\"drifted\":{},\"total\":{}}}",
                    m, p, d, t
                )
            })
            .collect();
        println!("{{\"machine_drift\":[{}]}}", items.join(","));
    } else if data.is_empty() {
        println!("No machines found.");
    } else {
        println!("Machine drift summary:");
        for (m, pct, d, t) in &data {
            println!("  {} — {}% drifted ({}/{})", m, pct, d, t);
        }
    }
    Ok(())
}

/// FJ-774: Show total apply count per machine from event log.
pub(crate) fn cmd_status_apply_history_count(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let data: Vec<_> = targets
        .iter()
        .map(|m| {
            let events_path = state_dir.join(m.as_str()).join("events.jsonl");
            let count = count_apply_events(&events_path);
            (m.to_string(), count)
        })
        .collect();
    if json {
        let items: Vec<String> = data
            .iter()
            .map(|(m, c)| format!("{{\"machine\":\"{}\",\"apply_count\":{}}}", m, c))
            .collect();
        println!("{{\"apply_history\":[{}]}}", items.join(","));
    } else if data.is_empty() {
        println!("No machines found.");
    } else {
        println!("Apply history counts:");
        for (m, c) in &data {
            println!("  {} — {} applies", m, c);
        }
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
pub(crate) fn cmd_status_lock_file_count(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let count = machines.len();
    if json {
        println!(
            "{{\"lock_file_count\":{},\"machines\":{:?}}}",
            count, machines
        );
    } else {
        println!("Lock files: {} ({} machines)", count, count);
        for m in &machines {
            println!("  {}.lock.yaml", m);
        }
    }
    Ok(())
}

/// FJ-780: Show resource type breakdown across fleet.
pub(crate) fn cmd_status_resource_type_distribution(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let dist = count_resource_types(&config);
    if json {
        let items: Vec<String> = dist
            .iter()
            .map(|(t, c)| format!("{{\"type\":\"{}\",\"count\":{}}}", t, c))
            .collect();
        println!("{{\"resource_types\":[{}]}}", items.join(","));
    } else if dist.is_empty() {
        println!("No resources.");
    } else {
        let total: usize = dist.iter().map(|(_, c)| c).sum();
        println!("Resource type distribution ({} total):", total);
        for (t, c) in &dist {
            println!("  {} — {}", t, c);
        }
    }
    Ok(())
}

/// Count resources by type.
fn count_resource_types(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for resource in config.resources.values() {
        *counts
            .entry(resource.resource_type.to_string())
            .or_default() += 1;
    }
    let mut result: Vec<(String, usize)> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result
}

/// FJ-782: Show time since last apply per resource.
pub(crate) fn cmd_status_resource_apply_age(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let entries = collect_apply_ages(state_dir, &targets);
    if json {
        let items: Vec<String> = entries
            .iter()
            .map(|(m, r, age)| {
                format!(
                    "{{\"machine\":\"{}\",\"resource\":\"{}\",\"age\":\"{}\"}}",
                    m, r, age
                )
            })
            .collect();
        println!("{{\"resource_apply_ages\":[{}]}}", items.join(","));
    } else if entries.is_empty() {
        println!("No resource apply data found.");
    } else {
        println!("Resource apply ages:");
        for (m, r, age) in &entries {
            println!("  {} / {} — {}", m, r, age);
        }
    }
    Ok(())
}

/// Collect age strings from lock file applied_at timestamps.
fn collect_apply_ages(sd: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
    let now = std::time::SystemTime::now();
    let mut entries = Vec::new();
    for m in targets {
        let lock_path = sd.join(format!("{}.lock.yaml", m));
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
                for (name, rl) in &lock.resources {
                    let age = format_age_from_timestamp(rl.applied_at.as_deref(), &now);
                    entries.push((m.to_string(), name.clone(), age));
                }
            }
        }
    }
    entries
}

/// Format a human-readable age from an ISO timestamp.
fn format_age_from_timestamp(ts: Option<&str>, now: &std::time::SystemTime) -> String {
    let Some(ts) = ts else {
        return "unknown".to_string();
    };
    let Some(epoch_secs) = parse_rfc3339_to_epoch(ts) else {
        return "unknown".to_string();
    };
    let applied = std::time::UNIX_EPOCH + std::time::Duration::from_secs(epoch_secs);
    match now.duration_since(applied) {
        Ok(d) => format_duration_human(d.as_secs()),
        Err(_) => "in future".to_string(),
    }
}

/// Format seconds into human-readable age string.
fn format_duration_human(secs: u64) -> String {
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}

/// Parse RFC3339 timestamp to unix epoch seconds (simple parser, no chrono).
fn parse_rfc3339_to_epoch(ts: &str) -> Option<u64> {
    // Format: 2024-01-15T10:30:00Z or 2024-01-15T10:30:00+00:00
    let date_part = ts.get(..10)?;
    let time_part = ts.get(11..19)?;
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year: u64 = parts[0].parse().ok()?;
    let month: u64 = parts[1].parse().ok()?;
    let day: u64 = parts[2].parse().ok()?;
    let tparts: Vec<&str> = time_part.split(':').collect();
    if tparts.len() != 3 {
        return None;
    }
    let hour: u64 = tparts[0].parse().ok()?;
    let min: u64 = tparts[1].parse().ok()?;
    let sec: u64 = tparts[2].parse().ok()?;
    // Approximate days from epoch (good enough for age display)
    let days = days_from_epoch(year, month, day)?;
    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

/// Approximate days from unix epoch to a date.
fn days_from_epoch(year: u64, month: u64, day: u64) -> Option<u64> {
    if year < 1970 {
        return None;
    }
    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let month_days = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += month_days[m as usize];
        if m == 2 && is_leap(year) {
            days += 1;
        }
    }
    days += day - 1;
    Some(days)
}

/// Check if a year is a leap year.
fn is_leap(y: u64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

/// FJ-786: Show time since first apply per machine.
pub(crate) fn cmd_status_machine_uptime(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let now = std::time::SystemTime::now();
    let data: Vec<_> = targets
        .iter()
        .map(|m| {
            let events_path = state_dir.join(m.as_str()).join("events.jsonl");
            let age = first_event_age(&events_path, &now);
            (m.to_string(), age)
        })
        .collect();
    if json {
        let items: Vec<String> = data
            .iter()
            .map(|(m, a)| format!("{{\"machine\":\"{}\",\"uptime\":\"{}\"}}", m, a))
            .collect();
        println!("{{\"machine_uptime\":[{}]}}", items.join(","));
    } else if data.is_empty() {
        println!("No machines found.");
    } else {
        println!("Machine uptime (since first apply):");
        for (m, a) in &data {
            println!("  {} — {}", m, a);
        }
    }
    Ok(())
}

/// Get age since first event in events.jsonl.
fn first_event_age(path: &Path, now: &std::time::SystemTime) -> String {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return "no events".to_string(),
    };
    let first_line = match content.lines().next() {
        Some(l) => l,
        None => return "no events".to_string(),
    };
    // Try to extract timestamp from JSON event line
    if let Some(start) = first_line.find("\"timestamp\":\"") {
        let rest = &first_line[start + 13..];
        if let Some(end) = rest.find('"') {
            let ts = &rest[..end];
            return format_age_from_timestamp(Some(ts), now);
        }
    }
    "unknown".to_string()
}

/// FJ-788: Show apply frequency per resource over time.
pub(crate) fn cmd_status_resource_churn(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let data = collect_resource_churn(state_dir, &targets);
    if json {
        let items: Vec<String> = data
            .iter()
            .map(|(m, r, c)| {
                format!(
                    "{{\"machine\":\"{}\",\"resource\":\"{}\",\"apply_count\":{}}}",
                    m, r, c
                )
            })
            .collect();
        println!("{{\"resource_churn\":[{}]}}", items.join(","));
    } else if data.is_empty() {
        println!("No resource churn data found.");
    } else {
        println!("Resource churn (apply counts):");
        for (m, r, c) in &data {
            println!("  {} / {} — {} applies", m, r, c);
        }
    }
    Ok(())
}

/// Count per-resource apply events from event logs.
fn collect_resource_churn(sd: &Path, targets: &[&String]) -> Vec<(String, String, usize)> {
    let mut counts: std::collections::HashMap<(String, String), usize> =
        std::collections::HashMap::new();
    for m in targets {
        let events_path = sd.join(m.as_str()).join("events.jsonl");
        if let Ok(content) = std::fs::read_to_string(&events_path) {
            for line in content.lines() {
                if line.contains("resource_applied") {
                    if let Some(rname) = extract_resource_name(line) {
                        *counts.entry((m.to_string(), rname)).or_default() += 1;
                    }
                }
            }
        }
    }
    let mut result: Vec<(String, String, usize)> =
        counts.into_iter().map(|((m, r), c)| (m, r, c)).collect();
    result.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
    result
}

/// Extract resource name from a JSON event line.
fn extract_resource_name(line: &str) -> Option<String> {
    if let Some(start) = line.find("\"resource\":\"") {
        let rest = &line[start + 12..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}
