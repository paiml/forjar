//! Status operational insights — last apply, fleet drift, apply durations.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::*;
#[allow(unused_imports)]
use super::helpers_state::*;

/// FJ-814: Show last apply timestamp per machine.
pub(crate) fn cmd_status_machine_last_apply(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let timestamps = collect_last_apply_times(state_dir, &targets);
    if json {
        let items: Vec<String> = timestamps.iter()
            .map(|(m, t)| format!("{{\"machine\":\"{}\",\"last_apply\":\"{}\"}}", m, t))
            .collect();
        println!("{{\"machine_last_apply\":[{}]}}", items.join(","));
    } else if timestamps.is_empty() {
        println!("No apply history available.");
    } else {
        println!("Last apply per machine:");
        for (m, t) in &timestamps { println!("  {} — {}", m, t); }
    }
    Ok(())
}

fn collect_last_apply_times(state_dir: &Path, targets: &[&String]) -> Vec<(String, String)> {
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
        let ts = lock.resources.values()
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
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let summary = collect_fleet_drift(state_dir, &targets);
    if json {
        let items: Vec<String> = summary.iter()
            .map(|(m, d, t)| format!("{{\"machine\":\"{}\",\"drifted\":{},\"total\":{}}}", m, d, t))
            .collect();
        println!("{{\"fleet_drift_summary\":[{}]}}", items.join(","));
    } else if summary.is_empty() {
        println!("No fleet drift data available.");
    } else {
        println!("Fleet drift summary:");
        for (m, d, t) in &summary {
            let pct = if *t > 0 { (*d as f64 / *t as f64) * 100.0 } else { 0.0 };
            println!("  {} — {}/{} drifted ({:.1}%)", m, d, t, pct);
        }
    }
    Ok(())
}

fn collect_fleet_drift(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
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
        let drifted = lock.resources.values()
            .filter(|r| r.status == types::ResourceStatus::Drifted)
            .count();
        results.push((m.to_string(), drifted, total));
    }
    results.sort();
    results
}

/// FJ-820: Average apply duration per resource type.
pub(crate) fn cmd_status_resource_apply_duration(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let durations = collect_apply_durations(state_dir, &targets);
    if json {
        let items: Vec<String> = durations.iter()
            .map(|(rtype, avg)| format!("{{\"resource_type\":\"{}\",\"avg_duration_secs\":{:.2}}}", rtype, avg))
            .collect();
        println!("{{\"resource_apply_durations\":[{}]}}", items.join(","));
    } else if durations.is_empty() {
        println!("No apply duration data available.");
    } else {
        println!("Average apply duration per resource type:");
        for (rtype, avg) in &durations { println!("  {} — {:.2}s", rtype, avg); }
    }
    Ok(())
}

fn collect_apply_durations(state_dir: &Path, targets: &[&String]) -> Vec<(String, f64)> {
    let mut type_durations: std::collections::HashMap<String, Vec<f64>> =
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
        for rs in lock.resources.values() {
            if let Some(dur) = rs.duration_seconds {
                let rtype = format!("{:?}", rs.resource_type);
                type_durations.entry(rtype).or_default().push(dur);
            }
        }
    }
    let mut results: Vec<(String, f64)> = type_durations.into_iter()
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
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let health = collect_machine_health(state_dir, &targets);
    if json {
        let items: Vec<String> = health.iter()
            .map(|(m, c, f, d)| format!("{{\"machine\":\"{}\",\"converged\":{},\"failed\":{},\"drifted\":{}}}", m, c, f, d))
            .collect();
        println!("{{\"machine_resource_health\":[{}]}}", items.join(","));
    } else if health.is_empty() {
        println!("No machine health data available.");
    } else {
        println!("Machine resource health:");
        for (m, c, f, d) in &health { println!("  {} — converged: {}, failed: {}, drifted: {}", m, c, f, d); }
    }
    Ok(())
}

fn collect_machine_health(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize, usize, usize)> {
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
        let converged = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Converged).count();
        let failed = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Failed).count();
        let drifted = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Drifted).count();
        results.push((m.to_string(), converged, failed, drifted));
    }
    results.sort();
    results
}

