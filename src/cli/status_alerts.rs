//! Alerts and uptime.

use crate::core::{state, types};
use std::path::Path;
use super::helpers::*;


// ── FJ-457: status --alerts ──

fn collect_alerts_from_lock(m_name: &str, lock: &types::StateLock, alerts: &mut Vec<(String, String, String)>) {
    for (rname, rl) in &lock.resources {
        if matches!(rl.status, types::ResourceStatus::Failed | types::ResourceStatus::Drifted) {
            alerts.push((m_name.to_string(), rname.clone(), format!("{:?}", rl.status)));
        }
    }
}

fn collect_alerts(
    state_dir: &Path,
    machine: Option<&str>,
) -> Result<Vec<(String, String, String)>, String> {
    let mut alerts = Vec::new();
    if !state_dir.exists() {
        return Ok(alerts);
    }
    let entries = std::fs::read_dir(state_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let m_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if m_name.starts_with('.') {
            continue;
        }
        if let Some(filter) = machine {
            if m_name != filter {
                continue;
            }
        }
        if let Ok(Some(lock)) = state::load_lock(state_dir, &m_name) {
            collect_alerts_from_lock(&m_name, &lock, &mut alerts);
        }
    }
    Ok(alerts)
}

pub(crate) fn cmd_status_alerts(state_dir: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let alerts = collect_alerts(state_dir, machine)?;

    if json {
        let items: Vec<String> = alerts
            .iter()
            .map(|(m, r, s)| {
                format!(
                    "{{\"machine\":\"{}\",\"resource\":\"{}\",\"status\":\"{}\"}}",
                    m, r, s
                )
            })
            .collect();
        println!("[{}]", items.join(","));
    } else if alerts.is_empty() {
        println!("{} No alerts — all resources healthy.", green("✓"));
    } else {
        println!("{} {} alert(s):", red("⚠"), alerts.len());
        for (m, r, s) in &alerts {
            println!("  {}/{}: {}", m, r, s);
        }
    }
    Ok(())
}


/// FJ-642: Show resource uptime based on convergence history
fn collect_uptime_entries(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, String, String, String)> {
    let mut entries = Vec::new();
    for m in targets {
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
            let status_str = format!("{:?}", rlock.status);
            let applied = rlock.applied_at.clone().unwrap_or_default();
            entries.push((m.to_string(), rname.clone(), status_str, applied));
        }
    }
    entries
}

pub(crate) fn cmd_status_uptime(state_dir: &Path, machine: Option<&str>, json: bool) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = if let Some(m) = machine {
        machines.iter().filter(|x| x.as_str() == m).collect()
    } else {
        machines.iter().collect()
    };

    let entries = collect_uptime_entries(state_dir, &targets);

    if json {
        print!("{{\"uptime\":[");
        for (i, (m, rname, status_str, applied)) in entries.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(
                r#"{{"machine":"{}","resource":"{}","status":"{}","since":"{}"}}"#,
                m, rname, status_str, applied
            );
        }
        println!("]}}");
    } else {
        for (m, rname, status_str, applied) in &entries {
            println!(
                "{}/{}: {} (since {})",
                m,
                rname,
                status_str,
                if applied.is_empty() { "unknown" } else { applied }
            );
        }
    }
    Ok(())
}


/// FJ-632: Comprehensive diagnostic report with recommendations.
fn collect_diagnostic_stats(
    state_dir: &Path,
    machines: &[String],
    machine: Option<&str>,
) -> (u64, u64, u64, u64, u64) {
    let mut total_resources = 0u64;
    let mut converged = 0u64;
    let mut failed = 0u64;
    let mut drifted = 0u64;
    let mut machine_count = 0u64;

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
        machine_count += 1;
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            for (_, rlock) in &lock.resources {
                total_resources += 1;
                match rlock.status {
                    crate::core::types::ResourceStatus::Converged => converged += 1,
                    crate::core::types::ResourceStatus::Failed => failed += 1,
                    crate::core::types::ResourceStatus::Drifted => drifted += 1,
                    _ => {}
                }
            }
        }
    }
    (machine_count, total_resources, converged, failed, drifted)
}

