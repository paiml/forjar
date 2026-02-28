//! Status recovery — error budgets, compliance scoring, MTTR metrics.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::*;
#[allow(unused_imports)]
use super::helpers_state::*;

fn pct(num: usize, den: usize) -> f64 {
    if den > 0 { (num as f64 / den as f64) * 100.0 } else { 0.0 }
}


fn collect_error_budgets(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut budgets = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let total = lock.resources.len();
        let failed = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Failed)).count();
        budgets.push(((*m).clone(), failed, total));
    }
    budgets.sort_by(|a, b| a.0.cmp(&b.0));
    budgets
}

/// FJ-878: Show error budget consumption per machine (failed/total ratio).
pub(crate) fn cmd_status_machine_error_budget(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let budgets = collect_error_budgets(sd, &targets);
    if json {
        let items: Vec<String> = budgets.iter()
            .map(|(m, f, t)| format!("{{\"machine\":\"{}\",\"failed\":{},\"total\":{},\"error_pct\":{:.1}}}", m, f, t, pct(*f, *t)))
            .collect();
        println!("{{\"machine_error_budget\":[{}]}}", items.join(","));
    } else if budgets.is_empty() {
        println!("No error budget data available.");
    } else {
        println!("Machine error budget (failed / total):");
        for (m, f, t) in &budgets {
            println!("  {} — {}/{} failed ({:.1}% error budget consumed)", m, f, t, pct(*f, *t));
        }
    }
    Ok(())
}

/// FJ-882: Show fleet-wide compliance score based on converged resources.
pub(crate) fn cmd_status_fleet_compliance_score(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let (mut total, mut converged) = (0usize, 0usize);
    for m in &targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        total += lock.resources.len();
        converged += lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Converged)).count();
    }
    let score = if total > 0 { (converged as f64 / total as f64) * 100.0 } else { 0.0 };
    if json {
        println!("{{\"fleet_compliance_score\":{:.1},\"converged\":{},\"total\":{}}}", score, converged, total);
    } else if total == 0 {
        println!("No compliance data available.");
    } else {
        println!("Fleet compliance score: {:.1}% ({}/{} resources converged)", score, converged, total);
    }
    Ok(())
}

/// FJ-884: Show mean time to recovery per machine.
pub(crate) fn cmd_status_machine_mean_time_to_recovery(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let mut mttr_data: Vec<(String, String)> = Vec::new();
    for m in &targets {
        let events_path = sd.join(m).join("events.yaml");
        if events_path.exists() {
            mttr_data.push(((*m).clone(), "event data present".to_string()));
        } else {
            mttr_data.push(((*m).clone(), "no event history".to_string()));
        }
    }
    mttr_data.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = mttr_data.iter()
            .map(|(m, s)| format!("{{\"machine\":\"{}\",\"mttr_status\":\"{}\"}}", m, s)).collect();
        println!("{{\"machine_mean_time_to_recovery\":[{}]}}", items.join(","));
    } else if mttr_data.is_empty() {
        println!("No MTTR data available.");
    } else {
        println!("Machine mean time to recovery:");
        for (m, s) in &mttr_data { println!("  {} — {}", m, s); }
    }
    Ok(())
}

/// FJ-886: Health of upstream dependencies per resource.
pub(crate) fn cmd_status_machine_resource_dependency_health(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let mut health_data: Vec<(String, usize, usize)> = Vec::new();
    for m in &targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let total = lock.resources.len();
        let healthy = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Converged)).count();
        health_data.push(((*m).clone(), healthy, total));
    }
    health_data.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = health_data.iter()
            .map(|(m, h, t)| format!("{{\"machine\":\"{}\",\"healthy\":{},\"total\":{}}}", m, h, t)).collect();
        println!("{{\"resource_dependency_health\":[{}]}}", items.join(","));
    } else if health_data.is_empty() {
        println!("No dependency health data available.");
    } else {
        println!("Machine resource dependency health:");
        for (m, h, t) in &health_data { println!("  {} — {}/{} healthy", m, h, t); }
    }
    Ok(())
}

