//! Phase 105 — Fleet Resilience & Convergence Analysis: status commands (FJ-1101, FJ-1104, FJ-1107).

use std::path::Path;

use super::helpers::discover_machines;
use crate::core::{state, types};

// -- Helpers -----------------------------------------------------------------

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

// -- FJ-1101: Fleet Resource Apply Success Trend -----------------------------

/// FJ-1101: `status --fleet-resource-apply-success-trend`
///
/// Show apply success/failure trend per machine. Count converged vs total
/// resources to derive a success percentage.
pub(crate) fn cmd_status_fleet_resource_apply_success_trend(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, f64, usize, usize)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (c, d, f, u) = classify_resources(&lock);
            let total = c + d + f + u;
            let success_pct = if total > 0 {
                c as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            rows.push((m.clone(), success_pct, c, total));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(|(m, pct, conv, tot)| {
                serde_json::json!({
                    "machine": m,
                    "success_pct": (*pct * 10.0).round() / 10.0,
                    "converged": *conv,
                    "total": *tot,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"fleet_resource_apply_success_trend": entries})
            )
            .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Resource Apply Success Trend ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, pct, conv, tot) in &rows {
            println!("  {m}: {pct:.1}% success ({conv} converged / {tot} total)",);
        }
    }
    Ok(())
}

// -- FJ-1104: Machine Resource Drift Age Distribution ------------------------

/// FJ-1104: `status --machine-resource-drift-age-distribution`
///
/// Show distribution of resource statuses per machine: converged, drifted, failed.
pub(crate) fn cmd_status_machine_resource_drift_age_distribution(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, usize, usize, usize)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (c, d, f, _u) = classify_resources(&lock);
            rows.push((m.clone(), c, d, f));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(|(m, c, d, f)| {
                serde_json::json!({
                    "machine": m,
                    "converged": *c,
                    "drifted": *d,
                    "failed": *f,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"machine_resource_drift_age_distribution": entries})
            )
            .unwrap_or_default()
        );
    } else {
        println!("=== Machine Resource Drift Age Distribution ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, c, d, f) in &rows {
            println!("  {m}: {c} converged, {d} drifted, {f} failed",);
        }
    }
    Ok(())
}

// -- FJ-1107: Fleet Resource Convergence Gap Analysis ------------------------

/// FJ-1107: `status --fleet-resource-convergence-gap-analysis`
///
/// Compute convergence rate per machine. Identify machines below fleet average.
pub(crate) fn cmd_status_fleet_resource_convergence_gap_analysis(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, f64, usize, usize)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (c, d, f, u) = classify_resources(&lock);
            let total = c + d + f + u;
            let convergence_pct = if total > 0 {
                c as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            rows.push((m.clone(), convergence_pct, c, total));
        }
    }
    // Compute fleet average convergence.
    let fleet_avg = if rows.is_empty() {
        0.0
    } else {
        let sum: f64 = rows.iter().map(|(_, pct, _, _)| *pct).sum();
        sum / rows.len() as f64
    };
    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(|(m, pct, _c, _t)| {
                let gap = fleet_avg - *pct;
                serde_json::json!({
                    "machine": m,
                    "convergence_pct": (*pct * 10.0).round() / 10.0,
                    "gap": (gap * 10.0).round() / 10.0,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "fleet_average": (fleet_avg * 10.0).round() / 10.0,
                "machines": entries,
            }))
            .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Resource Convergence Gap Analysis ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        } else {
            println!("  fleet_average: {fleet_avg:.1}%");
            for (m, pct, _c, _t) in &rows {
                let gap = fleet_avg - *pct;
                println!("  {m}: {pct:.1}% (gap: {gap:.1}%)",);
            }
        }
    }
    Ok(())
}
