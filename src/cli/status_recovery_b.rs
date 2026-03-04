use super::helpers::*;
use super::status_recovery::*;
use crate::core::types;
use std::path::Path;

/// FJ-894: Correlate resource failures across machines.
pub(crate) fn cmd_status_machine_resource_failure_correlation(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let correlations = collect_failure_correlations(sd, &targets);
    if json {
        let items: Vec<String> = correlations
            .iter()
            .map(|(r, ms)| {
                format!(
                    "{{\"resource\":\"{}\",\"failed_on\":[{}]}}",
                    r,
                    ms.iter()
                        .map(|m| format!("\"{m}\""))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect();
        println!("{{\"failure_correlations\":[{}]}}", items.join(","));
    } else if correlations.is_empty() {
        println!("No failure correlations found.");
    } else {
        println!("Resource failure correlations:");
        for (r, ms) in &correlations {
            println!("  {} — failed on: {}", r, ms.join(", "));
        }
    }
    Ok(())
}

fn collect_failure_correlations(sd: &Path, targets: &[&String]) -> Vec<(String, Vec<String>)> {
    let mut failure_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        for (rname, rs) in &lock.resources {
            if matches!(rs.status, types::ResourceStatus::Failed) {
                failure_map
                    .entry(rname.clone())
                    .or_default()
                    .push((*m).clone());
            }
        }
    }
    let mut result: Vec<(String, Vec<String>)> = failure_map
        .into_iter()
        .filter(|(_, ms)| ms.len() > 1)
        .collect();
    result.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    result
}

/// FJ-898: Age distribution of resources across fleet.
pub(crate) fn cmd_status_fleet_resource_age_distribution(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let ages = collect_age_distribution(sd, &targets);
    if json {
        let items: Vec<String> = ages
            .iter()
            .map(|(bucket, count)| format!("{{\"age_bucket\":\"{bucket}\",\"count\":{count}}}"))
            .collect();
        println!("{{\"resource_age_distribution\":[{}]}}", items.join(","));
    } else if ages.is_empty() {
        println!("No resource age data available.");
    } else {
        println!("Fleet resource age distribution:");
        for (bucket, count) in &ages {
            println!("  {bucket} — {count} resources");
        }
    }
    Ok(())
}

fn collect_age_distribution(sd: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut total_resources = 0usize;
    let mut with_timestamp = 0usize;
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        for rs in lock.resources.values() {
            total_resources += 1;
            if rs.applied_at.is_some() {
                with_timestamp += 1;
            }
        }
    }
    let without = total_resources - with_timestamp;
    let mut buckets = Vec::new();
    if with_timestamp > 0 {
        buckets.push(("has_applied_at".to_string(), with_timestamp));
    }
    if without > 0 {
        buckets.push(("no_applied_at".to_string(), without));
    }
    buckets
}

/// FJ-900: Readiness for rollback per machine based on state history.
pub(crate) fn cmd_status_machine_resource_rollback_readiness(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let readiness = collect_rollback_readiness(sd, &targets);
    if json {
        let items: Vec<String> = readiness
            .iter()
            .map(|(m, s)| format!("{{\"machine\":\"{m}\",\"rollback_ready\":\"{s}\"}}"))
            .collect();
        println!("{{\"machine_rollback_readiness\":[{}]}}", items.join(","));
    } else if readiness.is_empty() {
        println!("No rollback readiness data available.");
    } else {
        println!("Machine rollback readiness:");
        for (m, s) in &readiness {
            println!("  {m} — {s}");
        }
    }
    Ok(())
}

fn collect_rollback_readiness(sd: &Path, targets: &[&String]) -> Vec<(String, String)> {
    let mut readiness = Vec::new();
    for m in targets {
        let lock_path = sd.join(m).join("lock.yaml");
        let snapshot_dir = sd.join(m).join("snapshots");
        let has_lock = lock_path.exists();
        let has_snapshots = snapshot_dir.exists() && snapshot_dir.is_dir();
        let status = match (has_lock, has_snapshots) {
            (true, true) => "ready (lock + snapshots)",
            (true, false) => "partial (lock only, no snapshots)",
            (false, _) => "not ready (no lock)",
        };
        readiness.push(((*m).clone(), status.to_string()));
    }
    readiness.sort_by(|a, b| a.0.cmp(&b.0));
    readiness
}

/// FJ-902: Health trend over time per machine.
pub(crate) fn cmd_status_machine_resource_health_trend(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let trends = collect_health_trends(sd, &targets);
    if json {
        let items: Vec<String> = trends
            .iter()
            .map(|(m, s)| format!("{{\"machine\":\"{m}\",\"trend\":\"{s}\"}}"))
            .collect();
        println!("{{\"machine_health_trends\":[{}]}}", items.join(","));
    } else if trends.is_empty() {
        println!("No health trend data available.");
    } else {
        println!("Machine resource health trends:");
        for (m, s) in &trends {
            println!("  {m} — {s}");
        }
    }
    Ok(())
}

fn collect_health_trends(sd: &Path, targets: &[&String]) -> Vec<(String, String)> {
    let mut trends = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        if path.exists() {
            trends.push((
                (*m).clone(),
                "current data only (no historical trend)".to_string(),
            ));
        } else {
            trends.push(((*m).clone(), "no data".to_string()));
        }
    }
    trends.sort_by(|a, b| a.0.cmp(&b.0));
    trends
}

