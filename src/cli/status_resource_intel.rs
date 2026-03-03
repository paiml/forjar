//! Phase 102 — Resource Intelligence & Topology Insight: status commands (FJ-1077, FJ-1080, FJ-1083).

use std::path::Path;

use super::helpers::discover_machines;
use crate::core::{state, types};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Minimal RFC-3339 timestamp parser returning seconds since Unix epoch.
pub(super) fn parse_rfc3339_to_epoch(s: &str) -> Option<u64> {
    if s.len() < 19 {
        return None;
    }
    let year: u64 = s.get(0..4)?.parse().ok()?;
    let month: u64 = s.get(5..7)?.parse().ok()?;
    let day: u64 = s.get(8..10)?.parse().ok()?;
    let hour: u64 = s.get(11..13)?.parse().ok()?;
    let min: u64 = s.get(14..16)?.parse().ok()?;
    let sec: u64 = s.get(17..19)?.parse().ok()?;
    let mut days: u64 = 0;
    for y in 1970..year {
        days += if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
            366
        } else {
            365
        };
    }
    let table = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30];
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mut md: u64 = 0;
    for m in 1..month.min(13) {
        md += table[m as usize];
        if m == 2 && leap {
            md += 1;
        }
    }
    days += md + (day - 1);
    Some(days * 86_400 + hour * 3600 + min * 60 + sec)
}

/// Return current Unix epoch in seconds.
pub(super) fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Filter machines by optional name.
pub(super) fn filtered_machines(state_dir: &Path, machine: Option<&str>) -> Vec<String> {
    let all = discover_machines(state_dir);
    match machine {
        Some(m) => all.into_iter().filter(|n| n == m).collect(),
        None => all,
    }
}

/// Classify resources in a lock file into (converged, drifted, failed, unknown).
pub(super) fn classify_resources(lock: &types::StateLock) -> (usize, usize, usize, usize) {
    let mut converged = 0usize;
    let mut drifted = 0usize;
    let mut failed = 0usize;
    let mut unknown = 0usize;
    for rl in lock.resources.values() {
        match rl.status {
            types::ResourceStatus::Converged => converged += 1,
            types::ResourceStatus::Drifted => drifted += 1,
            types::ResourceStatus::Failed => failed += 1,
            types::ResourceStatus::Unknown => unknown += 1,
        }
    }
    (converged, drifted, failed, unknown)
}

// ── FJ-1077: Fleet Resource Dependency Lag ──────────────────────────────────

/// FJ-1077: `status --fleet-resource-dependency-lag`
///
/// For each machine, count resources with status != "converged" and report the
/// lag count. Non-converged resources represent dependency lag in the fleet.
pub(crate) fn cmd_status_fleet_resource_dependency_lag_report(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, usize, usize)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (converged, drifted, failed, unknown) = classify_resources(&lock);
            let total = converged + drifted + failed + unknown;
            let lagging = drifted + failed + unknown;
            rows.push((m.clone(), lagging, total));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(
                |(m, lag, total)| serde_json::json!({"machine": m, "lagging": lag, "total": total}),
            )
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"dependency_lag": entries}))
                .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Resource Dependency Lag ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, lag, total) in &rows {
            println!("  {}: {}/{} lagging", m, lag, total);
        }
    }
    Ok(())
}

// ── FJ-1080: Machine Resource Convergence Rate Trend ────────────────────────

/// FJ-1080: `status --machine-resource-convergence-rate-trend`
///
/// Track convergence rate per machine: converged / total * 100. Shows the
/// percentage of resources that have reached converged state.
pub(crate) fn cmd_status_machine_resource_convergence_rate_trend(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, f64, usize, usize)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (converged, drifted, failed, unknown) = classify_resources(&lock);
            let total = converged + drifted + failed + unknown;
            let rate = if total == 0 {
                0.0
            } else {
                converged as f64 / total as f64 * 100.0
            };
            rows.push((m.clone(), rate, converged, total));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(|(m, rate, conv, total)| {
                serde_json::json!({
                    "machine": m,
                    "convergence_rate": (*rate * 10.0).round() / 10.0,
                    "converged": conv,
                    "total": total,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"convergence_rate_trend": entries}))
                .unwrap_or_default()
        );
    } else {
        println!("=== Machine Resource Convergence Rate Trend ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, rate, conv, total) in &rows {
            println!("  {}: {:.1}% ({}/{})", m, rate, conv, total);
        }
    }
    Ok(())
}

// ── FJ-1083: Fleet Resource Apply Lag ───────────────────────────────────────

/// FJ-1083: `status --fleet-resource-apply-lag`
///
/// Report time since last successful apply per machine. Parses `generated_at`
/// from lock files and computes the age relative to the current time.
pub(crate) fn cmd_status_fleet_resource_apply_lag(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let now = now_epoch();
    let mut rows: Vec<(String, String, u64)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let age_secs = parse_rfc3339_to_epoch(&lock.generated_at)
                .filter(|&epoch| now >= epoch)
                .map_or(0, |epoch| now - epoch);
            rows.push((m.clone(), lock.generated_at.clone(), age_secs));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(|(m, ts, age)| {
                serde_json::json!({
                    "machine": m,
                    "last_apply": ts,
                    "age_seconds": age,
                    "age_days": *age / 86_400,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"apply_lag": entries}))
                .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Resource Apply Lag ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, _ts, age) in &rows {
            let days = *age / 86_400;
            let hours = (*age % 86_400) / 3600;
            println!("  {}: {}d {}h ago", m, days, hours);
        }
    }
    Ok(())
}
