//! Observability exports.

use super::helpers::*;
use crate::core::{state, types};
use std::path::Path;

// FJ-382: Prometheus exposition format
pub(crate) fn cmd_status_prometheus(
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<(), String> {
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {e}"))?;

    let mut converged = 0u64;
    let mut failed = 0u64;
    let mut drifted = 0u64;
    let mut total = 0u64;

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
            for (_, rl) in &lock.resources {
                total += 1;
                match rl.status {
                    types::ResourceStatus::Converged => converged += 1,
                    types::ResourceStatus::Failed => failed += 1,
                    types::ResourceStatus::Drifted => drifted += 1,
                    types::ResourceStatus::Unknown => {}
                }
            }
        }
    }

    println!("# HELP forjar_resources_total Total managed resources");
    println!("# TYPE forjar_resources_total gauge");
    println!("forjar_resources_total {total}");
    println!("# HELP forjar_resources_converged Converged resources");
    println!("# TYPE forjar_resources_converged gauge");
    println!("forjar_resources_converged {converged}");
    println!("# HELP forjar_resources_failed Failed resources");
    println!("# TYPE forjar_resources_failed gauge");
    println!("forjar_resources_failed {failed}");
    println!("# HELP forjar_resources_drifted Drifted resources");
    println!("# TYPE forjar_resources_drifted gauge");
    println!("forjar_resources_drifted {drifted}");

    Ok(())
}

// ── FJ-442: status --export ──

fn collect_export_entries(state_dir: &Path, machine: Option<&str>) -> Result<Vec<String>, String> {
    let mut entries = Vec::new();
    if !state_dir.exists() {
        return Ok(entries);
    }
    let dir_entries = std::fs::read_dir(state_dir).map_err(|e| e.to_string())?;
    for entry in dir_entries.flatten() {
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
            for (rname, rl) in &lock.resources {
                entries.push(format!(
                    "{{\"machine\":\"{}\",\"resource\":\"{}\",\"status\":\"{:?}\",\"hash\":\"{}\"}}",
                    m_name, rname, rl.status, rl.hash
                ));
            }
        }
    }
    Ok(entries)
}

pub(crate) fn cmd_status_export(
    state_dir: &Path,
    machine: Option<&str>,
    output_path: &Path,
    json: bool,
) -> Result<(), String> {
    let entries = collect_export_entries(state_dir, machine)?;

    let content = format!("[{}]", entries.join(",\n"));
    std::fs::write(output_path, &content).map_err(|e| e.to_string())?;

    if json {
        println!(
            "{{\"exported\":true,\"path\":\"{}\",\"entries\":{}}}",
            output_path.display(),
            entries.len()
        );
    } else {
        println!(
            "{} Exported {} entries to {}",
            green("✓"),
            entries.len(),
            output_path.display()
        );
    }
    Ok(())
}

// ── FJ-402: status --anomalies ──

fn detect_resource_anomalies(
    m_name: &str,
    name: &str,
    rl: &types::ResourceLock,
    anomalies: &mut Vec<(String, String, String)>,
) {
    match rl.status {
        types::ResourceStatus::Failed => {
            anomalies.push((
                m_name.to_string(),
                name.to_string(),
                "status is Failed".to_string(),
            ));
        }
        types::ResourceStatus::Drifted => {
            anomalies.push((
                m_name.to_string(),
                name.to_string(),
                "status is Drifted".to_string(),
            ));
        }
        _ => {}
    }
    if rl.applied_at.is_none() {
        anomalies.push((
            m_name.to_string(),
            name.to_string(),
            "missing applied_at timestamp".to_string(),
        ));
    }
}

fn collect_anomalies(
    state_dir: &Path,
    machine: Option<&str>,
) -> Result<Vec<(String, String, String)>, String> {
    let mut anomalies = Vec::new();
    if !state_dir.exists() {
        return Ok(anomalies);
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
        if let Some(filter) = machine {
            if m_name != filter {
                continue;
            }
        }
        if let Ok(Some(lock)) = state::load_lock(state_dir, &m_name) {
            for (name, rl) in &lock.resources {
                detect_resource_anomalies(&m_name, name, rl, &mut anomalies);
            }
        }
    }
    Ok(anomalies)
}

