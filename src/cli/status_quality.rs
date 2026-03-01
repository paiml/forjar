//! Phase 107 — Resource Quality & Convergence Analysis: status commands (FJ-1117, FJ-1120, FJ-1123).

use std::path::Path;

use crate::core::{state, types};
use super::helpers::discover_machines;

// -- Helpers -----------------------------------------------------------------

/// Filter machines by optional name.
fn filtered_machines(state_dir: &Path, machine: Option<&str>) -> Vec<String> {
    let all = discover_machines(state_dir);
    match machine {
        Some(m) => all.into_iter().filter(|n| n == m).collect(),
        None => all,
    }
}
fn safe_pct(numerator: usize, denominator: usize) -> f64 {
    if denominator > 0 { numerator as f64 / denominator as f64 * 100.0 } else { 0.0 }
}
fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

// -- FJ-1117: Fleet Resource Quality Score -----------------------------------

struct QualityRow {
    machine: String,
    score: f64,
    converged: usize,
    drifted: usize,
    failed: usize,
    total: usize,
}
fn gather_quality_rows(state_dir: &Path, machines: &[String]) -> Vec<QualityRow> {
    let mut rows = Vec::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let total = lock.resources.len();
            let converged = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Converged).count();
            let drifted = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Drifted).count();
            let failed = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Failed).count();
            let convergence_pct = safe_pct(converged, total);
            let non_drift_pct = safe_pct(total.saturating_sub(drifted), total);
            let non_fail_pct = safe_pct(total.saturating_sub(failed), total);
            let score = convergence_pct * 0.4 + non_drift_pct * 0.3 + non_fail_pct * 0.3;
            rows.push(QualityRow { machine: m.clone(), score, converged, drifted, failed, total });
        }
    }
    rows
}
fn print_quality_text(rows: &[QualityRow]) {
    println!("=== Fleet Resource Quality Score ===");
    if rows.is_empty() { println!("  No machine state found."); }
    for r in rows {
        println!(
            "  {}: {:.1}/100 ({}/{} converged, {} drifted, {} failed)",
            r.machine, r.score, r.converged, r.total, r.drifted, r.failed,
        );
    }
}
fn print_quality_json(rows: &[QualityRow]) {
    let entries: Vec<serde_json::Value> = rows.iter()
        .map(|r| serde_json::json!({
            "machine": r.machine, "score": round1(r.score),
            "converged": r.converged, "drifted": r.drifted,
            "failed": r.failed, "total": r.total,
        }))
        .collect();
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"fleet_resource_quality_score": entries})).unwrap_or_default());
}
pub(crate) fn cmd_status_fleet_resource_quality_score(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let rows = gather_quality_rows(state_dir, &filtered_machines(state_dir, machine));
    if json { print_quality_json(&rows); } else { print_quality_text(&rows); }
    Ok(())
}

// -- FJ-1120: Machine Resource Drift Pattern Classification ------------------

struct DriftPatternRow {
    machine: String,
    classification: String,
    drifted: usize,
    total: usize,
    drift_pct: f64,
}
fn classify_drift(pct: f64) -> &'static str {
    if pct > 50.0 { "chronic" } else if pct >= 10.0 { "transient" } else { "stable" }
}
fn gather_drift_pattern_rows(state_dir: &Path, machines: &[String]) -> Vec<DriftPatternRow> {
    let mut rows = Vec::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let total = lock.resources.len();
            let drifted = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Drifted).count();
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
fn print_drift_pattern_text(rows: &[DriftPatternRow]) {
    println!("=== Machine Resource Drift Pattern Classification ===");
    if rows.is_empty() { println!("  No machine state found."); }
    for r in rows {
        println!("  {}: {} ({}/{} drifted, {:.1}%)", r.machine, r.classification, r.drifted, r.total, r.drift_pct);
    }
}
fn print_drift_pattern_json(rows: &[DriftPatternRow]) {
    let entries: Vec<serde_json::Value> = rows.iter()
        .map(|r| serde_json::json!({
            "machine": r.machine, "classification": r.classification,
            "drifted": r.drifted, "total": r.total,
            "drift_pct": round1(r.drift_pct),
        }))
        .collect();
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"machine_resource_drift_pattern_classification": entries})).unwrap_or_default());
}
pub(crate) fn cmd_status_machine_resource_drift_pattern_classification(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let rows = gather_drift_pattern_rows(state_dir, &filtered_machines(state_dir, machine));
    if json { print_drift_pattern_json(&rows); } else { print_drift_pattern_text(&rows); }
    Ok(())
}

// -- FJ-1123: Fleet Resource Convergence Window Analysis ---------------------

