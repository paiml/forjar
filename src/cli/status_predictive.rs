//! Status predictive — age distribution, convergence velocity, failure correlation, churn, staleness, trends.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::*;
#[allow(unused_imports)]
use super::helpers_state::*;

/// FJ-854: Age distribution of resources per machine.
pub(crate) fn cmd_status_machine_resource_age_distribution(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let dist = collect_age_distribution(state_dir, &targets);
    if json {
        let items: Vec<String> = dist.iter()
            .map(|(m, with, without)| format!("{{\"machine\":\"{}\",\"with_timestamp\":{},\"without_timestamp\":{}}}", m, with, without))
            .collect();
        println!("{{\"resource_age_distribution\":[{}]}}", items.join(","));
    } else if dist.is_empty() {
        println!("No age distribution data available.");
    } else {
        println!("Resource age distribution per machine:");
        for (m, with, without) in &dist { println!("  {} — {} with timestamp, {} without", m, with, without); }
    }
    Ok(())
}

fn collect_age_distribution(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c, Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l, Err(_) => continue,
        };
        let with_ts = lock.resources.values().filter(|r| r.applied_at.is_some()).count();
        let without_ts = lock.resources.values().filter(|r| r.applied_at.is_none()).count();
        results.push((m.to_string(), with_ts, without_ts));
    }
    results.sort();
    results
}

/// FJ-858: Rate of convergence across fleet.
pub(crate) fn cmd_status_fleet_convergence_velocity(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let velocity = compute_convergence_velocity(state_dir, &targets);
    if json {
        println!("{{\"fleet_convergence_velocity\":{{\"total\":{},\"converged\":{},\"rate\":{:.2}}}}}", velocity.0, velocity.1, velocity.2);
    } else {
        println!("Fleet convergence velocity: {}/{} resources converged ({:.1}%)", velocity.1, velocity.0, velocity.2 * 100.0);
    }
    Ok(())
}

fn compute_convergence_velocity(state_dir: &Path, targets: &[&String]) -> (usize, usize, f64) {
    let mut total = 0usize;
    let mut converged = 0usize;
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c, Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l, Err(_) => continue,
        };
        total += lock.resources.len();
        converged += lock.resources.values()
            .filter(|r| r.status == types::ResourceStatus::Converged)
            .count();
    }
    let rate = if total > 0 { converged as f64 / total as f64 } else { 0.0 };
    (total, converged, rate)
}

/// FJ-860: Correlate failures across resources.
pub(crate) fn cmd_status_resource_failure_correlation(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let correlations = find_failure_correlations(state_dir, &targets);
    if json {
        let items: Vec<String> = correlations.iter()
            .map(|(r, count)| format!("{{\"resource\":\"{}\",\"failure_count\":{}}}", r, count))
            .collect();
        println!("{{\"failure_correlations\":[{}]}}", items.join(","));
    } else if correlations.is_empty() {
        println!("No failure correlations found.");
    } else {
        println!("Resource failure correlations (across machines):");
        for (r, count) in &correlations { println!("  {} — failed on {} machines", r, count); }
    }
    Ok(())
}

fn find_failure_correlations(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut failure_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c, Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l, Err(_) => continue,
        };
        for (name, rs) in &lock.resources {
            if rs.status == types::ResourceStatus::Failed {
                *failure_counts.entry(name.clone()).or_default() += 1;
            }
        }
    }
    let mut results: Vec<(String, usize)> = failure_counts.into_iter().collect();
    results.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    results
}

/// FJ-862: Resource change frequency per machine over time.
pub(crate) fn cmd_status_machine_resource_churn_rate(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let mut rates: Vec<(String, usize)> = Vec::new();
    for m in &targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c, Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l, Err(_) => continue,
        };
        let churn = lock.resources.len();
        rates.push(((*m).clone(), churn));
    }
    rates.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if json {
        let items: Vec<String> = rates.iter()
            .map(|(m, c)| format!("{{\"machine\":\"{}\",\"resource_count\":{}}}", m, c)).collect();
        println!("{{\"machine_resource_churn_rate\":[{}]}}", items.join(","));
    } else if rates.is_empty() {
        println!("No resource churn data available.");
    } else {
        println!("Machine resource churn rate:");
        for (m, c) in &rates { println!("  {} — {} resources tracked", m, c); }
    }
    Ok(())
}

/// FJ-866: Identify resources not applied in configurable window (staleness).
pub(crate) fn cmd_status_fleet_resource_staleness(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let mut stale: Vec<(String, String, String)> = Vec::new();
    for m in &targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c, Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l, Err(_) => continue,
        };
        for (name, rs) in &lock.resources {
            let age = rs.applied_at.as_deref().unwrap_or("unknown");
            stale.push(((*m).clone(), name.clone(), age.to_string()));
        }
    }
    stale.sort_by(|a, b| a.2.cmp(&b.2));
    if json {
        let items: Vec<String> = stale.iter()
            .map(|(m, r, a)| format!("{{\"machine\":\"{}\",\"resource\":\"{}\",\"applied_at\":\"{}\"}}", m, r, a)).collect();
        println!("{{\"fleet_resource_staleness\":[{}]}}", items.join(","));
    } else if stale.is_empty() {
        println!("No staleness data available.");
    } else {
        println!("Fleet resource staleness (oldest first):");
        for (m, r, a) in &stale { println!("  {} / {} — last applied {}", m, r, a); }
    }
    Ok(())
}

/// FJ-868: Convergence trend per machine over time.
pub(crate) fn cmd_status_machine_convergence_trend(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let mut trends: Vec<(String, usize, usize, f64)> = Vec::new();
    for m in &targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c, Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l, Err(_) => continue,
        };
        let total = lock.resources.len();
        let converged = lock.resources.values()
            .filter(|r| r.status == types::ResourceStatus::Converged).count();
        let pct = if total > 0 { (converged as f64 / total as f64) * 100.0 } else { 0.0 };
        trends.push(((*m).clone(), converged, total, pct));
    }
    trends.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
    if json {
        let items: Vec<String> = trends.iter()
            .map(|(m, c, t, p)| format!("{{\"machine\":\"{}\",\"converged\":{},\"total\":{},\"pct\":{:.1}}}", m, c, t, p)).collect();
        println!("{{\"machine_convergence_trend\":[{}]}}", items.join(","));
    } else if trends.is_empty() {
        println!("No convergence trend data available.");
    } else {
        println!("Machine convergence trend:");
        for (m, c, t, p) in &trends { println!("  {} — {}/{} converged ({:.1}%)", m, c, t, p); }
    }
    Ok(())
}
