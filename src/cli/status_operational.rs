//! Status operational insights — last apply, fleet drift, apply durations.

use super::helpers::*;
#[allow(unused_imports)]
use super::helpers_state::*;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-814: Show last apply timestamp per machine.
pub(crate) fn cmd_status_machine_last_apply(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let timestamps = collect_last_apply_times(state_dir, &targets);
    if json {
        let items: Vec<String> = timestamps
            .iter()
            .map(|(m, t)| format!("{{\"machine\":\"{m}\",\"last_apply\":\"{t}\"}}"))
            .collect();
        println!("{{\"machine_last_apply\":[{}]}}", items.join(","));
    } else if timestamps.is_empty() {
        println!("No apply history available.");
    } else {
        println!("Last apply per machine:");
        for (m, t) in &timestamps {
            println!("  {m} — {t}");
        }
    }
    Ok(())
}

pub(super) fn collect_last_apply_times(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let ts = lock
            .resources
            .values()
            .filter_map(|r| r.applied_at.as_deref())
            .max()
            .unwrap_or("unknown")
            .to_string();
        results.push((m.to_string(), ts));
    }
    results.sort();
    results
}

/// FJ-818: Aggregated drift summary across all machines.
pub(crate) fn cmd_status_fleet_drift_summary(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let summary = collect_fleet_drift(state_dir, &targets);
    if json {
        let items: Vec<String> = summary
            .iter()
            .map(|(m, d, t)| format!("{{\"machine\":\"{m}\",\"drifted\":{d},\"total\":{t}}}"))
            .collect();
        println!("{{\"fleet_drift_summary\":[{}]}}", items.join(","));
    } else if summary.is_empty() {
        println!("No fleet drift data available.");
    } else {
        println!("Fleet drift summary:");
        for (m, d, t) in &summary {
            let pct = if *t > 0 {
                (*d as f64 / *t as f64) * 100.0
            } else {
                0.0
            };
            println!("  {m} — {d}/{t} drifted ({pct:.1}%)");
        }
    }
    Ok(())
}

pub(super) fn collect_fleet_drift(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, usize, usize)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let total = lock.resources.len();
        let drifted = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Drifted)
            .count();
        results.push((m.to_string(), drifted, total));
    }
    results.sort();
    results
}

/// FJ-820: Average apply duration per resource type.
pub(crate) fn cmd_status_resource_apply_duration(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let durations = collect_apply_durations(state_dir, &targets);
    if json {
        let items: Vec<String> = durations
            .iter()
            .map(|(rtype, avg)| {
                format!("{{\"resource_type\":\"{rtype}\",\"avg_duration_secs\":{avg:.2}}}")
            })
            .collect();
        println!("{{\"resource_apply_durations\":[{}]}}", items.join(","));
    } else if durations.is_empty() {
        println!("No apply duration data available.");
    } else {
        println!("Average apply duration per resource type:");
        for (rtype, avg) in &durations {
            println!("  {rtype} — {avg:.2}s");
        }
    }
    Ok(())
}

pub(super) fn collect_apply_durations(state_dir: &Path, targets: &[&String]) -> Vec<(String, f64)> {
    let mut type_durations: std::collections::HashMap<String, Vec<f64>> =
        std::collections::HashMap::new();
    for m in targets {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        for rs in lock.resources.values() {
            if let Some(dur) = rs.duration_seconds {
                let rtype = format!("{:?}", rs.resource_type);
                type_durations.entry(rtype).or_default().push(dur);
            }
        }
    }
    let mut results: Vec<(String, f64)> = type_durations
        .into_iter()
        .map(|(rtype, durs)| {
            let avg = durs.iter().sum::<f64>() / durs.len() as f64;
            (rtype, avg)
        })
        .collect();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

/// FJ-822: Per-machine breakdown of resource health status.
pub(crate) fn cmd_status_machine_resource_health(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let health = collect_machine_health(state_dir, &targets);
    if json {
        let items: Vec<String> = health
            .iter()
            .map(|(m, c, f, d)| {
                format!("{{\"machine\":\"{m}\",\"converged\":{c},\"failed\":{f},\"drifted\":{d}}}")
            })
            .collect();
        println!("{{\"machine_resource_health\":[{}]}}", items.join(","));
    } else if health.is_empty() {
        println!("No machine health data available.");
    } else {
        println!("Machine resource health:");
        for (m, c, f, d) in &health {
            println!("  {m} — converged: {c}, failed: {f}, drifted: {d}");
        }
    }
    Ok(())
}

pub(super) fn collect_machine_health(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, usize, usize, usize)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let converged = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Converged)
            .count();
        let failed = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Failed)
            .count();
        let drifted = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Drifted)
            .count();
        results.push((m.to_string(), converged, failed, drifted));
    }
    results.sort();
    results
}

pub(super) use super::status_operational_b::*;
