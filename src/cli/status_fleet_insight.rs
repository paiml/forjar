//! Phase 101 — Fleet Insight & Dependency Quality: status commands (FJ-1069, FJ-1072, FJ-1075).

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

/// Default staleness threshold in seconds (7 days).
const STALENESS_THRESHOLD_SECS: u64 = 7 * 24 * 3600;

// ── FJ-1069: Fleet Resource Staleness Report ────────────────────────────────

/// Compute staleness for a single machine lock file.
pub(super) fn staleness_row(
    state_dir: &Path,
    m: &str,
    now: u64,
) -> Option<(String, String, u64, bool)> {
    let lock = state::load_lock(state_dir, m).ok()??;
    let age_secs = parse_rfc3339_to_epoch(&lock.generated_at)
        .filter(|&epoch| now >= epoch)
        .map_or(0, |epoch| now - epoch);
    let stale = age_secs >= STALENESS_THRESHOLD_SECS;
    Some((m.to_string(), lock.generated_at.clone(), age_secs, stale))
}

/// FJ-1069: `status --fleet-resource-staleness-report`
pub(crate) fn cmd_status_fleet_resource_staleness_report(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let now = now_epoch();
    let rows: Vec<_> = machines
        .iter()
        .filter_map(|m| staleness_row(state_dir, m, now))
        .collect();
    if json {
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, ts, age, stale)| {
            serde_json::json!({"machine": m, "generated_at": ts, "age_days": *age / 86_400, "stale": *stale})
        }).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"staleness_report": entries}))
                .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Resource Staleness Report ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, _ts, age, stale) in &rows {
            let label = if *stale { " [STALE]" } else { "" };
            println!("  {}: {}d old{}", m, *age / 86_400, label);
        }
    }
    Ok(())
}

// ── FJ-1072: Machine Resource Type Distribution ─────────────────────────────

/// FJ-1072: `status --machine-resource-type-distribution`
///
/// Shows per-machine breakdown of resource types (file, package, service, etc.).
/// Parses lock files, counts resources by type per machine, and prints a table.
pub(crate) fn cmd_status_machine_resource_type_distribution(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, std::collections::BTreeMap<String, usize>)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let mut counts: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();
            for rl in lock.resources.values() {
                *counts.entry(rl.resource_type.to_string()).or_insert(0) += 1;
            }
            rows.push((m.clone(), counts));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(|(m, counts)| serde_json::json!({"machine": m, "types": counts}))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"type_distribution": entries}))
                .unwrap_or_default()
        );
    } else {
        println!("=== Machine Resource Type Distribution ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, counts) in &rows {
            let parts: Vec<String> = counts.iter().map(|(t, c)| format!("{t}={c}")).collect();
            println!("  {}: {}", m, parts.join(", "));
        }
    }
    Ok(())
}

// ── FJ-1075: Fleet Machine Health Score ─────────────────────────────────────

/// FJ-1075: `status --fleet-machine-health-score`
///
/// Computes a composite health score per machine based on convergence, drift,
/// and error rates. Score = converged_pct * 100, penalized by drift and failures.
pub(crate) fn cmd_status_fleet_machine_health_score(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, f64, f64, f64, f64)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (c, d, f, u) = classify_resources(&lock);
            let total = c + d + f + u;
            if total == 0 {
                rows.push((m.clone(), 0.0, 0.0, 0.0, 0.0));
                continue;
            }
            let converged_pct = c as f64 / total as f64;
            let drifted_pct = d as f64 / total as f64;
            let failed_pct = f as f64 / total as f64;
            // Score: base from converged percentage, penalized by drift and failures
            let score = (converged_pct * 100.0) - (drifted_pct * 25.0) - (failed_pct * 50.0);
            let score = score.clamp(0.0, 100.0);
            rows.push((
                m.clone(),
                score,
                converged_pct * 100.0,
                drifted_pct * 100.0,
                failed_pct * 100.0,
            ));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(|(m, score, conv, drift, fail)| {
                serde_json::json!({
                    "machine": m,
                    "health_score": (*score * 10.0).round() / 10.0,
                    "converged_pct": (*conv * 10.0).round() / 10.0,
                    "drifted_pct": (*drift * 10.0).round() / 10.0,
                    "failed_pct": (*fail * 10.0).round() / 10.0,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({"health_scores": entries}))
                .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Machine Health Score ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, score, conv, drift, fail) in &rows {
            println!(
                "  {m}: score={score:.1}, converged={conv:.1}%, drifted={drift:.1}%, failed={fail:.1}%",
            );
        }
    }
    Ok(())
}
