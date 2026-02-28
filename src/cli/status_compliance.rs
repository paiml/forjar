//! Compliance status.

use crate::core::{state, types};
use std::path::Path;
use super::helpers::*;


// ── FJ-467: status --compliance ──

fn tally_lock_compliance(
    m_name: &str,
    lock: &types::StateLock,
    total: &mut usize,
    compliant: &mut usize,
    violations: &mut Vec<String>,
) {
    for (rname, rl) in &lock.resources {
        *total += 1;
        if rl.status == types::ResourceStatus::Converged {
            *compliant += 1;
        } else {
            violations.push(format!("{}/{}: {:?}", m_name, rname, rl.status));
        }
    }
}

fn check_compliance(
    state_dir: &Path,
    machine: Option<&str>,
) -> Result<(usize, usize, Vec<String>), String> {
    let mut total = 0usize;
    let mut compliant = 0usize;
    let mut violations = Vec::new();
    if !state_dir.exists() {
        return Ok((total, compliant, violations));
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
            tally_lock_compliance(&m_name, &lock, &mut total, &mut compliant, &mut violations);
        }
    }
    Ok((total, compliant, violations))
}

pub(crate) fn cmd_status_compliance(
    state_dir: &Path,
    machine: Option<&str>,
    policy: &str,
    json: bool,
) -> Result<(), String> {
    let (total, compliant, violations) = check_compliance(state_dir, machine)?;

    let pass = violations.is_empty();
    if json {
        println!(
            "{{\"policy\":\"{}\",\"total\":{},\"compliant\":{},\"violations\":{},\"pass\":{}}}",
            policy, total, compliant, violations.len(), pass
        );
    } else if pass {
        println!(
            "{} Compliance '{}': {}/{} resources compliant.",
            green("✓"), policy, compliant, total
        );
    } else {
        println!(
            "{} Compliance '{}': {} violation(s):",
            red("✗"), policy, violations.len()
        );
        for v in &violations {
            println!("  - {}", v);
        }
    }
    if pass {
        Ok(())
    } else {
        Err(format!(
            "Compliance check '{}' failed: {} violations",
            policy, violations.len()
        ))
    }
}


// ── FJ-507: status --compliance-report ──

fn collect_compliance_findings(
    state_dir: &Path,
    machines: &[String],
    policy: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let mut findings = Vec::new();
    for m in machines {
        if let Some(lock) = state::load_lock(state_dir, m).map_err(|e| e.to_string())? {
            for (rname, rl) in &lock.resources {
                let compliant = rl.status == types::ResourceStatus::Converged;
                findings.push(serde_json::json!({
                    "machine": m,
                    "resource": rname,
                    "status": format!("{:?}", rl.status),
                    "compliant": compliant,
                    "policy": policy,
                }));
            }
        }
    }
    Ok(findings)
}

fn print_compliance_text(findings: &[serde_json::Value], policy: &str, compliance_pct: f64, compliant_count: usize, total: usize) {
    let indicator = if compliance_pct >= 100.0 { green("✓") } else { yellow("⚠") };
    println!(
        "{} Compliance report for '{}': {:.0}% ({}/{})",
        indicator, policy, compliance_pct, compliant_count, total
    );
    for f in findings {
        if !f["compliant"].as_bool().unwrap_or(true) {
            println!(
                "  {} {}:{} — {}",
                red("✗"),
                f["machine"].as_str().unwrap_or("?"),
                f["resource"].as_str().unwrap_or("?"),
                f["status"].as_str().unwrap_or("?")
            );
        }
    }
}

pub(crate) fn cmd_status_compliance_report(
    state_dir: &Path,
    machine: Option<&str>,
    policy: &str,
    json: bool,
) -> Result<(), String> {
    let all_machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        all_machines.into_iter().filter(|n| n == m).collect()
    } else {
        all_machines
    };
    let findings = collect_compliance_findings(state_dir, &machines, policy)?;
    let total = findings.len();
    let compliant_count = findings
        .iter()
        .filter(|f| f["compliant"].as_bool().unwrap_or(false))
        .count();
    let compliance_pct = if total > 0 {
        (compliant_count as f64 / total as f64 * 100.0).round()
    } else {
        100.0
    };
    if json {
        let result = serde_json::json!({
            "policy": policy,
            "compliance_pct": compliance_pct,
            "compliant": compliant_count,
            "total": total,
            "findings": findings,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else {
        print_compliance_text(&findings, policy, compliance_pct, compliant_count, total);
    }
    Ok(())
}


/// FJ-602: Show security-relevant resource states (modes, ownership).
pub(crate) fn cmd_status_security_posture(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut items: Vec<(String, String, String, String)> = Vec::new(); // (machine, resource, type, status)

    for m in &machines {
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
                let rtype = format!("{:?}", rlock.resource_type);
                // Security-relevant types: file, user, network, service
                let is_security = matches!(
                    rlock.resource_type,
                    crate::core::types::ResourceType::File
                        | crate::core::types::ResourceType::User
                        | crate::core::types::ResourceType::Network
                        | crate::core::types::ResourceType::Service
                );
                if is_security {
                    let status = format!("{:?}", rlock.status);
                    items.push((m.clone(), rname.clone(), rtype, status));
                }
            }
        }
    }

    if json {
        let json_items: Vec<String> = items
            .iter()
            .map(|(m, r, t, s)| {
                format!(
                    r#"{{"machine":"{}","resource":"{}","type":"{}","status":"{}"}}"#,
                    m, r, t, s
                )
            })
            .collect();
        println!(
            r#"{{"security_resources":[{}],"count":{}}}"#,
            json_items.join(","),
            items.len()
        );
    } else if items.is_empty() {
        println!("No security-relevant resources found");
    } else {
        println!("Security posture ({} resources):", items.len());
        for (m, r, t, s) in &items {
            println!("  {}:{} ({}) — {}", m, r, t, s);
        }
    }
    Ok(())
}


/// FJ-552: Full audit trail from event logs — who/what/when for each change.
fn collect_audit_entries(
    state_dir: &Path,
    machines: &[String],
    machine: Option<&str>,
) -> Vec<(String, String, String, String)> {
    let mut entries = Vec::new();
    for m in machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let log_path = state_dir.join(format!("{}.events.jsonl", m));
        if !log_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                let resource = val.get("resource").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                let status = val.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                let timestamp = val.get("timestamp").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                entries.push((m.clone(), resource, status, timestamp));
            }
        }
    }
    entries
}

pub(crate) fn cmd_status_audit_trail(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let entries = collect_audit_entries(state_dir, &machines, machine);

    if json {
        let items: Vec<String> = entries
            .iter()
            .map(|(m, r, s, t)| {
                format!(
                    r#"{{"machine":"{}","resource":"{}","status":"{}","timestamp":"{}"}}"#,
                    m, r, s, t
                )
            })
            .collect();
        println!(
            r#"{{"audit_trail":[{}],"count":{}}}"#,
            items.join(","),
            entries.len()
        );
    } else if entries.is_empty() {
        println!("No audit trail entries found");
    } else {
        println!("Audit trail ({} entries):", entries.len());
        for (m, r, s, t) in &entries {
            println!("  {} | {} | {} | {}", t, m, r, s);
        }
    }
    Ok(())
}

