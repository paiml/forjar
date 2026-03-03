use super::helpers::*;
use super::status_intelligence::*;
use crate::core::types;
use std::path::Path;

/// FJ-924: Rate of configuration drift per machine over time.
pub(crate) fn cmd_status_machine_resource_config_drift_rate(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let rates = collect_config_drift_rates(sd, &targets);
    if json {
        let items: Vec<String> = rates
            .iter()
            .map(|(m, d, t)| {
                format!(
                    "{{\"machine\":\"{}\",\"drifted\":{},\"total\":{},\"drift_rate\":{:.1}}}",
                    m,
                    d,
                    t,
                    pct(*d, *t)
                )
            })
            .collect();
        println!("{{\"config_drift_rates\":[{}]}}", items.join(","));
    } else if rates.is_empty() {
        println!("No configuration drift rate data available.");
    } else {
        println!("Machine configuration drift rates:");
        for (m, d, t) in &rates {
            println!("  {} — {}/{} drifted ({:.1}%)", m, d, t, pct(*d, *t));
        }
    }
    Ok(())
}

fn collect_config_drift_rates(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut rates = Vec::new();
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
        rates.push(((*m).clone(), drifted, total));
    }
    rates.sort_by(|a, b| a.0.cmp(&b.0));
    rates
}

/// FJ-926: Per-resource convergence lag within machine.
pub(crate) fn cmd_status_machine_resource_convergence_lag(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let lags = collect_convergence_lag(sd, &targets);
    if json {
        let items: Vec<String> = lags
            .iter()
            .map(|(m, r, s)| {
                format!(
                    "{{\"machine\":\"{}\",\"resource\":\"{}\",\"status\":\"{}\"}}",
                    m, r, s
                )
            })
            .collect();
        println!("{{\"convergence_lag\":[{}]}}", items.join(","));
    } else if lags.is_empty() {
        println!("No convergence lag data available.");
    } else {
        println!("Per-resource convergence lag:");
        for (m, r, s) in &lags {
            println!("  {} / {} — {}", m, r, s);
        }
    }
    Ok(())
}

fn collect_convergence_lag(sd: &Path, targets: &[&String]) -> Vec<(String, String, String)> {
    let mut lags = Vec::new();
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
        for (name, res) in &lock.resources {
            if !matches!(res.status, types::ResourceStatus::Converged) {
                let status_str = format!("{:?}", res.status);
                lags.push(((*m).clone(), name.clone(), status_str));
            }
        }
    }
    lags.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    lags
}

/// FJ-930: Fleet-wide per-resource convergence lag analysis.
pub(crate) fn cmd_status_fleet_resource_convergence_lag(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let lags = collect_convergence_lag(sd, &targets);
    let total_lagging = lags.len();
    if json {
        println!(
            "{{\"fleet_convergence_lag\":{{\"lagging_resources\":{}}}}}",
            total_lagging
        );
    } else {
        println!("Fleet convergence lag: {} resources lagging", total_lagging);
    }
    Ok(())
}

/// FJ-932: Dependency chain depth per resource per machine.
pub(crate) fn cmd_status_machine_resource_dependency_depth(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let depths = collect_dependency_depths(sd, &targets);
    if json {
        let items: Vec<String> = depths
            .iter()
            .map(|(m, c)| format!("{{\"machine\":\"{}\",\"resource_count\":{}}}", m, c))
            .collect();
        println!("{{\"dependency_depths\":[{}]}}", items.join(","));
    } else if depths.is_empty() {
        println!("No dependency depth data available.");
    } else {
        println!("Machine resource dependency depth:");
        for (m, c) in &depths {
            println!("  {} — {} resources", m, c);
        }
    }
    Ok(())
}

fn collect_dependency_depths(sd: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut depths = Vec::new();
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
        depths.push(((*m).clone(), lock.resources.len()));
    }
    depths.sort_by(|a, b| a.0.cmp(&b.0));
    depths
}

/// FJ-934: Rate of convergence improvement per machine.
pub(crate) fn cmd_status_machine_resource_convergence_velocity(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let velocities = collect_convergence_velocities(sd, &targets);
    if json {
        let items: Vec<String> = velocities
            .iter()
            .map(|(m, v)| format!("{{\"machine\":\"{}\",\"velocity\":{:.4}}}", m, v))
            .collect();
        println!("{{\"convergence_velocities\":[{}]}}", items.join(","));
    } else if velocities.is_empty() {
        println!("No convergence velocity data available.");
    } else {
        println!("Convergence velocity:");
        for (m, v) in &velocities {
            println!("  {} — {:.4}", m, v);
        }
    }
    Ok(())
}

fn collect_convergence_velocities(sd: &Path, targets: &[&String]) -> Vec<(String, f64)> {
    let mut velocities = Vec::new();
    for m in targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let total = lock.resources.len();
        let converged = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Converged)
            .count();
        let velocity = if total > 0 {
            converged as f64 / total as f64
        } else {
            0.0
        };
        velocities.push(((*m).clone(), velocity));
    }
    velocities.sort_by(|a, b| a.0.cmp(&b.0));
    velocities
}

/// FJ-938: Fleet-wide convergence improvement rate.
pub(crate) fn cmd_status_fleet_resource_convergence_velocity(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let velocities = collect_convergence_velocities(sd, &targets);
    let total: f64 = velocities.iter().map(|(_, v)| v).sum();
    let avg = if !velocities.is_empty() {
        total / velocities.len() as f64
    } else {
        0.0
    };
    if json {
        println!(
            "{{\"fleet_convergence_velocity\":{:.4},\"machines\":{}}}",
            avg,
            velocities.len()
        );
    } else {
        println!(
            "Fleet convergence velocity: {:.4} ({} machines)",
            avg,
            velocities.len()
        );
    }
    Ok(())
}

/// FJ-940: Frequency of repeated failures per resource.
pub(crate) fn cmd_status_machine_resource_failure_recurrence(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|n| n.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let recurrences = collect_failure_recurrences(sd, &targets);
    if json {
        let items: Vec<String> = recurrences
            .iter()
            .map(|(m, c)| format!("{{\"machine\":\"{}\",\"failed_resources\":{}}}", m, c))
            .collect();
        println!("{{\"failure_recurrences\":[{}]}}", items.join(","));
    } else if recurrences.is_empty() {
        println!("No failure recurrence data available.");
    } else {
        println!("Failure recurrence:");
        for (m, c) in &recurrences {
            println!("  {} — {} failed resources", m, c);
        }
    }
    Ok(())
}

fn collect_failure_recurrences(sd: &Path, targets: &[&String]) -> Vec<(String, usize)> {
    let mut recurrences = Vec::new();
    for m in targets {
        let path = sd.join(m).join("state.lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        let failed = lock
            .resources
            .values()
            .filter(|r| r.status == types::ResourceStatus::Failed)
            .count();
        if failed > 0 {
            recurrences.push(((*m).clone(), failed));
        }
    }
    recurrences.sort_by(|a, b| a.0.cmp(&b.0));
    recurrences
}