pub(crate) fn cmd_status_anomalies(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let anomalies = collect_anomalies(state_dir, machine)?;

    if json {
        let entries: Vec<String> = anomalies
            .iter()
            .map(|(m, r, issue)| {
                format!("{{\"machine\":\"{m}\",\"resource\":\"{r}\",\"issue\":\"{issue}\"}}")
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if anomalies.is_empty() {
        println!("{} No anomalies detected", green("✓"));
    } else {
        println!("{} {} anomalie(s) detected:", yellow("⚠"), anomalies.len());
        for (m, r, issue) in &anomalies {
            println!("  {} {}/{} — {}", red("•"), m, r, issue);
        }
    }
    Ok(())
}

// ── FJ-407: status --diff-from ──

fn diff_both_present(
    m_name: &str,
    cur: &types::StateLock,
    snap: &types::StateLock,
    diffs: &mut Vec<(String, String, String)>,
) {
    for (name, cur_rl) in &cur.resources {
        if let Some(snap_rl) = snap.resources.get(name) {
            if cur_rl.hash != snap_rl.hash {
                diffs.push((m_name.to_string(), name.clone(), "modified".to_string()));
            }
        } else {
            diffs.push((m_name.to_string(), name.clone(), "added".to_string()));
        }
    }
    for name in snap.resources.keys() {
        if !cur.resources.contains_key(name) {
            diffs.push((m_name.to_string(), name.clone(), "removed".to_string()));
        }
    }
}

fn add_all_as(
    m_name: &str,
    lock: &types::StateLock,
    change: &str,
    diffs: &mut Vec<(String, String, String)>,
) {
    for name in lock.resources.keys() {
        diffs.push((m_name.to_string(), name.clone(), change.to_string()));
    }
}

fn diff_machine_resources(
    m_name: &str,
    current: Option<types::StateLock>,
    snapshot: Option<types::StateLock>,
    diffs: &mut Vec<(String, String, String)>,
) {
    match (current, snapshot) {
        (Some(cur), Some(snap)) => diff_both_present(m_name, &cur, &snap, diffs),
        (Some(cur), None) => add_all_as(m_name, &cur, "added", diffs),
        (None, Some(snap)) => add_all_as(m_name, &snap, "removed", diffs),
        (None, None) => {}
    }
}

fn collect_diffs(
    state_dir: &Path,
    snap_dir: &Path,
) -> Result<Vec<(String, String, String)>, String> {
    let mut diffs = Vec::new();
    if !state_dir.exists() {
        return Ok(diffs);
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
        let current = state::load_lock(state_dir, &m_name).ok().flatten();
        let snapshot = state::load_lock(snap_dir, &m_name).ok().flatten();
        diff_machine_resources(&m_name, current, snapshot, &mut diffs);
    }
    Ok(diffs)
}

fn print_diff_output(diffs: &[(String, String, String)], snapshot_name: &str, json: bool) {
    if json {
        let entries: Vec<String> = diffs
            .iter()
            .map(|(m, r, change)| {
                format!("{{\"machine\":\"{m}\",\"resource\":\"{r}\",\"change\":\"{change}\"}}")
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if diffs.is_empty() {
        println!(
            "{} No changes since snapshot '{}'",
            green("✓"),
            snapshot_name
        );
    } else {
        println!("Changes since snapshot '{snapshot_name}':");
        for (m, r, change) in diffs {
            let prefix = match change.as_str() {
                "added" => green("+"),
                "removed" => red("-"),
                "modified" => yellow("~"),
                _ => dim("?"),
            };
            println!("  {prefix} {m}/{r}");
        }
    }
}

pub(crate) fn cmd_status_diff_from(
    state_dir: &Path,
    snapshot_name: &str,
    json: bool,
) -> Result<(), String> {
    let snap_dir = state_dir.join(".snapshots").join(snapshot_name);
    if !snap_dir.exists() {
        return Err(format!("snapshot '{snapshot_name}' not found"));
    }

    let diffs = collect_diffs(state_dir, &snap_dir)?;
    print_diff_output(&diffs, snapshot_name, json);
    Ok(())
}

/// FJ-594: Summarize errors across all machines.
fn collect_errors_for_machine(
    state_dir: &Path,
    m: &str,
    errors: &mut Vec<(String, String, String)>,
) {
    let lock_path = state_dir.join(m).join("state.lock.yaml");
    if !lock_path.exists() {
        return;
    }
    let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
    if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
        for (rname, rlock) in &lock.resources {
            if rlock.status == crate::core::types::ResourceStatus::Failed {
                let detail = if rlock.details.is_empty() {
                    "no details".to_string()
                } else {
                    rlock
                        .details
                        .iter()
                        .map(|(k, v)| format!("{k}={v:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                errors.push((m.to_string(), rname.clone(), detail));
            }
        }
    }
}

pub(crate) fn cmd_status_error_summary(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut errors: Vec<(String, String, String)> = Vec::new();

    for m in &machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        collect_errors_for_machine(state_dir, m, &mut errors);
    }

    if json {
        let items: Vec<String> = errors
            .iter()
            .map(|(m, r, d)| {
                format!(
                    r#"{{"machine":"{}","resource":"{}","error":"{}"}}"#,
                    m,
                    r,
                    d.replace('"', "\\\"")
                )
            })
            .collect();
        println!(
            r#"{{"errors":[{}],"count":{}}}"#,
            items.join(","),
            errors.len()
        );
    } else if errors.is_empty() {
        println!("No errors found across lock files");
    } else {
        println!("Error summary ({} failures):", errors.len());
        for (m, r, d) in &errors {
            println!("  {m}:{r} — {d}");
        }
    }
    Ok(())
}
