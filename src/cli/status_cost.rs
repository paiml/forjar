//! Cost and capacity.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use std::sync::atomic::Ordering;
use std::collections::HashMap;


/// FJ-537: Staleness report — show resources not applied within window.
pub(crate) fn cmd_status_staleness_report(
    state_dir: &Path,
    machine: Option<&str>,
    window: &str,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        machines.into_iter().filter(|n| n == m).collect()
    } else {
        machines
    };

    // Parse window as days
    let days: u64 = window.trim_end_matches('d').parse().unwrap_or(7);

    let threshold_secs = days * 86400;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut stale: Vec<(String, String, String)> = Vec::new(); // (machine, resource, last_applied)

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };

        // Check file modification time as proxy for last apply
        let mod_time = std::fs::metadata(&lock_path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if now.saturating_sub(mod_time) > threshold_secs {
            for resource_name in lock.resources.keys() {
                stale.push((
                    m.clone(),
                    resource_name.clone(),
                    format!("{}d ago", now.saturating_sub(mod_time) / 86400),
                ));
            }
        }
    }

    if json {
        let entries: Vec<String> = stale
            .iter()
            .map(|(m, r, age)| {
                format!(
                    r#"{{"machine":"{}","resource":"{}","last_applied":"{}"}}"#,
                    m, r, age
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if stale.is_empty() {
        println!(
            "{} All resources applied within {}d window.",
            green("✓"),
            days
        );
    } else {
        println!("Stale resources (not applied within {}d):\n", days);
        for (m, r, age) in &stale {
            println!("  {} {}:{} — last applied {}", yellow("⚠"), m, r, age);
        }
    }
    Ok(())
}


/// FJ-532: Cost estimate — estimate resource cost based on type counts.
pub(crate) fn cmd_status_cost_estimate(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        machines.into_iter().filter(|n| n == m).collect()
    } else {
        machines
    };

    // Cost units per resource type (relative complexity)
    let mut type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut total_resources = 0;

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };

        for rl in lock.resources.values() {
            let type_str = format!("{:?}", rl.resource_type);
            *type_counts.entry(type_str).or_insert(0) += 1;
            total_resources += 1;
        }
    }

    let cost_per_type = |t: &str| -> f64 {
        match t {
            "Package" => 2.0,
            "File" => 1.0,
            "Service" => 3.0,
            "Mount" => 4.0,
            "User" => 2.5,
            "Docker" => 5.0,
            "Cron" => 1.5,
            "Network" => 3.0,
            "Pepita" => 4.0,
            "Model" => 8.0,
            "Gpu" => 6.0,
            _ => 1.0,
        }
    };

    let total_cost: f64 = type_counts
        .iter()
        .map(|(t, c)| cost_per_type(t) * (*c as f64))
        .sum();

    if json {
        let entries: Vec<String> = type_counts
            .iter()
            .map(|(t, c)| {
                format!(
                    r#"{{"type":"{}","count":{},"unit_cost":{:.1},"total":{:.1}}}"#,
                    t,
                    c,
                    cost_per_type(t),
                    cost_per_type(t) * (*c as f64)
                )
            })
            .collect();
        println!(
            r#"{{"resources":{},"types":[{}],"total_cost":{:.1}}}"#,
            total_resources,
            entries.join(","),
            total_cost
        );
    } else {
        println!(
            "Cost estimate ({} resources across {} machines):\n",
            total_resources,
            machines.len()
        );
        let mut sorted: Vec<_> = type_counts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        for (t, c) in &sorted {
            let cost = cost_per_type(t) * (**c as f64);
            println!(
                "  {:>3}x {:12} @ {:.1} = {:.1} units",
                c,
                t,
                cost_per_type(t),
                cost
            );
        }
        println!("\n  Total: {:.1} complexity units", total_cost);
    }
    Ok(())
}


/// FJ-527: Status capacity — show resource utilization vs limits per machine.
pub(crate) fn cmd_status_capacity(state_dir: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        machines.into_iter().filter(|n| n == m).collect()
    } else {
        machines
    };

    let max_resources_per_machine = 50;
    let mut capacity_data: Vec<(String, usize, usize, f64)> = Vec::new(); // (machine, used, limit, pct)

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };

        let used = lock.resources.len();
        let pct = (used as f64 / max_resources_per_machine as f64) * 100.0;
        capacity_data.push((m.clone(), used, max_resources_per_machine, pct));
    }

    capacity_data.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

    if json {
        let entries: Vec<String> = capacity_data
            .iter()
            .map(|(m, used, limit, pct)| {
                format!(
                    r#"{{"machine":"{}","used":{},"limit":{},"utilization_pct":{:.1}}}"#,
                    m, used, limit, pct
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if capacity_data.is_empty() {
        println!("No machine state found.");
    } else {
        println!(
            "Resource capacity per machine (limit: {}):\n",
            max_resources_per_machine
        );
        for (m, used, limit, pct) in &capacity_data {
            let bar_len = (*pct / 5.0) as usize;
            let bar: String = "█".repeat(bar_len);
            let remaining: String = "░".repeat(20 - bar_len.min(20));
            let color_pct = if *pct > 80.0 {
                red(&format!("{:.0}%", pct))
            } else if *pct > 50.0 {
                yellow(&format!("{:.0}%", pct))
            } else {
                format!("{:.0}%", pct)
            };
            println!(
                "  {} {}{} {}/{} {}",
                m, bar, remaining, used, limit, color_pct
            );
        }
    }
    Ok(())
}

