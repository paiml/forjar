use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// FJ-826: Convergence percentage over last N applies.
pub(crate) fn cmd_status_fleet_convergence_trend(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let trend = collect_convergence_trend(state_dir, &targets);
    if json {
        let items: Vec<String> = trend
            .iter()
            .map(|(m, pct)| format!("{{\"machine\":\"{m}\",\"convergence_pct\":{pct:.1}}}"))
            .collect();
        println!("{{\"fleet_convergence_trend\":[{}]}}", items.join(","));
    } else if trend.is_empty() {
        println!("No convergence data available.");
    } else {
        println!("Fleet convergence trend:");
        for (m, pct) in &trend {
            println!("  {m} — {pct:.1}% converged");
        }
    }
    Ok(())
}

fn collect_convergence_trend(state_dir: &Path, targets: &[&String]) -> Vec<(String, f64)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{m}.lock.yaml"));
        let content = match std::fs::read_to_string(&lock_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let total = lock.resources.len();
        if total == 0 {
            continue;
        }
        let converged = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Converged)
            .count();
        let pct = (converged as f64 / total as f64) * 100.0;
        results.push((m.to_string(), pct));
    }
    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}

/// FJ-828: Distribution of resource states across fleet.
pub(crate) fn cmd_status_resource_state_distribution(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let dist = collect_state_distribution(state_dir, &targets);
    if json {
        let items: Vec<String> = dist
            .iter()
            .map(|(s, c)| format!("{{\"state\":\"{s}\",\"count\":{c}}}"))
            .collect();
        println!("{{\"resource_state_distribution\":[{}]}}", items.join(","));
    } else if dist.is_empty() {
        println!("No resource state data available.");
    } else {
        println!("Resource state distribution:");
        for (s, c) in &dist {
            println!("  {s} — {c}");
        }
    }
    Ok(())
}

fn collect_state_distribution(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{m}.lock.yaml"));
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
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let counts = collect_apply_counts(state_dir, &targets);
    if json {
        let items: Vec<String> = counts
            .iter()
            .map(|(m, c)| format!("{{\"machine\":\"{m}\",\"resource_count\":{c}}}"))
            .collect();
        println!("{{\"machine_apply_counts\":[{}]}}", items.join(","));
    } else if counts.is_empty() {
        println!("No apply data available.");
    } else {
        println!("Apply counts per machine:");
        for (m, c) in &counts {
            println!("  {m} — {c} resources tracked");
        }
    }
    Ok(())
}

fn collect_apply_counts(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{m}.lock.yaml"));
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
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let history = collect_fleet_apply_history(state_dir, &targets);
    if json {
        let items: Vec<String> = history
            .iter()
            .map(|(m, r, t)| {
                format!(
                    "{{\"machine\":\"{m}\",\"resource\":\"{r}\",\"applied_at\":\"{t}\"}}"
                )
            })
            .collect();
        println!("{{\"fleet_apply_history\":[{}]}}", items.join(","));
    } else if history.is_empty() {
        println!("No apply history available.");
    } else {
        println!("Fleet apply history (most recent):");
        for (m, r, t) in &history {
            println!("  {m} / {r} — {t}");
        }
    }
    Ok(())
}

fn collect_fleet_apply_history(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, String, String)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{m}.lock.yaml"));
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
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let changes = collect_hash_changes(state_dir, &targets);
    if json {
        let items: Vec<String> = changes
            .iter()
            .map(|(m, r, h)| {
                format!(
                    "{{\"machine\":\"{m}\",\"resource\":\"{r}\",\"hash\":\"{h}\"}}"
                )
            })
            .collect();
        println!("{{\"resource_hash_changes\":[{}]}}", items.join(","));
    } else if changes.is_empty() {
        println!("No resource hash data available.");
    } else {
        println!("Resource hashes ({} tracked):", changes.len());
        for (m, r, h) in &changes {
            println!("  {m} / {r} — {h}");
        }
    }
    Ok(())
}

fn collect_hash_changes(state_dir: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{m}.lock.yaml"));
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
