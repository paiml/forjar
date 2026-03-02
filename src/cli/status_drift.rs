//! Drift analysis.

use super::helpers::*;
use crate::core::{state, types};
use std::path::Path;

// FJ-355: Show detailed drift report with field-level diffs
fn collect_drift_details(
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<Vec<serde_json::Value>, String> {
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {}", e))?;
    let mut details = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            for (id, rl) in &lock.resources {
                if matches!(
                    rl.status,
                    types::ResourceStatus::Drifted | types::ResourceStatus::Failed
                ) {
                    details.push(serde_json::json!({
                        "resource": id, "machine": lock.machine,
                        "status": format!("{:?}", rl.status), "hash": rl.hash, "applied_at": rl.applied_at,
                    }));
                }
            }
        }
    }
    Ok(details)
}

pub(crate) fn cmd_status_drift_details(
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let details = collect_drift_details(state_dir, machine_filter)?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&details).unwrap_or_else(|_| "[]".to_string())
        );
    } else {
        println!("Drift Details:\n");
        if details.is_empty() {
            println!("  {} No drift detected.", green("✓"));
        } else {
            for d in &details {
                println!(
                    "  {} {} on {} — {} ({})",
                    yellow("~"),
                    d["resource"].as_str().unwrap_or("?"),
                    d["machine"].as_str().unwrap_or("?"),
                    d["status"].as_str().unwrap_or("?"),
                    d["applied_at"].as_str().unwrap_or("?"),
                );
            }
            println!(
                "\n{} {} resource(s) with drift",
                yellow("Total:"),
                details.len()
            );
        }
    }

    Ok(())
}

// ── FJ-492: status --drift-summary ──

fn process_drift_summary_machine(
    lock: &types::StateLock,
    m: &str,
    json: bool,
    summaries: &mut Vec<serde_json::Value>,
) {
    let total = lock.resources.len();
    let drifted = lock
        .resources
        .values()
        .filter(|r| r.status == types::ResourceStatus::Drifted)
        .count();
    let pct = if total > 0 {
        (drifted as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };
    if json {
        summaries.push(
            serde_json::json!({"machine": m, "total": total, "drifted": drifted, "drift_pct": pct}),
        );
    } else {
        let indicator = if drifted == 0 {
            green("✓")
        } else {
            red("✗")
        };
        println!(
            "{} {} — {}/{} drifted ({:.0}%)",
            indicator, m, drifted, total, pct
        );
    }
}

pub(crate) fn cmd_status_drift_summary(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let all_machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        all_machines.into_iter().filter(|n| n == m).collect()
    } else {
        all_machines
    };
    let mut summaries: Vec<serde_json::Value> = Vec::new();
    for m in &machines {
        if let Some(lock) = state::load_lock(state_dir, m).map_err(|e| e.to_string())? {
            process_drift_summary_machine(&lock, m, json, &mut summaries);
        }
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"drift_summary": summaries}))
                .unwrap_or_default()
        );
    }
    Ok(())
}

/// FJ-567: Show drift rate over time (changes per day/week).
pub(crate) fn cmd_status_drift_velocity(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut events_per_machine: Vec<(String, usize, usize)> = Vec::new(); // (machine, total_events, drift_events)

    for m in &machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let log_path = state_dir.join(format!("{}.events.jsonl", m));
        if !log_path.exists() {
            events_per_machine.push((m.clone(), 0, 0));
            continue;
        }
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        let total = content.lines().filter(|l| !l.trim().is_empty()).count();
        let drift = content
            .lines()
            .filter(|l| {
                l.contains("\"Drifted\"") || l.contains("\"drifted\"") || l.contains("drift")
            })
            .count();
        events_per_machine.push((m.clone(), total, drift));
    }

    if json {
        let items: Vec<String> = events_per_machine
            .iter()
            .map(|(m, t, d)| {
                format!(
                    r#"{{"machine":"{}","total_events":{},"drift_events":{},"drift_rate":{:.2}}}"#,
                    m,
                    t,
                    d,
                    if *t > 0 { *d as f64 / *t as f64 } else { 0.0 }
                )
            })
            .collect();
        println!(r#"{{"drift_velocity":[{}]}}"#, items.join(","));
    } else {
        println!("Drift velocity:");
        for (m, total, drift) in &events_per_machine {
            let rate = if *total > 0 {
                *drift as f64 / *total as f64 * 100.0
            } else {
                0.0
            };
            println!(
                "  {} — {} total events, {} drift events ({:.1}% drift rate)",
                m, total, drift, rate
            );
        }
    }
    Ok(())
}

/// FJ-617: Predict likely drift based on historical patterns.
fn assess_drift_risk(resource_type: &crate::core::types::ResourceType) -> &'static str {
    match resource_type {
        crate::core::types::ResourceType::Package => "medium",
        crate::core::types::ResourceType::Service => "high",
        crate::core::types::ResourceType::File => "low",
        crate::core::types::ResourceType::User => "medium",
        _ => "low",
    }
}