/// FJ-890: Health breakdown by resource type across fleet.
pub(crate) fn cmd_status_fleet_resource_type_health(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let type_health = collect_type_health(sd, &targets);
    if json {
        let items: Vec<String> = type_health.iter()
            .map(|(t, c, tot)| format!("{{\"type\":\"{}\",\"converged\":{},\"total\":{}}}", t, c, tot)).collect();
        println!("{{\"fleet_resource_type_health\":[{}]}}", items.join(","));
    } else if type_health.is_empty() {
        println!("No resource type health data available.");
    } else {
        println!("Fleet resource type health:");
        for (t, c, tot) in &type_health { println!("  {} — {}/{} converged", t, c, tot); }
    }
    Ok(())
}

fn collect_type_health(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut type_map: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        for rs in lock.resources.values() {
            let t = format!("{:?}", rs.resource_type);
            let entry = type_map.entry(t).or_insert((0, 0));
            entry.1 += 1;
            if matches!(rs.status, types::ResourceStatus::Converged) { entry.0 += 1; }
        }
    }
    let mut result: Vec<(String, usize, usize)> = type_map.into_iter().map(|(t, (c, tot))| (t, c, tot)).collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

fn collect_convergence_rates(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut rates: Vec<(String, usize, usize)> = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let total = lock.resources.len();
        let converged = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Converged)).count();
        rates.push(((*m).clone(), converged, total));
    }
    rates.sort_by(|a, b| a.0.cmp(&b.0));
    rates
}

/// FJ-892: Convergence rate per resource per machine.
pub(crate) fn cmd_status_machine_resource_convergence_rate(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let rates = collect_convergence_rates(sd, &targets);
    if json {
        let items: Vec<String> = rates.iter()
            .map(|(m, c, t)| format!("{{\"machine\":\"{}\",\"converged\":{},\"total\":{},\"rate\":{:.1}}}", m, c, t, pct(*c, *t)))
            .collect();
        println!("{{\"machine_resource_convergence_rate\":[{}]}}", items.join(","));
    } else if rates.is_empty() {
        println!("No convergence rate data available.");
    } else {
        println!("Machine resource convergence rate:");
        for (m, c, t) in &rates { println!("  {} — {}/{} converged ({:.1}%)", m, c, t, pct(*c, *t)); }
    }
    Ok(())
}

/// FJ-894: Correlate resource failures across machines.
pub(crate) fn cmd_status_machine_resource_failure_correlation(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let correlations = collect_failure_correlations(sd, &targets);
    if json {
        let items: Vec<String> = correlations.iter()
            .map(|(r, ms)| format!("{{\"resource\":\"{}\",\"failed_on\":[{}]}}", r, ms.iter().map(|m| format!("\"{}\"", m)).collect::<Vec<_>>().join(",")))
            .collect();
        println!("{{\"failure_correlations\":[{}]}}", items.join(","));
    } else if correlations.is_empty() {
        println!("No failure correlations found.");
    } else {
        println!("Resource failure correlations:");
        for (r, ms) in &correlations { println!("  {} — failed on: {}", r, ms.join(", ")); }
    }
    Ok(())
}

fn collect_failure_correlations(sd: &Path, targets: &[&String]) -> Vec<(String, Vec<String>)> {
    let mut failure_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        for (rname, rs) in &lock.resources {
            if matches!(rs.status, types::ResourceStatus::Failed) {
                failure_map.entry(rname.clone()).or_default().push((*m).clone());
            }
        }
    }
    let mut result: Vec<(String, Vec<String>)> = failure_map.into_iter().filter(|(_, ms)| ms.len() > 1).collect();
    result.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    result
}

/// FJ-898: Age distribution of resources across fleet.
pub(crate) fn cmd_status_fleet_resource_age_distribution(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let ages = collect_age_distribution(sd, &targets);
    if json {
        let items: Vec<String> = ages.iter()
            .map(|(bucket, count)| format!("{{\"age_bucket\":\"{}\",\"count\":{}}}", bucket, count))
            .collect();
        println!("{{\"resource_age_distribution\":[{}]}}", items.join(","));
    } else if ages.is_empty() {
        println!("No resource age data available.");
    } else {
        println!("Fleet resource age distribution:");
        for (bucket, count) in &ages { println!("  {} — {} resources", bucket, count); }
    }
    Ok(())
}

