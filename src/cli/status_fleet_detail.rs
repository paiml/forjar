//! Status fleet detail — drift timing, resource counts, convergence scoring.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::*;


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

/// Collect convergence history data from lock files.
fn collect_convergence_history(sd: &Path, targets: &[&String]) -> Vec<(String, String, f64)> {
    let mut history = Vec::new();
    for m in targets {
        let lock_path = sd.join(format!("{}.lock.yaml", m));
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
                let total = lock.resources.len();
                if total == 0 { continue; }
                let converged = lock.resources.values()
                    .filter(|r| r.status == types::ResourceStatus::Converged)
                    .count();
                let pct = converged as f64 * 100.0 / total as f64;
                let ts = lock.resources.values()
                    .filter_map(|r| r.applied_at.clone())
                    .max()
                    .unwrap_or_else(|| "unknown".to_string());
                history.push((m.to_string(), ts, pct));
            }
        }
    }
    history.sort_by(|a, b| a.0.cmp(&b.0));
    history
}

/// FJ-806: Convergence trend per machine over time.
pub(crate) fn cmd_status_machine_convergence_history(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let history = collect_convergence_history(state_dir, &targets);
    if json {
        let items: Vec<String> = history.iter()
            .map(|(m, ts, pct)| format!("{{\"machine\":\"{}\",\"time\":\"{}\",\"convergence_pct\":{:.1}}}", m, ts, pct))
            .collect();
        println!("{{\"machine_convergence_history\":[{}]}}", items.join(","));
    } else if history.is_empty() {
        println!("No convergence history available.");
    } else {
        println!("Machine convergence history:");
        for (m, ts, pct) in &history {
            println!("  {} — {:.1}% converged (at {})", m, pct, ts);
        }
    }
    Ok(())
}

/// Collect drift events from lock files.
fn collect_drift_events(sd: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
    let mut events = Vec::new();
    for m in targets {
        let lock_path = sd.join(format!("{}.lock.yaml", m));
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
                for (rname, rlock) in &lock.resources {
                    if rlock.status == types::ResourceStatus::Drifted {
                        let ts = rlock.applied_at.clone().unwrap_or_else(|| "unknown".to_string());
                        events.push((ts, m.to_string(), rname.clone()));
                    }
                }
            }
        }
    }
    events.sort();
    events
}

/// FJ-810: Drift events timeline across fleet.
pub(crate) fn cmd_status_drift_history(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let events = collect_drift_events(state_dir, &targets);
    if json {
        let items: Vec<String> = events.iter()
            .map(|(ts, m, r)| format!("{{\"time\":\"{}\",\"machine\":\"{}\",\"resource\":\"{}\"}}", ts, m, r))
            .collect();
        println!("{{\"drift_history\":[{}]}}", items.join(","));
    } else if events.is_empty() {
        println!("No drift events recorded.");
    } else {
        println!("Drift history ({} events):", events.len());
        for (ts, m, r) in &events { println!("  [{}] {} on {}", ts, r, m); }
    }
    Ok(())
}

/// Collect per-resource failure stats from lock files.
fn collect_failure_stats(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize, f64)> {
    let mut stats: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();
    for m in targets {
        let lock_path = sd.join(format!("{}.lock.yaml", m));
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
                for (rname, rlock) in &lock.resources {
                    let entry = stats.entry(rname.clone()).or_insert((0, 0));
                    entry.0 += 1;
                    if rlock.status == types::ResourceStatus::Failed {
                        entry.1 += 1;
                    }
                }
            }
        }
    }
    let mut rates: Vec<(String, usize, usize, f64)> = stats.into_iter()
        .map(|(name, (total, failed))| {
            let rate = if total > 0 { failed as f64 * 100.0 / total as f64 } else { 0.0 };
            (name, total, failed, rate)
        })
        .collect();
    rates.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
    rates
}

/// FJ-812: Failure rate per resource across applies.
pub(crate) fn cmd_status_resource_failure_rate(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let rates = collect_failure_stats(state_dir, &targets);
    if json {
        let items: Vec<String> = rates.iter()
            .map(|(r, t, f, rate)| format!("{{\"resource\":\"{}\",\"total\":{},\"failed\":{},\"failure_rate_pct\":{:.1}}}", r, t, f, rate))
            .collect();
        println!("{{\"resource_failure_rates\":[{}]}}", items.join(","));
    } else if rates.is_empty() {
        println!("No resource data available.");
    } else {
        println!("Resource failure rates:");
        for (r, t, f, rate) in &rates {
            println!("  {} — {}/{} failed ({:.1}%)", r, f, t, rate);
        }
    }
    Ok(())
}