struct ConvergenceRow {
    machine: String,
    convergence_pct: f64,
    converged: usize,
    total: usize,
}
fn gather_convergence_rows(state_dir: &Path, machines: &[String]) -> Vec<ConvergenceRow> {
    let mut rows = Vec::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let total = lock.resources.len();
            let converged = lock.resources.values().filter(|r| r.status == types::ResourceStatus::Converged).count();
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
fn fleet_average(rows: &[ConvergenceRow]) -> f64 {
    if rows.is_empty() { return 0.0; }
    let sum: f64 = rows.iter().map(|r| r.convergence_pct).sum();
    sum / rows.len() as f64
}
fn print_convergence_text(rows: &[ConvergenceRow], avg: f64) {
    println!("=== Fleet Resource Convergence Window Analysis ===");
    if rows.is_empty() { println!("  No machine state found."); }
    for r in rows {
        println!("  {}: {:.1}% converged ({}/{})", r.machine, r.convergence_pct, r.converged, r.total);
    }
    if !rows.is_empty() { println!("  fleet_average: {:.1}%", avg); }
}
fn print_convergence_json(rows: &[ConvergenceRow], avg: f64) {
    let entries: Vec<serde_json::Value> = rows.iter()
        .map(|r| serde_json::json!({
            "machine": r.machine, "convergence_pct": round1(r.convergence_pct),
            "converged": r.converged, "total": r.total,
        }))
        .collect();
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "fleet_resource_convergence_window_analysis": {
            "machines": entries,
            "fleet_average": round1(avg),
        }
    })).unwrap_or_default());
}
pub(crate) fn cmd_status_fleet_resource_convergence_window_analysis(
    state_dir: &Path, machine: Option<&str>, json: bool,
) -> Result<(), String> {
    let rows = gather_convergence_rows(state_dir, &filtered_machines(state_dir, machine));
    let avg = fleet_average(&rows);
    if json { print_convergence_json(&rows, avg); } else { print_convergence_text(&rows, avg); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mk(
        machine: &str,
        ts: &str,
        res: Vec<(&str, types::ResourceType, types::ResourceStatus)>,
    ) -> types::StateLock {
        let mut m = indexmap::IndexMap::new();
        for (id, rt, st) in res {
            m.insert(
                id.to_string(),
                types::ResourceLock {
                    resource_type: rt,
                    status: st,
                    applied_at: Some(ts.into()),
                    duration_seconds: Some(1.0),
                    hash: "abc".into(),
                    details: HashMap::new(),
                },
            );
        }
        types::StateLock {
            schema: "1".into(),
            machine: machine.into(),
            hostname: machine.into(),
            generated_at: ts.into(),
            generator: "test".into(),
            blake3_version: "1.0".into(),
            resources: m,
        }
    }
    fn wr(dir: &std::path::Path, lock: &types::StateLock) {
        let d = dir.join(&lock.machine);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("state.lock.yaml"), serde_yaml_ng::to_string(lock).unwrap()).unwrap();
    }

    // -- FJ-1117: Fleet Resource Quality Score -----------------------------------

    #[test]
    fn test_quality_score_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_quality_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_quality_score_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg1", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("pkg2", types::ResourceType::Package, types::ResourceStatus::Drifted),
            ("svc1", types::ResourceType::Service, types::ResourceStatus::Failed),
            ("cfg1", types::ResourceType::File, types::ResourceStatus::Converged),
        ]));
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("pkg1", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc1", types::ResourceType::Service, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_quality_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_quality_score_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("n1", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("s", types::ResourceType::Service, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_fleet_resource_quality_score(d.path(), None, true).is_ok());
    }
    #[test]
    fn test_quality_score_filtered() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Failed),
        ]));
        assert!(cmd_status_fleet_resource_quality_score(d.path(), Some("web"), false).is_ok());
    }

    // -- FJ-1120: Machine Resource Drift Pattern Classification ------------------

    #[test]
    fn test_drift_pattern_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_pattern_classification(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_drift_pattern_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg1", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("pkg2", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc1", types::ResourceType::Service, types::ResourceStatus::Converged),
        ]));
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("pkg1", types::ResourceType::Package, types::ResourceStatus::Drifted),
            ("svc1", types::ResourceType::Service, types::ResourceStatus::Drifted),
            ("cfg1", types::ResourceType::File, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_machine_resource_drift_pattern_classification(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_drift_pattern_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("n1", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Drifted),
            ("s", types::ResourceType::Service, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_machine_resource_drift_pattern_classification(d.path(), None, true).is_ok());
    }
    #[test]
    fn test_drift_pattern_chronic() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("sick", "2026-01-15T10:00:00Z", vec![
            ("p1", types::ResourceType::Package, types::ResourceStatus::Drifted),
            ("p2", types::ResourceType::Package, types::ResourceStatus::Drifted),
            ("p3", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_machine_resource_drift_pattern_classification(d.path(), None, false).is_ok());
    }

    // -- FJ-1123: Fleet Resource Convergence Window Analysis ---------------------

    #[test]
    fn test_convergence_window_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_convergence_window_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg1", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc1", types::ResourceType::Service, types::ResourceStatus::Converged),
            ("cfg1", types::ResourceType::File, types::ResourceStatus::Drifted),
        ]));
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("pkg1", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc1", types::ResourceType::Service, types::ResourceStatus::Failed),
        ]));
        assert!(cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_convergence_window_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("n1", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("s", types::ResourceType::Service, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, true).is_ok());
    }
    #[test]
    fn test_convergence_window_fleet_average() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("a", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        wr(d.path(), &mk("b", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, false).is_ok());
    }
}
