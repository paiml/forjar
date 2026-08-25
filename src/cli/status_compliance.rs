//! Compliance status.

use super::helpers::*;
use crate::core::{state, types};
use std::path::Path;

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
            policy,
            total,
            compliant,
            violations.len(),
            pass
        );
    } else if pass {
        println!(
            "{} Compliance '{}': {}/{} resources compliant.",
            green("✓"),
            policy,
            compliant,
            total
        );
    } else {
        println!(
            "{} Compliance '{}': {} violation(s):",
            red("✗"),
            policy,
            violations.len()
        );
        for v in &violations {
            println!("  - {v}");
        }
    }
    if pass {
        Ok(())
    } else {
        Err(format!(
            "Compliance check '{}' failed: {} violations",
            policy,
            violations.len()
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

fn print_compliance_text(
    findings: &[serde_json::Value],
    policy: &str,
    compliance_pct: f64,
    compliant_count: usize,
    total: usize,
) {
    let indicator = if compliance_pct >= 100.0 {
        green("✓")
    } else {
        yellow("⚠")
    };
    println!(
        "{indicator} Compliance report for '{policy}': {compliance_pct:.0}% ({compliant_count}/{total})"
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

/// Resource types whose recorded state carries security meaning — permissions,
/// ownership, exposure.
fn is_security_relevant(resource_type: &types::ResourceType) -> bool {
    matches!(
        resource_type,
        types::ResourceType::File
            | types::ResourceType::User
            | types::ResourceType::Network
            | types::ResourceType::Service
    )
}

/// Appends `(machine, resource, type, status)` for every security-relevant
/// entry in one machine's lock.
fn push_security_resources(
    m: &str,
    lock: &types::StateLock,
    items: &mut Vec<(String, String, String, String)>,
) {
    for (rname, rlock) in &lock.resources {
        if is_security_relevant(&rlock.resource_type) {
            items.push((
                m.to_string(),
                rname.clone(),
                format!("{:?}", rlock.resource_type),
                format!("{:?}", rlock.status),
            ));
        }
    }
}

/// Security-relevant resources across the selected machines. A machine with no
/// lock, or a lock that will not parse, contributes nothing.
fn collect_security_resources(
    state_dir: &Path,
    machines: &[String],
    machine: Option<&str>,
) -> Vec<(String, String, String, String)> {
    let mut items = Vec::new();
    for m in machines {
        if machine.is_some_and(|filter| m != filter) {
            continue;
        }
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
            push_security_resources(m, &lock, &mut items);
        }
    }
    items
}

/// Renders the security posture as JSON, as a per-resource listing, or as the
/// "nothing relevant" line.
fn print_security_posture(items: &[(String, String, String, String)], json: bool) {
    if json {
        let json_items: Vec<String> = items
            .iter()
            .map(|(m, r, t, s)| {
                format!(r#"{{"machine":"{m}","resource":"{r}","type":"{t}","status":"{s}"}}"#)
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
        for (m, r, t, s) in items {
            println!("  {m}:{r} ({t}) — {s}");
        }
    }
}

/// FJ-602: Show security-relevant resource states (modes, ownership).
pub(crate) fn cmd_status_security_posture(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let items = collect_security_resources(state_dir, &machines, machine);
    print_security_posture(&items, json);
    Ok(())
}

/// One audit row — `(machine, resource, status, timestamp)` — from a single
/// event-log line. `None` for a blank line or one that is not JSON; a field the
/// event omits reads as "unknown" rather than dropping the row.
fn audit_entry_from_line(m: &str, line: &str) -> Option<(String, String, String, String)> {
    if line.trim().is_empty() {
        return None;
    }
    let val = serde_json::from_str::<serde_json::Value>(line).ok()?;
    let field = |key: &str| {
        val.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string()
    };
    Some((
        m.to_string(),
        field("resource"),
        field("status"),
        field("timestamp"),
    ))
}

/// FJ-552: Full audit trail from event logs — who/what/when for each change.
fn collect_audit_entries(
    state_dir: &Path,
    machines: &[String],
    machine: Option<&str>,
) -> Vec<(String, String, String, String)> {
    let mut entries = Vec::new();
    for m in machines {
        if machine.is_some_and(|filter| m != filter) {
            continue;
        }
        let log_path = state_dir.join(format!("{m}.events.jsonl"));
        if !log_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        entries.extend(
            content
                .lines()
                .filter_map(|line| audit_entry_from_line(m, line)),
        );
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
                format!(r#"{{"machine":"{m}","resource":"{r}","status":"{s}","timestamp":"{t}"}}"#)
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
            println!("  {t} | {m} | {r} | {s}");
        }
    }
    Ok(())
}