pub(crate) fn cmd_status_diagnostic(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let (machine_count, total_resources, converged, failed, drifted) =
        collect_diagnostic_stats(state_dir, &machines, machine);

    let health = if total_resources > 0 {
        (converged as f64 / total_resources as f64 * 100.0).clamp(0.0, 100.0)
    } else {
        100.0
    };

    if json {
        println!(
            r#"{{"machines":{},"resources":{},"converged":{},"failed":{},"drifted":{},"health":{:.1}}}"#,
            machine_count, total_resources, converged, failed, drifted, health
        );
    } else {
        println!("Diagnostic Report");
        println!("  Machines: {}", machine_count);
        println!(
            "  Resources: {} (converged: {}, failed: {}, drifted: {})",
            total_resources, converged, failed, drifted
        );
        println!("  Health: {:.1}%", health);
        if failed > 0 {
            println!(
                "  Recommendation: Run 'forjar status --error-summary' to investigate failures"
            );
        }
        if drifted > 0 {
            println!("  Recommendation: Run 'forjar drift' to detect unauthorized changes");
        }
    }
    Ok(())
}


// ── FJ-502: status --sla-report ──

fn process_sla_machine(
    lock: &types::StateLock,
    m: &str,
    json: bool,
    reports: &mut Vec<serde_json::Value>,
) {
    let total = lock.resources.len();
    let converged = lock
        .resources
        .values()
        .filter(|r| r.status == types::ResourceStatus::Converged)
        .count();
    let sla_pct = if total > 0 {
        (converged as f64 / total as f64 * 100.0).round()
    } else {
        100.0
    };
    let meets_sla = sla_pct >= 99.0;
    if json {
        reports.push(serde_json::json!({"machine": m, "sla_pct": sla_pct, "meets_sla": meets_sla, "converged": converged, "total": total}));
    } else {
        let indicator = if meets_sla { green("✓") } else { red("✗") };
        println!(
            "{} {} — SLA: {:.1}% ({}/{}) {}",
            indicator, m, sla_pct, converged, total,
            if meets_sla { "" } else { "⚠ BELOW SLA" }
        );
    }
}

pub(crate) fn cmd_status_sla_report(
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
    let mut reports: Vec<serde_json::Value> = Vec::new();
    for m in &machines {
        if let Some(lock) = state::load_lock(state_dir, m).map_err(|e| e.to_string())? {
            process_sla_machine(&lock, m, json, &mut reports);
        }
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"sla_report": reports}))
                .unwrap_or_default()
        );
    }
    Ok(())
}


// ── FJ-477: status --dependency-health ──

fn compute_dependency_scores(
    state_dir: &Path,
    machines: &[String],
) -> Result<Vec<(String, String, f64)>, String> {
    let mut all_resources = Vec::new();
    for m in machines {
        if let Some(lock) = state::load_lock(state_dir, m).map_err(|e| e.to_string())? {
            let total = lock.resources.len() as f64;
            for (idx, (rname, rl)) in lock.resources.iter().enumerate() {
                let position_weight = 1.0 + ((total - idx as f64) / total.max(1.0));
                let base_score = match rl.status {
                    types::ResourceStatus::Converged => 100.0,
                    types::ResourceStatus::Unknown => 50.0,
                    types::ResourceStatus::Drifted => 25.0,
                    types::ResourceStatus::Failed => 0.0,
                };
                let weighted = base_score * position_weight / 2.0;
                all_resources.push((m.clone(), rname.clone(), weighted));
            }
        }
    }
    Ok(all_resources)
}

pub(crate) fn cmd_status_dependency_health(
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
    let all_resources = compute_dependency_scores(state_dir, &machines)?;
    let total_score: f64 = all_resources.iter().map(|(_, _, s)| s).sum();
    let max_possible: f64 = all_resources.iter().map(|(_, _, _)| 100.0).sum();
    let health_pct = if max_possible > 0.0 {
        (total_score / max_possible * 100.0).round()
    } else {
        100.0
    };
    if json {
        let result = serde_json::json!({
            "dependency_health_score": health_pct,
            "resource_count": all_resources.len(),
            "weighted_total": total_score,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else {
        let indicator = if health_pct >= 80.0 {
            green("✓")
        } else if health_pct >= 50.0 {
            yellow("⚠")
        } else {
            red("✗")
        };
        println!(
            "{} Dependency-weighted health score: {:.0}%",
            indicator, health_pct
        );
        println!(
            "  Resources: {}, Weighted total: {:.1}/{:.1}",
            all_resources.len(),
            total_score,
            max_possible
        );
    }
    Ok(())
}

