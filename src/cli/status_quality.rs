//! Phase 107 — Resource Quality & Convergence Analysis: status commands (FJ-1117, FJ-1120, FJ-1123).

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
pub(super) fn safe_pct(numerator: usize, denominator: usize) -> f64 {
    if denominator > 0 {
        numerator as f64 / denominator as f64 * 100.0
    } else {
        0.0
    }
}
pub(super) fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

// -- FJ-1117: Fleet Resource Quality Score -----------------------------------

pub(super) struct QualityRow {
    pub(super) machine: String,
    pub(super) score: f64,
    pub(super) converged: usize,
    pub(super) drifted: usize,
    pub(super) failed: usize,
    pub(super) total: usize,
}
pub(super) fn gather_quality_rows(state_dir: &Path, machines: &[String]) -> Vec<QualityRow> {
    let mut rows = Vec::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let total = lock.resources.len();
            let converged = lock
                .resources
                .values()
                .filter(|r| r.status == types::ResourceStatus::Converged)
                .count();
            let drifted = lock
                .resources
                .values()
                .filter(|r| r.status == types::ResourceStatus::Drifted)
                .count();
            let failed = lock
                .resources
                .values()
                .filter(|r| r.status == types::ResourceStatus::Failed)
                .count();
            let convergence_pct = safe_pct(converged, total);
            let non_drift_pct = safe_pct(total.saturating_sub(drifted), total);
            let non_fail_pct = safe_pct(total.saturating_sub(failed), total);
            let score = convergence_pct * 0.4 + non_drift_pct * 0.3 + non_fail_pct * 0.3;
            rows.push(QualityRow {
                machine: m.clone(),
                score,
                converged,
                drifted,
                failed,
                total,
            });
        }
    }
    rows
}
pub(super) fn print_quality_text(rows: &[QualityRow]) {
    println!("=== Fleet Resource Quality Score ===");
    if rows.is_empty() {
        println!("  No machine state found.");
    }
    for r in rows {
        println!(
            "  {}: {:.1}/100 ({}/{} converged, {} drifted, {} failed)",
            r.machine, r.score, r.converged, r.total, r.drifted, r.failed,
        );
    }
}
pub(super) fn print_quality_json(rows: &[QualityRow]) {
    let entries: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "machine": r.machine, "score": round1(r.score),
                "converged": r.converged, "drifted": r.drifted,
                "failed": r.failed, "total": r.total,
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({"fleet_resource_quality_score": entries}))
            .unwrap_or_default()
    );
}
pub(crate) fn cmd_status_fleet_resource_quality_score(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let rows = gather_quality_rows(state_dir, &filtered_machines(state_dir, machine));
    if json {
        print_quality_json(&rows);
    } else {
        print_quality_text(&rows);
    }
    Ok(())
}

// -- FJ-1120: Machine Resource Drift Pattern Classification ------------------

pub(super) struct DriftPatternRow {
    pub(super) machine: String,
    pub(super) classification: String,
    pub(super) drifted: usize,
    pub(super) total: usize,
    pub(super) drift_pct: f64,
}
pub(super) fn classify_drift(pct: f64) -> &'static str {
    if pct > 50.0 {
        "chronic"
    } else if pct >= 10.0 {
        "transient"
    } else {
        "stable"
    }
}
pub(super) fn gather_drift_pattern_rows(
    state_dir: &Path,
    machines: &[String],
) -> Vec<DriftPatternRow> {
    let mut rows = Vec::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let total = lock.resources.len();
            let drifted = lock
                .resources
                .values()
                .filter(|r| r.status == types::ResourceStatus::Drifted)
                .count();
            let pct = safe_pct(drifted, total);
            rows.push(DriftPatternRow {
                machine: m.clone(),
                classification: classify_drift(pct).to_string(),
                drifted,
                total,
                drift_pct: pct,
            });
        }
    }
    rows
}
pub(super) fn print_drift_pattern_text(rows: &[DriftPatternRow]) {
    println!("=== Machine Resource Drift Pattern Classification ===");
    if rows.is_empty() {
        println!("  No machine state found.");
    }
    for r in rows {
        println!(
            "  {}: {} ({}/{} drifted, {:.1}%)",
            r.machine, r.classification, r.drifted, r.total, r.drift_pct
        );
    }
}
pub(super) fn print_drift_pattern_json(rows: &[DriftPatternRow]) {
    let entries: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "machine": r.machine, "classification": r.classification,
                "drifted": r.drifted, "total": r.total,
                "drift_pct": round1(r.drift_pct),
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"machine_resource_drift_pattern_classification": entries})
        )
        .unwrap_or_default()
    );
}
pub(crate) fn cmd_status_machine_resource_drift_pattern_classification(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let rows = gather_drift_pattern_rows(state_dir, &filtered_machines(state_dir, machine));
    if json {
        print_drift_pattern_json(&rows);
    } else {
        print_drift_pattern_text(&rows);
    }
    Ok(())
}

// -- FJ-1123: Fleet Resource Convergence Window Analysis ---------------------

pub(super) struct ConvergenceRow {
    pub(super) machine: String,
    pub(super) convergence_pct: f64,
    pub(super) converged: usize,
    pub(super) total: usize,
}
pub(super) fn gather_convergence_rows(
    state_dir: &Path,
    machines: &[String],
) -> Vec<ConvergenceRow> {
    let mut rows = Vec::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let total = lock.resources.len();
            let converged = lock
                .resources
                .values()
                .filter(|r| r.status == types::ResourceStatus::Converged)
                .count();
            rows.push(ConvergenceRow {
                machine: m.clone(),
                convergence_pct: safe_pct(converged, total),
                converged,
                total,
            });
        }
    }
    rows
}
pub(super) fn fleet_average(rows: &[ConvergenceRow]) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    let sum: f64 = rows.iter().map(|r| r.convergence_pct).sum();
    sum / rows.len() as f64
}
pub(super) fn print_convergence_text(rows: &[ConvergenceRow], avg: f64) {
    println!("=== Fleet Resource Convergence Window Analysis ===");
    if rows.is_empty() {
        println!("  No machine state found.");
    }
    for r in rows {
        println!(
            "  {}: {:.1}% converged ({}/{})",
            r.machine, r.convergence_pct, r.converged, r.total
        );
    }
    if !rows.is_empty() {
        println!("  fleet_average: {avg:.1}%");
    }
}
pub(super) fn print_convergence_json(rows: &[ConvergenceRow], avg: f64) {
    let entries: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "machine": r.machine, "convergence_pct": round1(r.convergence_pct),
                "converged": r.converged, "total": r.total,
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "fleet_resource_convergence_window_analysis": {
                "machines": entries,
                "fleet_average": round1(avg),
            }
        }))
        .unwrap_or_default()
    );
}
pub(crate) fn cmd_status_fleet_resource_convergence_window_analysis(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let rows = gather_convergence_rows(state_dir, &filtered_machines(state_dir, machine));
    let avg = fleet_average(&rows);
    if json {
        print_convergence_json(&rows, avg);
    } else {
        print_convergence_text(&rows, avg);
    }
    Ok(())
}
