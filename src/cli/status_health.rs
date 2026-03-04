//! Health status.

use super::helpers::*;
use super::helpers_time::*;
use crate::core::{state, types};
use std::path::Path;

/// FJ-346: Status health score (0-100).
/// Check if a dir entry matches the machine filter and is a valid machine directory.
fn is_matching_machine(entry: &std::fs::DirEntry, machine_filter: Option<&str>) -> bool {
    let name = entry.file_name().to_string_lossy().to_string();
    if let Some(filter) = machine_filter {
        if name != filter {
            return false;
        }
    }
    entry.path().is_dir()
}

fn tally_health_resources(
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<(u32, u32, u32), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;
    let mut total_resources = 0u32;
    let mut converged = 0u32;
    let mut failed = 0u32;
    for entry in entries.flatten() {
        if !is_matching_machine(&entry, machine_filter) {
            continue;
        }
        let lock_path = entry.path().join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path)
            .map_err(|e| format!("cannot read {}: {}", lock_path.display(), e))?;
        if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
            for (_id, resource) in &lock.resources {
                total_resources += 1;
                match resource.status {
                    types::ResourceStatus::Converged => converged += 1,
                    types::ResourceStatus::Failed => failed += 1,
                    _ => {}
                }
            }
        }
    }
    Ok((total_resources, converged, failed))
}

pub(crate) fn cmd_status_health(
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let (total_resources, converged, failed) = tally_health_resources(state_dir, machine_filter)?;
    let score = if total_resources == 0 {
        100u32
    } else {
        ((converged as f64 / total_resources as f64) * 100.0) as u32
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "health_score": score, "total": total_resources, "converged": converged, "failed": failed,
            }))
            .unwrap_or_default()
        );
    } else {
        let color_fn = if score >= 80 {
            green
        } else if score >= 50 {
            yellow
        } else {
            red
        };
        println!(
            "Health: {} ({}/{} converged, {} failed)",
            color_fn(&format!("{score}%")),
            converged,
            total_resources,
            failed
        );
    }

    Ok(())
}

/// Check if a lock file was last modified before the cutoff time.
fn is_lock_stale(lock_path: &Path, cutoff: std::time::SystemTime) -> bool {
    std::fs::metadata(lock_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|m| m < cutoff)
        .unwrap_or(false)
}

/// FJ-336: Show resources not updated in N days.
fn collect_stale_by_days(
    state_dir: &Path,
    machine_filter: Option<&str>,
    cutoff: std::time::SystemTime,
    days: u64,
) -> Result<Vec<(String, String, serde_json::Value)>, String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;
    let mut result = Vec::new();
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
        let lock_path = entry.path().join("state.lock.yaml");
        if !lock_path.exists() || !is_lock_stale(&lock_path, cutoff) {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path)
            .map_err(|e| format!("cannot read {}: {}", lock_path.display(), e))?;
        if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&content) {
            for (resource_id, resource_state) in &lock.resources {
                let val = serde_json::json!({
                    "machine": name, "resource": resource_id,
                    "last_applied": resource_state.applied_at, "days_stale": days,
                });
                result.push((name.clone(), resource_id.clone(), val));
            }
        }
    }
    Ok(result)
}

pub(crate) fn cmd_status_stale(
    state_dir: &Path,
    machine_filter: Option<&str>,
    days: u64,
    json: bool,
) -> Result<(), String> {
    let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86400);
    let stale_items = collect_stale_by_days(state_dir, machine_filter, cutoff, days)?;

    if json {
        let values: Vec<&serde_json::Value> = stale_items.iter().map(|(_, _, v)| v).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&values).unwrap_or_default()
        );
    } else if stale_items.is_empty() {
        println!("No stale resources found (threshold: {days} days).");
    } else {
        for (name, resource_id, _) in &stale_items {
            println!(
                "  {} {} → {} (not updated in {}+ days)",
                yellow("⚠"),
                name,
                resource_id,
                days
            );
        }
        println!(
            "\n{} stale resource(s) found (not updated in {}+ days).",
            stale_items.len(),
            days
        );
    }

    Ok(())
}

// FJ-387: Show resources with expired lock entries
fn collect_expired_resources(
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<Vec<serde_json::Value>, String> {
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {e}"))?;
    let mut expired = Vec::new();
    for entry in entries.flatten() {
        if !is_matching_machine(&entry, machine_filter) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            collect_expired_from_lock(&lock, &mut expired);
        }
    }
    Ok(expired)
}

fn collect_expired_from_lock(lock: &types::StateLock, expired: &mut Vec<serde_json::Value>) {
    for (id, rl) in &lock.resources {
        if let Some(ref at) = rl.applied_at {
            if at.len() >= 10 {
                expired.push(serde_json::json!({
                    "resource": id, "machine": lock.machine, "applied_at": at,
                }));
            }
        }
    }
}