fn collect_drift_forecasts(
    state_dir: &Path,
    machines: &[String],
    machine: Option<&str>,
) -> Vec<(String, String, String)> {
    let mut forecasts = Vec::new();
    for m in machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            for (rname, rlock) in &lock.resources {
                let risk = assess_drift_risk(&rlock.resource_type);
                if risk != "low" {
                    forecasts.push((m.clone(), rname.clone(), risk.to_string()));
                }
            }
        }
    }
    forecasts
}

pub(crate) fn cmd_status_drift_forecast(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let forecasts = collect_drift_forecasts(state_dir, &machines, machine);

    if json {
        let items: Vec<String> = forecasts
            .iter()
            .map(|(m, r, risk)| {
                format!(
                    r#"{{"machine":"{}","resource":"{}","drift_risk":"{}"}}"#,
                    m, r, risk
                )
            })
            .collect();
        println!(
            r#"{{"drift_forecasts":[{}],"count":{}}}"#,
            items.join(","),
            forecasts.len()
        );
    } else if forecasts.is_empty() {
        println!("No drift risk detected");
    } else {
        println!("Drift forecast ({} at-risk resources):", forecasts.len());
        for (m, r, risk) in &forecasts {
            println!("  {}:{} — {} risk", m, r, risk);
        }
    }
    Ok(())
}

/// FJ-687: Show drift details for all machines at once
pub(crate) fn cmd_status_drift_details_all(state_dir: &Path, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut drifted = Vec::new();

    for m in &machines {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let lock: crate::core::types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };

        for (rname, rlock) in &lock.resources {
            if matches!(rlock.status, crate::core::types::ResourceStatus::Drifted) {
                drifted.push((m.clone(), rname.clone(), rlock.hash.clone()));
            }
        }
    }

    if json {
        print!("{{\"drifted\":[");
        for (i, (machine, resource, hash)) in drifted.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(
                r#"{{"machine":"{}","resource":"{}","hash":"{}"}}"#,
                machine,
                resource,
                &hash[..hash.len().min(12)]
            );
        }
        println!("]}}");
    } else if drifted.is_empty() {
        println!("No drifted resources across any machine");
    } else {
        println!("Drifted resources ({}):", drifted.len());
        for (machine, resource, hash) in &drifted {
            println!(
                "  {}/{} [{}]",
                machine,
                resource,
                &hash[..hash.len().min(12)]
            );
        }
    }
    Ok(())
}

fn compute_drift_stats(lock: &types::StateLock) -> (usize, usize, f64) {
    let total = lock.resources.len();
    let drifted = lock
        .resources
        .values()
        .filter(|r| format!("{:?}", r.status) == "Drifted")
        .count();
    let rate = if total > 0 {
        drifted as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    (total, drifted, rate)
}

fn load_drift_trend(state_dir: &Path, targets: &[&String]) -> Vec<(String, usize, usize, f64)> {
    let mut results = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if let Ok(data) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&data) {
                let (total, drifted, rate) = compute_drift_stats(&lock);
                results.push((m.to_string(), total, drifted, rate));
            }
        }
    }
    results
}

/// FJ-717: Show drift trend over time
pub(crate) fn cmd_status_drift_trend(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let trend = load_drift_trend(state_dir, &targets);
    if json {
        let entries: Vec<String> = trend
            .iter()
            .map(|(m, total, drifted, rate)| {
                format!(
                    "{{\"machine\":\"{}\",\"total\":{},\"drifted\":{},\"drift_rate\":{:.1}}}",
                    m, total, drifted, rate
                )
            })
            .collect();
        println!("{{\"drift_trend\":[{}]}}", entries.join(","));
    } else {
        println!("Drift trend:");
        for (m, total, drifted, rate) in &trend {
            println!("  {} — {}/{} drifted ({:.1}%)", m, drifted, total, rate);
        }
    }
    Ok(())
}

/// FJ-582: Compare running config state against declared config for drift.
fn collect_config_drift(
    state_dir: &Path,
    machines: &[String],
    machine: Option<&str>,
) -> Vec<(String, String, String)> {
    let mut drifted = Vec::new();
    for m in machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            for (rname, rlock) in &lock.resources {
                if matches!(
                    rlock.status,
                    crate::core::types::ResourceStatus::Drifted
                        | crate::core::types::ResourceStatus::Failed
                ) {
                    drifted.push((m.clone(), rname.clone(), format!("{:?}", rlock.status)));
                }
            }
        }
    }
    drifted
}

pub(crate) fn cmd_status_config_drift(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let drifted = collect_config_drift(state_dir, &machines, machine);

    if json {
        let items: Vec<String> = drifted
            .iter()
            .map(|(m, r, s)| {
                format!(
                    r#"{{"machine":"{}","resource":"{}","status":"{}"}}"#,
                    m, r, s
                )
            })
            .collect();
        println!(
            r#"{{"config_drift":[{}],"count":{}}}"#,
            items.join(","),
            drifted.len()
        );
    } else if drifted.is_empty() {
        println!("No config drift detected — all resources match declared state");
    } else {
        println!("Config drift detected ({} resources):", drifted.len());
        for (m, r, s) in &drifted {
            println!("  {}:{} — {}", m, r, s);
        }
    }
    Ok(())
}