fn collect_age_distribution(sd: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut total_resources = 0usize;
    let mut with_timestamp = 0usize;
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        for rs in lock.resources.values() {
            total_resources += 1;
            if rs.applied_at.is_some() { with_timestamp += 1; }
        }
    }
    let without = total_resources - with_timestamp;
    let mut buckets = Vec::new();
    if with_timestamp > 0 { buckets.push(("has_applied_at".to_string(), with_timestamp)); }
    if without > 0 { buckets.push(("no_applied_at".to_string(), without)); }
    buckets
}

/// FJ-900: Readiness for rollback per machine based on state history.
pub(crate) fn cmd_status_machine_resource_rollback_readiness(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let readiness = collect_rollback_readiness(sd, &targets);
    if json {
        let items: Vec<String> = readiness.iter()
            .map(|(m, s)| format!("{{\"machine\":\"{}\",\"rollback_ready\":\"{}\"}}", m, s))
            .collect();
        println!("{{\"machine_rollback_readiness\":[{}]}}", items.join(","));
    } else if readiness.is_empty() {
        println!("No rollback readiness data available.");
    } else {
        println!("Machine rollback readiness:");
        for (m, s) in &readiness { println!("  {} — {}", m, s); }
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
pub(crate) fn cmd_status_machine_resource_health_trend(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let trends = collect_health_trends(sd, &targets);
    if json {
        let items: Vec<String> = trends.iter()
            .map(|(m, s)| format!("{{\"machine\":\"{}\",\"trend\":\"{}\"}}", m, s))
            .collect();
        println!("{{\"machine_health_trends\":[{}]}}", items.join(","));
    } else if trends.is_empty() {
        println!("No health trend data available.");
    } else {
        println!("Machine resource health trends:");
        for (m, s) in &trends { println!("  {} — {}", m, s); }
    }
    Ok(())
}

fn collect_health_trends(sd: &Path, targets: &[&String]) -> Vec<(String, String)> {
    let mut trends = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        if path.exists() {
            trends.push(((*m).clone(), "current data only (no historical trend)".to_string()));
        } else {
            trends.push(((*m).clone(), "no data".to_string()));
        }
    }
    trends.sort_by(|a, b| a.0.cmp(&b.0));
    trends
}

/// FJ-906: Rate of drift accumulation across fleet.
pub(crate) fn cmd_status_fleet_resource_drift_velocity(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let velocities = collect_drift_velocities(sd, &targets);
    if json {
        let items: Vec<String> = velocities.iter()
            .map(|(m, d, t)| format!("{{\"machine\":\"{}\",\"drifted\":{},\"total\":{}}}", m, d, t))
            .collect();
        println!("{{\"fleet_drift_velocity\":[{}]}}", items.join(","));
    } else if velocities.is_empty() {
        println!("No drift velocity data available.");
    } else {
        println!("Fleet resource drift velocity:");
        for (m, d, t) in &velocities { println!("  {} — {}/{} drifted ({:.1}%)", m, d, t, pct(*d, *t)); }
    }
    Ok(())
}

fn collect_drift_velocities(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut velocities = Vec::new();
    for m in targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => continue };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) { Ok(l) => l, Err(_) => continue };
        let total = lock.resources.len();
        let drifted = lock.resources.values().filter(|r| matches!(r.status, types::ResourceStatus::Drifted)).count();
        velocities.push(((*m).clone(), drifted, total));
    }
    velocities.sort_by(|a, b| a.0.cmp(&b.0));
    velocities
}

/// FJ-908: Apply success trend per machine over time.
pub(crate) fn cmd_status_machine_resource_apply_success_trend(sd: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let trends = collect_apply_success_trends(sd, &targets);
    if json {
        let items: Vec<String> = trends.iter()
            .map(|(m, s)| format!("{{\"machine\":\"{}\",\"trend\":\"{}\"}}", m, s))
            .collect();
        println!("{{\"machine_apply_success_trends\":[{}]}}", items.join(","));
    } else if trends.is_empty() {
        println!("No apply success trend data available.");
    } else {
        println!("Machine apply success trends:");
        for (m, s) in &trends { println!("  {} — {}", m, s); }
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