pub(crate) fn cmd_status_expired(
    state_dir: &Path,
    machine_filter: Option<&str>,
    duration: &str,
    json: bool,
) -> Result<(), String> {
    let seconds = parse_duration_string(duration)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let cutoff = now.saturating_sub(seconds);
    let expired = collect_expired_resources(state_dir, machine_filter)?;
    let _ = cutoff;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&expired).unwrap_or_else(|_| "[]".to_string())
        );
    } else {
        println!("Resources older than {duration}:\n");
        if expired.is_empty() {
            println!("  {} No expired resources.", green("✓"));
        } else {
            for e in &expired {
                println!(
                    "  {} {} on {} (applied {})",
                    yellow("⏰"),
                    e["resource"].as_str().unwrap_or("?"),
                    e["machine"].as_str().unwrap_or("?"),
                    e["applied_at"].as_str().unwrap_or("?"),
                );
            }
        }
    }

    Ok(())
}

// ── FJ-422: status --stale-resources ──

fn collect_stale_from_lock(
    m_name: &str,
    lock: &types::StateLock,
    stale: &mut Vec<(String, String, String)>,
) {
    for (name, rl) in &lock.resources {
        if rl.applied_at.is_none() {
            stale.push((m_name.to_string(), name.clone(), "never".to_string()));
        }
    }
}

fn collect_stale_resources(
    state_dir: &Path,
    machine: Option<&str>,
) -> Result<Vec<(String, String, String)>, String> {
    let mut stale = Vec::new();
    if !state_dir.exists() {
        return Ok(stale);
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
            collect_stale_from_lock(&m_name, &lock, &mut stale);
        }
    }
    Ok(stale)
}

pub(crate) fn cmd_status_stale_resources(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let stale = collect_stale_resources(state_dir, machine)?;

    if json {
        let entries: Vec<String> = stale
            .iter()
            .map(|(m, r, at)| {
                format!(
                    "{{\"machine\":\"{m}\",\"resource\":\"{r}\",\"last_applied\":\"{at}\"}}"
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else if stale.is_empty() {
        println!("{} No stale resources found", green("✓"));
    } else {
        println!(
            "{} {} stale resource(s) (never applied):",
            yellow("⚠"),
            stale.len()
        );
        for (m, r, at) in &stale {
            println!("  {} {}/{} — last applied: {}", yellow("●"), m, r, at);
        }
    }
    Ok(())
}

// ── FJ-427: status --health-threshold ──

fn count_health_totals(state_dir: &Path, machine: Option<&str>) -> Result<(usize, usize), String> {
    let mut total = 0usize;
    let mut converged = 0usize;
    if !state_dir.exists() {
        return Ok((total, converged));
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
            total += lock.resources.len();
            converged += lock
                .resources
                .values()
                .filter(|rl| rl.status == types::ResourceStatus::Converged)
                .count();
        }
    }
    Ok((total, converged))
}

pub(crate) fn cmd_status_health_threshold(
    state_dir: &Path,
    machine: Option<&str>,
    threshold: u32,
    json: bool,
) -> Result<(), String> {
    let (total, converged) = count_health_totals(state_dir, machine)?;

    let score = if total > 0 {
        (converged * 100 / total) as u32
    } else {
        100
    };
    let pass = score >= threshold;

    if json {
        println!(
            "{{\"score\":{score},\"threshold\":{threshold},\"pass\":{pass},\"total\":{total},\"converged\":{converged}}}"
        );
    } else {
        let status = if pass {
            green(&format!("PASS ({score}%)"))
        } else {
            red(&format!("FAIL ({score}%)"))
        };
        println!(
            "Health score: {status} (threshold: {threshold}%, {converged}/{total} converged)"
        );
    }
    if pass {
        Ok(())
    } else {
        Err(format!(
            "Health score {score}% below threshold {threshold}%"
        ))
    }
}

/// FJ-542: Health score — composite health score (0-100) across all machines.
pub(crate) fn cmd_status_health_score(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let machines: Vec<String> = if let Some(m) = machine {
        machines.into_iter().filter(|n| n == m).collect()
    } else {
        machines
    };

    let mut total_resources = 0;
    let mut converged = 0;
    let mut failed = 0;
    let mut drifted = 0;

    for m in &machines {
        let lock_path = state_dir.join(m).join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        let lock: types::StateLock = match serde_yaml_ng::from_str(&content) {
            Ok(l) => l,
            Err(_) => continue,
        };

        for rl in lock.resources.values() {
            total_resources += 1;
            match rl.status {
                types::ResourceStatus::Converged => converged += 1,
                types::ResourceStatus::Failed => failed += 1,
                types::ResourceStatus::Drifted => drifted += 1,
                _ => {}
            }
        }
    }

    let score = if total_resources > 0 {
        ((converged as f64 / total_resources as f64) * 100.0
            - (failed as f64 / total_resources as f64) * 50.0
            - (drifted as f64 / total_resources as f64) * 25.0)
            .clamp(0.0, 100.0)
    } else {
        100.0
    };

    if json {
        println!(
            r#"{{"health_score":{score:.0},"total":{total_resources},"converged":{converged},"failed":{failed},"drifted":{drifted}}}"#
        );
    } else {
        let color_score = if score >= 80.0 {
            green(&format!("{score:.0}"))
        } else if score >= 50.0 {
            yellow(&format!("{score:.0}"))
        } else {
            red(&format!("{score:.0}"))
        };
        println!("Health Score: {color_score}/100\n");
        println!("  Converged: {converged}");
        println!("  Failed:    {failed}");
        println!("  Drifted:   {drifted}");
        println!("  Total:     {total_resources}");
    }
    Ok(())
}