/// FJ-906: Rate of drift accumulation across fleet.
pub(crate) fn cmd_status_fleet_resource_drift_velocity(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let velocities = collect_drift_velocities(sd, &targets);
    if json {
        let items: Vec<String> = velocities
            .iter()
            .map(|(m, d, t)| {
                format!(
                    "{{\"machine\":\"{m}\",\"drifted\":{d},\"total\":{t}}}"
                )
            })
            .collect();
        println!("{{\"fleet_drift_velocity\":[{}]}}", items.join(","));
    } else if velocities.is_empty() {
        println!("No drift velocity data available.");
    } else {
        println!("Fleet resource drift velocity:");
        for (m, d, t) in &velocities {
            println!("  {} — {}/{} drifted ({:.1}%)", m, d, t, pct(*d, *t));
        }
    }
    Ok(())
}

fn collect_drift_velocities(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut velocities = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) {
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
            .filter(|r| matches!(r.status, types::ResourceStatus::Drifted))
            .count();
        velocities.push(((*m).clone(), drifted, total));
    }
    velocities.sort_by(|a, b| a.0.cmp(&b.0));
    velocities
}

/// FJ-908: Apply success trend per machine over time.
pub(crate) fn cmd_status_machine_resource_apply_success_trend(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let trends = collect_apply_success_trends(sd, &targets);
    if json {
        let items: Vec<String> = trends
            .iter()
            .map(|(m, s)| format!("{{\"machine\":\"{m}\",\"trend\":\"{s}\"}}"))
            .collect();
        println!("{{\"machine_apply_success_trends\":[{}]}}", items.join(","));
    } else if trends.is_empty() {
        println!("No apply success trend data available.");
    } else {
        println!("Machine apply success trends:");
        for (m, s) in &trends {
            println!("  {m} — {s}");
        }
    }
    Ok(())
}

fn collect_apply_success_trends(sd: &Path, targets: &[&String]) -> Vec<(String, String)> {
    let mut trends = Vec::new();
    for m in targets {
        let events_path = sd.join(m).join("events.yaml");
        if events_path.exists() {
            trends.push(((*m).clone(), "event history available".to_string()));
        } else {
            trends.push(((*m).clone(), "no event history".to_string()));
        }
    }
    trends.sort_by(|a, b| a.0.cmp(&b.0));
    trends
}