/// FJ-826: Convergence percentage over last N applies.
pub(crate) fn cmd_status_fleet_convergence_trend(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let trend = collect_convergence_trend(state_dir, &targets);
    if json {
        let items: Vec<String> = trend.iter()
            .map(|(m, pct)| format!("{{\"machine\":\"{}\",\"convergence_pct\":{:.1}}}", m, pct))
            .collect();
        println!("{{\"fleet_convergence_trend\":[{}]}}", items.join(","));
    } else if trend.is_empty() {
        println!("No convergence data available.");
    } else {
        println!("Fleet convergence trend:");
        for (m, pct) in &trend { println!("  {} — {:.1}% converged", m, pct); }
    }
    Ok(())
}

fn collect_convergence_trend(state_dir: &Path, targets: &[&String]) -> Vec<(String, f64)> {
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
        if total == 0 { continue; }
        let converged = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Converged).count();
        let pct = (converged as f64 / total as f64) * 100.0;
        results.push((m.to_string(), pct));
    }
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

/// FJ-828: Distribution of resource states across fleet.
pub(crate) fn cmd_status_resource_state_distribution(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let dist = collect_state_distribution(state_dir, &targets);
    if json {
        let items: Vec<String> = dist.iter()
            .map(|(s, c)| format!("{{\"state\":\"{}\",\"count\":{}}}", s, c))
            .collect();
        println!("{{\"resource_state_distribution\":[{}]}}", items.join(","));
    } else if dist.is_empty() {
        println!("No resource state data available.");
    } else {
        println!("Resource state distribution:");
        for (s, c) in &dist { println!("  {} — {}", s, c); }
    }
    Ok(())
}

fn collect_state_distribution(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize)> {
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
            *counts.entry(rs.status.to_string()).or_default() += 1;
        }
    }
    let mut results: Vec<(String, usize)> = counts.into_iter().collect();
    results.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    results
}

/// FJ-830: Show number of applies per machine.
pub(crate) fn cmd_status_machine_apply_count(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let counts = collect_apply_counts(state_dir, &targets);
    if json {
        let items: Vec<String> = counts.iter()
            .map(|(m, c)| format!("{{\"machine\":\"{}\",\"resource_count\":{}}}", m, c))
            .collect();
        println!("{{\"machine_apply_counts\":[{}]}}", items.join(","));
    } else if counts.is_empty() {
        println!("No apply data available.");
    } else {
        println!("Apply counts per machine:");
        for (m, c) in &counts { println!("  {} — {} resources tracked", m, c); }
    }
    Ok(())
}

fn collect_apply_counts(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize)> {
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
        results.push((m.to_string(), lock.resources.len()));
    }
    results.sort();
    results
}

/// FJ-834: Show fleet-wide apply history (most recent applies).
pub(crate) fn cmd_status_fleet_apply_history(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let history = collect_fleet_apply_history(state_dir, &targets);
    if json {
        let items: Vec<String> = history.iter()
            .map(|(m, r, t)| format!("{{\"machine\":\"{}\",\"resource\":\"{}\",\"applied_at\":\"{}\"}}", m, r, t))
            .collect();
        println!("{{\"fleet_apply_history\":[{}]}}", items.join(","));
    } else if history.is_empty() {
        println!("No apply history available.");
    } else {
        println!("Fleet apply history (most recent):");
        for (m, r, t) in &history { println!("  {} / {} — {}", m, r, t); }
    }
    Ok(())
}

fn collect_fleet_apply_history(state_dir: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
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
            let ts = rs.applied_at.as_deref().unwrap_or("unknown");
            results.push((m.to_string(), name.clone(), ts.to_string()));
        }
    }
    results.sort_by(|a, b| b.2.cmp(&a.2));
    results.truncate(20);
    results
}

/// FJ-836: Show resources whose hash has changed between applies.
pub(crate) fn cmd_status_resource_hash_changes(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let changes = collect_hash_changes(state_dir, &targets);
    if json {
        let items: Vec<String> = changes.iter()
            .map(|(m, r, h)| format!("{{\"machine\":\"{}\",\"resource\":\"{}\",\"hash\":\"{}\"}}", m, r, h))
            .collect();
        println!("{{\"resource_hash_changes\":[{}]}}", items.join(","));
    } else if changes.is_empty() {
        println!("No resource hash data available.");
    } else {
        println!("Resource hashes ({} tracked):", changes.len());
        for (m, r, h) in &changes { println!("  {} / {} — {}", m, r, h); }
    }
    Ok(())
}

fn collect_hash_changes(state_dir: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
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
            if !rs.hash.is_empty() {
                results.push((m.to_string(), name.clone(), rs.hash.clone()));
            }
        }
    }
    results.sort();
    results
}
