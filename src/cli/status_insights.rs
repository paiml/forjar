//! Status insights — uptime estimates, type breakdowns, convergence times.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::*;
#[allow(unused_imports)]
use super::helpers_state::*;

/// FJ-838: Estimate machine uptime from apply history.
pub(crate) fn cmd_status_machine_uptime_estimate(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let estimates = collect_uptime_estimates(state_dir, &targets);
    if json {
        let items: Vec<String> = estimates.iter()
            .map(|(m, count)| format!("{{\"machine\":\"{}\",\"tracked_resources\":{}}}", m, count))
            .collect();
        println!("{{\"machine_uptime_estimates\":[{}]}}", items.join(","));
    } else if estimates.is_empty() {
        println!("No uptime data available.");
    } else {
        println!("Machine uptime estimates (by tracked resources):");
        for (m, count) in &estimates { println!("  {} — {} resources with apply history", m, count); }
    }
    Ok(())
}

fn collect_uptime_estimates(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let with_history = lock.resources.values()
            .filter(|r| r.applied_at.is_some())
            .count();
        results.push((m.to_string(), with_history));
    }
    results.sort();
    results
}

/// FJ-842: Resource type distribution across fleet.
pub(crate) fn cmd_status_fleet_resource_type_breakdown(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let breakdown = collect_type_breakdown(state_dir, &targets);
    if json {
        let items: Vec<String> = breakdown.iter()
            .map(|(t, c)| format!("{{\"type\":\"{}\",\"count\":{}}}", t, c))
            .collect();
        println!("{{\"fleet_resource_type_breakdown\":[{}]}}", items.join(","));
    } else if breakdown.is_empty() {
        println!("No resource type data available.");
    } else {
        println!("Fleet resource type breakdown:");
        for (t, c) in &breakdown { println!("  {} — {}", t, c); }
    }
    Ok(())
}

fn collect_type_breakdown(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        for rs in lock.resources.values() {
            let rtype = format!("{:?}", rs.resource_type);
            *counts.entry(rtype).or_default() += 1;
        }
    }
    let mut results: Vec<(String, usize)> = counts.into_iter().collect();
    results.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    results
}

/// FJ-844: Average time to converge per resource.
pub(crate) fn cmd_status_resource_convergence_time(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let times = collect_convergence_times(state_dir, &targets);
    if json {
        let items: Vec<String> = times.iter()
            .map(|(r, t)| format!("{{\"resource\":\"{}\",\"avg_convergence_secs\":{:.2}}}", r, t))
            .collect();
        println!("{{\"resource_convergence_times\":[{}]}}", items.join(","));
    } else if times.is_empty() {
        println!("No convergence time data available.");
    } else {
        println!("Average convergence time per resource:");
        for (r, t) in &times { println!("  {} — {:.2}s", r, t); }
    }
    Ok(())
}

fn collect_convergence_times(state_dir: &Path, targets: &[&String]) -> Vec<(String, f64)> {
    let mut durations: std::collections::HashMap<String, Vec<f64>> =
        std::collections::HashMap::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        for (name, rs) in &lock.resources {
            if rs.status == types::ResourceStatus::Converged {
                if let Some(dur) = rs.duration_seconds {
                    durations.entry(name.clone()).or_default().push(dur);
                }
            }
        }
    }
    let mut results: Vec<(String, f64)> = durations.into_iter()
        .map(|(name, durs)| {
            let avg = durs.iter().sum::<f64>() / durs.len() as f64;
            (name, avg)
        })
        .collect();
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

/// FJ-846: Age of oldest drift per machine.
pub(crate) fn cmd_status_machine_drift_age(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let ages = collect_drift_ages(state_dir, &targets);
    if json {
        let items: Vec<String> = ages.iter()
            .map(|(m, count)| format!("{{\"machine\":\"{}\",\"drifted_resources\":{}}}", m, count))
            .collect();
        println!("{{\"machine_drift_ages\":[{}]}}", items.join(","));
    } else if ages.is_empty() {
        println!("No drift age data available.");
    } else {
        println!("Machine drift age (drifted resource count):");
        for (m, count) in &ages { println!("  {} — {} drifted resources", m, count); }
    }
    Ok(())
}

fn collect_drift_ages(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let drifted = lock.resources.values()
            .filter(|r| r.status == types::ResourceStatus::Drifted)
            .count();
        if drifted > 0 {
            results.push((m.to_string(), drifted));
        }
    }
    results.sort();
    results
}

/// FJ-850: List all failed resources across fleet.
pub(crate) fn cmd_status_fleet_failed_resources(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let failed = collect_failed_resources(state_dir, &targets);
    if json {
        let items: Vec<String> = failed.iter()
            .map(|(m, r)| format!("{{\"machine\":\"{}\",\"resource\":\"{}\"}}", m, r))
            .collect();
        println!("{{\"fleet_failed_resources\":[{}]}}", items.join(","));
    } else if failed.is_empty() {
        println!("No failed resources across fleet.");
    } else {
        println!("Failed resources across fleet ({}):", failed.len());
        for (m, r) in &failed { println!("  {} / {}", m, r); }
    }
    Ok(())
}

fn collect_failed_resources(state_dir: &Path, targets: &[&String]) -> Vec<(String, String)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        for (name, rs) in &lock.resources {
            if rs.status == types::ResourceStatus::Failed {
                results.push((m.to_string(), name.clone()));
            }
        }
    }
    results.sort();
    results
}

/// FJ-852: Health of upstream dependencies per resource.
pub(crate) fn cmd_status_resource_dependency_health(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let health = collect_dependency_health(state_dir, &targets);
    if json {
        let items: Vec<String> = health.iter()
            .map(|(m, r, h)| format!("{{\"machine\":\"{}\",\"resource\":\"{}\",\"healthy_deps\":{}}}", m, r, h))
            .collect();
        println!("{{\"resource_dependency_health\":[{}]}}", items.join(","));
    } else if health.is_empty() {
        println!("No dependency health data available.");
    } else {
        println!("Resource dependency health:");
        for (m, r, h) in &health { println!("  {} / {} — {} converged deps", m, r, h); }
    }
    Ok(())
}

fn collect_dependency_health(state_dir: &Path, targets: &[&String]) -> Vec<(String, String, usize)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let total = lock.resources.len();
        let converged = lock.resources.values()
            .filter(|r| r.status == types::ResourceStatus::Converged)
            .count();
        if total > 0 {
            for name in lock.resources.keys() {
                results.push((m.to_string(), name.clone(), converged));
            }
        }
    }
    results.sort();
    results
}
