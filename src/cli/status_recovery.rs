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
        let path = sd.join(m).join("state.lock.yaml");
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
            "{{\"fleet_compliance_score\":{score:.1},\"converged\":{converged},\"total\":{total}}}"
        );
    } else if total == 0 {
        println!("No compliance data available.");
    } else {
        println!(
            "Fleet compliance score: {score:.1}% ({converged}/{total} resources converged)"
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
        let events_path = sd.join(m).join("events.jsonl");
        let content = match std::fs::read_to_string(&events_path) {
            Ok(c) => c,
            Err(_) => {
                mttr_data.push(((*m).clone(), "no event history".to_string()));
                continue;
            }
        };
        let mttr = compute_mttr_from_events(&content);
        mttr_data.push(((*m).clone(), mttr));
    }
    mttr_data.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = mttr_data
            .iter()
            .map(|(m, s)| format!("{{\"machine\":\"{m}\",\"mttr_status\":\"{s}\"}}"))
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
            println!("  {m} — {s}");
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
                    "{{\"machine\":\"{m}\",\"healthy\":{h},\"total\":{t}}}"
                )
            })
            .collect();
        println!("{{\"resource_dependency_health\":[{}]}}", items.join(","));
    } else if health_data.is_empty() {
        println!("No dependency health data available.");
    } else {
        println!("Machine resource dependency health:");
        for (m, h, t) in &health_data {
            println!("  {m} — {h}/{t} healthy");
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
                    "{{\"type\":\"{t}\",\"converged\":{c},\"total\":{tot}}}"
                )
            })
            .collect();
        println!("{{\"fleet_resource_type_health\":[{}]}}", items.join(","));
    } else if type_health.is_empty() {
        println!("No resource type health data available.");
    } else {
        println!("Fleet resource type health:");
        for (t, c, tot) in &type_health {
            println!("  {t} — {c}/{tot} converged");
        }
    }
    Ok(())
}

pub(super) fn collect_type_health(sd: &Path, targets: &[&String]) -> Vec<(String, usize, usize)> {
    let mut type_map: std::collections::HashMap<String, (usize, usize)> =
        std::collections::HashMap::new();
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

/// Parse ISO 8601 timestamp to epoch seconds.
fn parse_ts_epoch(s: &str) -> Option<f64> {
    // "2026-02-16T16:32:54Z" → epoch
    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<u32> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    let time_str = parts[1].trim_end_matches('Z');
    let time_parts: Vec<f64> = time_str.split(':').filter_map(|p| p.parse().ok()).collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return None;
    }
    // Approximate epoch: days since 1970 * 86400 + time
    let y = date_parts[0] as f64;
    let m = date_parts[1] as f64;
    let d = date_parts[2] as f64;
    let days = (y - 1970.0) * 365.25 + (m - 1.0) * 30.44 + d;
    Some(days * 86400.0 + time_parts[0] * 3600.0 + time_parts[1] * 60.0 + time_parts[2])
}

/// Extract recovery durations from events.jsonl content.
fn extract_recovery_durations(content: &str) -> Vec<f64> {
    let mut fail_times: std::collections::HashMap<String, f64> =
        std::collections::HashMap::new();
    let mut durations: Vec<f64> = Vec::new();

    for line in content.lines() {
        let val: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let event = val.get("event").and_then(|v| v.as_str()).unwrap_or("");
        let resource = val.get("resource").and_then(|v| v.as_str()).unwrap_or("");
        let ts = val
            .get("ts")
            .and_then(|v| v.as_str())
            .and_then(parse_ts_epoch);
        let ts = match ts {
            Some(t) => t,
            None => continue,
        };

        if event == "resource_failed" || event == "resource_drifted" {
            fail_times.insert(resource.to_string(), ts);
        } else if event == "resource_converged" {
            if let Some(fail_ts) = fail_times.remove(resource) {
                let d = ts - fail_ts;
                if d > 0.0 {
                    durations.push(d);
                }
            }
        }
    }
    durations
}

/// Format recovery duration as human-readable string.
fn format_recovery(avg: f64, count: usize) -> String {
    let time = if avg < 60.0 {
        format!("{avg:.1}s")
    } else if avg < 3600.0 {
        format!("{:.1}m", avg / 60.0)
    } else {
        format!("{:.1}h", avg / 3600.0)
    };
    format!("{time} avg recovery ({count} incident(s))")
}

/// Compute MTTR from events.jsonl content.
fn compute_mttr_from_events(content: &str) -> String {
    let durations = extract_recovery_durations(content);
    if durations.is_empty() {
        let total_events = content.lines().count();
        if total_events > 0 {
            format!("no failures detected ({total_events} events analyzed)")
        } else {
            "no events".to_string()
        }
    } else {
        let avg = durations.iter().sum::<f64>() / durations.len() as f64;
        format_recovery(avg, durations.len())
    }
}

pub(super) use super::status_recovery_b::*;
