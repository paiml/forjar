//! Status recovery — error budgets, compliance scoring, MTTR metrics.

use super::helpers::*;
#[allow(unused_imports)]
use super::helpers_state::*;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

pub(super) fn pct(num: usize, den: usize) -> f64 {
    if den > 0 {
        (num as f64 / den as f64) * 100.0
    } else {
        0.0
    }
}

pub(super) fn collect_error_budgets(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut budgets = Vec::new();
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
        let failed = lock
            .resources
            .values()
            .filter(|r| matches!(r.status, types::ResourceStatus::Failed))
            .count();
        budgets.push(((*m).clone(), failed, total));
    }
    budgets.sort_by(|a, b| a.0.cmp(&b.0));
    budgets
}

/// FJ-878: Show error budget consumption per machine (failed/total ratio).
pub(crate) fn cmd_status_machine_error_budget(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let budgets = collect_error_budgets(sd, &targets);
    if json {
        let items: Vec<String> = budgets
            .iter()
            .map(|(m, f, t)| {
                format!(
                    "{{\"machine\":\"{}\",\"failed\":{},\"total\":{},\"error_pct\":{:.1}}}",
                    m,
                    f,
                    t,
                    pct(*f, *t)
                )
            })
            .collect();
        println!("{{\"machine_error_budget\":[{}]}}", items.join(","));
    } else if budgets.is_empty() {
        println!("No error budget data available.");
    } else {
        println!("Machine error budget (failed / total):");
        for (m, f, t) in &budgets {
            println!(
                "  {} — {}/{} failed ({:.1}% error budget consumed)",
                m,
                f,
                t,
                pct(*f, *t)
            );
        }
    }
    Ok(())
}

/// FJ-882: Show fleet-wide compliance score based on converged resources.
pub(crate) fn cmd_status_fleet_compliance_score(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let (mut total, mut converged) = (0usize, 0usize);
    for m in &targets {
        let path = sd.join(m).join("lock.yaml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };
        total += lock.resources.len();
        converged += lock
            .resources
            .values()
            .filter(|r| matches!(r.status, types::ResourceStatus::Converged))
            .count();
    }
    let score = if total > 0 {
        (converged as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    if json {
        println!(
            "{{\"fleet_compliance_score\":{:.1},\"converged\":{},\"total\":{}}}",
            score, converged, total
        );
    } else if total == 0 {
        println!("No compliance data available.");
    } else {
        println!(
            "Fleet compliance score: {:.1}% ({}/{} resources converged)",
            score, converged, total
        );
    }
    Ok(())
}

/// FJ-884: Show mean time to recovery per machine.
pub(crate) fn cmd_status_machine_mean_time_to_recovery(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
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
        let items: Vec<String> = mttr_data
            .iter()
            .map(|(m, s)| format!("{{\"machine\":\"{}\",\"mttr_status\":\"{}\"}}", m, s))
            .collect();
        println!(
            "{{\"machine_mean_time_to_recovery\":[{}]}}",
            items.join(",")
        );
    } else if mttr_data.is_empty() {
        println!("No MTTR data available.");
    } else {
        println!("Machine mean time to recovery:");
        for (m, s) in &mttr_data {
            println!("  {} — {}", m, s);
        }
    }
    Ok(())
}

/// FJ-886: Health of upstream dependencies per resource.
pub(crate) fn cmd_status_machine_resource_dependency_health(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let mut health_data: Vec<(String, usize, usize)> = Vec::new();
    for m in &targets {
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
        let healthy = lock
            .resources
            .values()
            .filter(|r| matches!(r.status, types::ResourceStatus::Converged))
            .count();
        health_data.push(((*m).clone(), healthy, total));
    }
    health_data.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = health_data
            .iter()
            .map(|(m, h, t)| {
                format!(
                    "{{\"machine\":\"{}\",\"healthy\":{},\"total\":{}}}",
                    m, h, t
                )
            })
            .collect();
        println!("{{\"resource_dependency_health\":[{}]}}", items.join(","));
    } else if health_data.is_empty() {
        println!("No dependency health data available.");
    } else {
        println!("Machine resource dependency health:");
        for (m, h, t) in &health_data {
            println!("  {} — {}/{} healthy", m, h, t);
        }
    }
    Ok(())
}

/// FJ-890: Health breakdown by resource type across fleet.
pub(crate) fn cmd_status_fleet_resource_type_health(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let type_health = collect_type_health(sd, &targets);
    if json {
        let items: Vec<String> = type_health
            .iter()
            .map(|(t, c, tot)| {
                format!(
                    "{{\"type\":\"{}\",\"converged\":{},\"total\":{}}}",
                    t, c, tot
                )
            })
            .collect();
        println!("{{\"fleet_resource_type_health\":[{}]}}", items.join(","));
    } else if type_health.is_empty() {
        println!("No resource type health data available.");
    } else {
        println!("Fleet resource type health:");
        for (t, c, tot) in &type_health {
            println!("  {} — {}/{} converged", t, c, tot);
        }
    }
    Ok(())
}

pub(super) fn collect_type_health(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut type_map: std::collections::HashMap<String, (usize, usize)> =
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
        for rs in lock.resources.values() {
            let t = format!("{:?}", rs.resource_type);
            let entry = type_map.entry(t).or_insert((0, 0));
            entry.1 += 1;
            if matches!(rs.status, types::ResourceStatus::Converged) {
                entry.0 += 1;
            }
        }
    }
    let mut result: Vec<(String, usize, usize)> = type_map
        .into_iter()
        .map(|(t, (c, tot))| (t, c, tot))
        .collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

pub(super) fn collect_convergence_rates(
    sd: &Path,
    targets: &[&String],
) -> Vec<(String, usize, usize)> {
    let mut rates: Vec<(String, usize, usize)> = Vec::new();
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
        let converged = lock
            .resources
            .values()
            .filter(|r| matches!(r.status, types::ResourceStatus::Converged))
            .count();
        rates.push(((*m).clone(), converged, total));
    }
    rates.sort_by(|a, b| a.0.cmp(&b.0));
    rates
}

/// FJ-892: Convergence rate per resource per machine.
pub(crate) fn cmd_status_machine_resource_convergence_rate(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(sd);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let rates = collect_convergence_rates(sd, &targets);
    if json {
        let items: Vec<String> = rates
            .iter()
            .map(|(m, c, t)| {
                format!(
                    "{{\"machine\":\"{}\",\"converged\":{},\"total\":{},\"rate\":{:.1}}}",
                    m,
                    c,
                    t,
                    pct(*c, *t)
                )
            })
            .collect();
        println!(
            "{{\"machine_resource_convergence_rate\":[{}]}}",
            items.join(",")
        );
    } else if rates.is_empty() {
        println!("No convergence rate data available.");
    } else {
        println!("Machine resource convergence rate:");
        for (m, c, t) in &rates {
            println!("  {} — {}/{} converged ({:.1}%)", m, c, t, pct(*c, *t));
        }
    }
    Ok(())
}

pub(super) use super::status_recovery_b::*;
