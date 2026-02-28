//! Status fleet detail — drift timing, resource counts, convergence scoring.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::*;
use super::helpers_state::*;


/// FJ-790: Show timestamp of last drift detection per resource.
pub(crate) fn cmd_status_last_drift_time(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let entries = collect_drift_times(state_dir, &targets);
    if json {
        let items: Vec<String> = entries.iter()
            .map(|(m, r, t)| format!("{{\"machine\":\"{}\",\"resource\":\"{}\",\"last_drift\":\"{}\"}}", m, r, t))
            .collect();
        println!("{{\"last_drift_times\":[{}]}}", items.join(","));
    } else if entries.is_empty() {
        println!("No drift detection data found.");
    } else {
        println!("Last drift detection times:");
        for (m, r, t) in &entries { println!("  {} / {} — {}", m, r, t); }
    }
    Ok(())
}

/// Collect last drift timestamps from lock files.
fn collect_drift_times(sd: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
    let mut entries = Vec::new();
    for m in targets {
        let lock_path = sd.join(format!("{}.lock.yaml", m));
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
                for (name, rl) in &lock.resources {
                    let ts = rl.applied_at.as_deref().unwrap_or("unknown").to_string();
                    if rl.status == types::ResourceStatus::Drifted {
                        entries.push((m.to_string(), name.clone(), ts));
                    }
                }
            }
        }
    }
    entries
}


/// FJ-794: Show resource count per machine from config.
pub(crate) fn cmd_status_machine_resource_count(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let counts = count_resources_per_machine(&config);
    if json {
        let items: Vec<String> = counts.iter()
            .map(|(m, c)| format!("{{\"machine\":\"{}\",\"resource_count\":{}}}", m, c))
            .collect();
        println!("{{\"machine_resource_counts\":[{}]}}", items.join(","));
    } else if counts.is_empty() {
        println!("No machine-resource mappings found.");
    } else {
        let total: usize = counts.iter().map(|(_, c)| c).sum();
        println!("Resources per machine ({} total):", total);
        for (m, c) in &counts { println!("  {} — {}", m, c); }
    }
    Ok(())
}

/// Count resources assigned to each machine.
fn count_resources_per_machine(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for resource in config.resources.values() {
        for m in resource.machine.to_vec() {
            *counts.entry(m).or_default() += 1;
        }
    }
    let mut result: Vec<(String, usize)> = counts.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result
}


/// FJ-796: Weighted convergence score across fleet.
pub(crate) fn cmd_status_convergence_score(
    state_dir: &Path, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let (mut total, mut converged, mut failed) = (0usize, 0usize, 0usize);
    for m in &machines {
        let (t, c, f, _) = super::status_resource_detail::tally_machine_health(state_dir, m);
        total += t;
        converged += c;
        failed += f;
    }
    // Weighted score: converged resources add points, failed subtract more
    let score = if total > 0 {
        let base = (converged * 100) as f64 / total as f64;
        let penalty = (failed * 20) as f64 / total as f64;
        (base - penalty).clamp(0.0, 100.0)
    } else {
        100.0
    };
    if json {
        println!("{{\"convergence_score\":{:.1},\"converged\":{},\"failed\":{},\"total\":{},\"machines\":{}}}", score, converged, failed, total, machines.len());
    } else {
        println!("Fleet convergence score: {:.1}/100 ({}/{} converged, {} failed, {} machines)", score, converged, total, failed, machines.len());
    }
    Ok(())
}


/// FJ-800: Show apply success rate per machine.
pub(crate) fn cmd_status_apply_success_rate(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let rates = compute_success_rates(state_dir, &targets);
    if json {
        let items: Vec<String> = rates.iter()
            .map(|(m, s, t)| format!("{{\"machine\":\"{}\",\"success\":{},\"total\":{},\"rate\":{:.1}}}", m, s, t, if *t > 0 { *s as f64 * 100.0 / *t as f64 } else { 100.0 }))
            .collect();
        println!("{{\"apply_success_rates\":[{}]}}", items.join(","));
    } else if rates.is_empty() {
        println!("No apply data found.");
    } else {
        println!("Apply success rates:");
        for (m, s, t) in &rates {
            let pct = if *t > 0 { *s as f64 * 100.0 / *t as f64 } else { 100.0 };
            println!("  {} — {:.1}% ({}/{})", m, pct, s, t);
        }
    }
    Ok(())
}

/// Compute success/total per machine from lock file resource statuses.
fn compute_success_rates(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut rates = Vec::new();
    for m in targets {
        let lock_path = sd.join(format!("{}.lock.yaml", m));
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
                let total = lock.resources.len();
                let success = lock.resources.values()
                    .filter(|r| r.status == types::ResourceStatus::Converged)
                    .count();
                rates.push((m.to_string(), success, total));
            }
        }
    }
    rates
}


/// FJ-802: Show error rate per machine.
pub(crate) fn cmd_status_error_rate(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let rates = compute_error_rates(state_dir, &targets);
    if json {
        let items: Vec<String> = rates.iter()
            .map(|(m, e, t)| format!("{{\"machine\":\"{}\",\"errors\":{},\"total\":{},\"rate\":{:.1}}}", m, e, t, if *t > 0 { *e as f64 * 100.0 / *t as f64 } else { 0.0 }))
            .collect();
        println!("{{\"error_rates\":[{}]}}", items.join(","));
    } else if rates.is_empty() {
        println!("No error data found.");
    } else {
        println!("Error rates:");
        for (m, e, t) in &rates {
            let pct = if *t > 0 { *e as f64 * 100.0 / *t as f64 } else { 0.0 };
            println!("  {} — {:.1}% ({}/{})", m, pct, e, t);
        }
    }
    Ok(())
}

/// Compute error/total per machine from lock file resource statuses.
fn compute_error_rates(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut rates = Vec::new();
    for m in targets {
        let lock_path = sd.join(format!("{}.lock.yaml", m));
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
                let total = lock.resources.len();
                let errors = lock.resources.values()
                    .filter(|r| r.status == types::ResourceStatus::Failed)
                    .count();
                rates.push((m.to_string(), errors, total));
            }
        }
    }
    rates
}


/// FJ-804: Aggregated fleet health summary.
pub(crate) fn cmd_status_fleet_health_summary(
    state_dir: &Path, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let (mut total_r, mut converged, mut failed, mut drifted) = (0usize, 0usize, 0usize, 0usize);
    for m in &machines {
        let (t, c, f, d) = super::status_resource_detail::tally_machine_health(state_dir, m);
        total_r += t;
        converged += c;
        failed += f;
        drifted += d;
    }
    let health_pct = if total_r > 0 { converged as f64 * 100.0 / total_r as f64 } else { 100.0 };
    if json {
        println!("{{\"fleet_health\":{{\"machines\":{},\"total_resources\":{},\"converged\":{},\"failed\":{},\"drifted\":{},\"health_pct\":{:.1}}}}}", machines.len(), total_r, converged, failed, drifted, health_pct);
    } else {
        println!("Fleet Health Summary");
        println!("  Machines: {}", machines.len());
        println!("  Total resources: {}", total_r);
        println!("  Converged: {} ({:.1}%)", converged, health_pct);
        println!("  Failed: {}", failed);
        println!("  Drifted: {}", drifted);
    }
    Ok(())
}
