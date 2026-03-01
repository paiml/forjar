//! Phase 104 — Operational Maturity & Dependency Governance: status commands (FJ-1093, FJ-1096, FJ-1099).

use std::collections::BTreeSet;
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

/// Classify resources in a lock file into (converged, drifted, failed, unknown).
fn classify_resources(lock: &types::StateLock) -> (usize, usize, usize, usize) {
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

/// Count distinct resource types present in a lock file.
fn distinct_resource_types(lock: &types::StateLock) -> usize {
    let set: BTreeSet<String> = lock
        .resources
        .values()
        .map(|rl| rl.resource_type.to_string())
        .collect();
    set.len()
}

// -- FJ-1093: Fleet Resource Maturity Index ----------------------------------

/// FJ-1093: `status --fleet-resource-maturity-index`
///
/// Compute maturity index per machine.
/// Score = (converged_pct * 0.5 + distinct_types * 10 + total_resources * 5), clamped 0-100.
pub(crate) fn cmd_status_fleet_resource_maturity_index(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, f64, f64, usize, usize)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (c, d, f, u) = classify_resources(&lock);
            let total = c + d + f + u;
            let converged_pct = if total > 0 {
                c as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            let distinct = distinct_resource_types(&lock);
            let raw = converged_pct * 0.5 + distinct as f64 * 10.0 + total as f64 * 5.0;
            let score = raw.clamp(0.0, 100.0);
            rows.push((m.clone(), score, converged_pct, distinct, total));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, score, conv, dist, tot)| {
            serde_json::json!({
                "machine": m,
                "maturity_score": (*score * 10.0).round() / 10.0,
                "converged_pct": (*conv * 10.0).round() / 10.0,
                "distinct_types": *dist,
                "total_resources": *tot,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(
            &serde_json::json!({"maturity_index": entries})
        ).unwrap_or_default());
    } else {
        println!("=== Fleet Resource Maturity Index ===");
        if rows.is_empty() { println!("  No machine state found."); }
        for (m, score, conv, dist, tot) in &rows {
            println!(
                "  {}: score={:.1}, converged={:.1}%, types={}, resources={}",
                m, score, conv, dist, tot,
            );
        }
    }
    Ok(())
}

// -- FJ-1096: Machine Resource Convergence Stability Index -------------------

/// FJ-1096: `status --machine-resource-convergence-stability-index`
///
/// Score convergence stability per machine.
/// stability = converged / total * 100 if total > 0, else 0.
pub(crate) fn cmd_status_machine_resource_convergence_stability_index(
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
            let stability = if total > 0 {
                c as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            rows.push((m.clone(), stability, c, total));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, stab, conv, tot)| {
            serde_json::json!({
                "machine": m,
                "stability": (*stab * 10.0).round() / 10.0,
                "converged": *conv,
                "total": *tot,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(
            &serde_json::json!({"convergence_stability": entries})
        ).unwrap_or_default());
    } else {
        println!("=== Machine Resource Convergence Stability Index ===");
        if rows.is_empty() { println!("  No machine state found."); }
        for (m, stab, conv, tot) in &rows {
            println!(
                "  {}: stability={:.1}%, converged={}/{}",
                m, stab, conv, tot,
            );
        }
    }
    Ok(())
}

// -- FJ-1099: Fleet Resource Drift Pattern Analysis --------------------------

/// Classify a drift pattern from drifted/total counts.
fn classify_drift_pattern(drifted: usize, total: usize) -> &'static str {
    if drifted == 0 {
        "none"
    } else if drifted == 1 {
        "sporadic"
    } else if drifted == total {
        "cascading"
    } else {
        "chronic"
    }
}

/// FJ-1099: `status --fleet-resource-drift-pattern-analysis`
///
/// Classify drift patterns per machine:
/// - none: zero drifted resources
/// - sporadic: exactly one drifted resource
/// - chronic: more than one but less than total drifted
/// - cascading: all resources drifted
pub(crate) fn cmd_status_fleet_resource_drift_pattern_analysis(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, String, usize, usize)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (c, d, f, u) = classify_resources(&lock);
            let total = c + d + f + u;
            let pattern = classify_drift_pattern(d, total).to_string();
            rows.push((m.clone(), pattern, d, total));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, pat, dr, tot)| {
            serde_json::json!({
                "machine": m,
                "pattern": pat,
                "drifted": *dr,
                "total": *tot,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(
            &serde_json::json!({"drift_patterns": entries})
        ).unwrap_or_default());
    } else {
        println!("=== Fleet Resource Drift Pattern Analysis ===");
        if rows.is_empty() { println!("  No machine state found."); }
        for (m, pat, dr, tot) in &rows {
            println!(
                "  {}: pattern={}, drifted={}/{}",
                m, pat, dr, tot,
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mk(machine: &str, ts: &str, res: Vec<(&str, types::ResourceType, types::ResourceStatus)>) -> types::StateLock {
        let mut m = indexmap::IndexMap::new();
        for (id, rt, st) in res {
            m.insert(id.to_string(), types::ResourceLock {
                resource_type: rt, status: st,
                applied_at: Some(ts.into()),
                duration_seconds: Some(1.0), hash: "abc".into(), details: HashMap::new(),
            });
        }
        types::StateLock {
            schema: "1".into(), machine: machine.into(), hostname: machine.into(),
            generated_at: ts.into(), generator: "test".into(),
            blake3_version: "1.0".into(), resources: m,
        }
    }

    fn wr(dir: &std::path::Path, lock: &types::StateLock) {
        let d = dir.join(&lock.machine);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("state.lock.yaml"), serde_yaml_ng::to_string(lock).unwrap()).unwrap();
    }

    // -- FJ-1093: Maturity Index -----------------------------------------------

    #[test]
    fn test_maturity_index_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_maturity_index_all_converged() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_maturity_index_mixed_types() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Drifted),
            ("cfg", types::ResourceType::File, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_maturity_index_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("n1", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("s", types::ResourceType::Service, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_maturity_index_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("svc", types::ResourceType::Service, types::ResourceStatus::Failed),
        ]));
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), Some("web"), false).is_ok());
    }

    #[test]
    fn test_maturity_index_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_maturity_index_score_clamped() {
        let d = tempfile::tempdir().unwrap();
        // Many resources + many types + high convergence => raw > 100, should clamp to 100
        wr(d.path(), &mk("big", "2026-01-15T10:00:00Z", vec![
            ("p1", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("p2", types::ResourceType::File, types::ResourceStatus::Converged),
            ("p3", types::ResourceType::Service, types::ResourceStatus::Converged),
            ("p4", types::ResourceType::Mount, types::ResourceStatus::Converged),
            ("p5", types::ResourceType::User, types::ResourceStatus::Converged),
            ("p6", types::ResourceType::Docker, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }

    // -- FJ-1096: Convergence Stability Index ----------------------------------

    #[test]
    fn test_convergence_stability_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_stability_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_stability_all_converged() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_machine_resource_convergence_stability_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_stability_partial() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Drifted),
            ("cfg", types::ResourceType::File, types::ResourceStatus::Failed),
        ]));
        assert!(cmd_status_machine_resource_convergence_stability_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_stability_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("n1", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("s", types::ResourceType::Service, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_machine_resource_convergence_stability_index(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_convergence_stability_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("svc", types::ResourceType::Service, types::ResourceStatus::Failed),
        ]));
        assert!(cmd_status_machine_resource_convergence_stability_index(d.path(), Some("db"), false).is_ok());
    }

    #[test]
    fn test_convergence_stability_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(cmd_status_machine_resource_convergence_stability_index(d.path(), None, false).is_ok());
    }

    // -- FJ-1099: Drift Pattern Analysis ---------------------------------------

    #[test]
    fn test_drift_pattern_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_pattern_none() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_pattern_sporadic() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Drifted),
            ("cfg", types::ResourceType::File, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_pattern_chronic() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Drifted),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Drifted),
            ("cfg", types::ResourceType::File, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_pattern_cascading() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Drifted),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_pattern_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("n1", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Drifted),
            ("s", types::ResourceType::Service, types::ResourceStatus::Converged),
            ("f", types::ResourceType::File, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_drift_pattern_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Drifted),
        ]));
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("svc", types::ResourceType::Service, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), Some("web"), false).is_ok());
    }

    // -- Helper unit tests -----------------------------------------------------

    #[test]
    fn test_classify_resources_all_statuses() {
        let lock = mk("m", "2026-01-15T10:00:00Z", vec![
            ("a", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("b", types::ResourceType::Service, types::ResourceStatus::Drifted),
            ("c", types::ResourceType::File, types::ResourceStatus::Failed),
            ("d", types::ResourceType::File, types::ResourceStatus::Unknown),
        ]);
        assert_eq!(classify_resources(&lock), (1, 1, 1, 1));
    }

    #[test]
    fn test_classify_resources_empty() {
        let lock = mk("m", "2026-01-15T10:00:00Z", vec![]);
        assert_eq!(classify_resources(&lock), (0, 0, 0, 0));
    }

    #[test]
    fn test_distinct_resource_types() {
        let lock = mk("m", "2026-01-15T10:00:00Z", vec![
            ("a", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("b", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("c", types::ResourceType::Service, types::ResourceStatus::Converged),
        ]);
        assert_eq!(distinct_resource_types(&lock), 2);
    }

    #[test]
    fn test_distinct_resource_types_empty() {
        let lock = mk("m", "2026-01-15T10:00:00Z", vec![]);
        assert_eq!(distinct_resource_types(&lock), 0);
    }

    #[test]
    fn test_classify_drift_pattern_none() {
        assert_eq!(classify_drift_pattern(0, 5), "none");
    }

    #[test]
    fn test_classify_drift_pattern_sporadic() {
        assert_eq!(classify_drift_pattern(1, 5), "sporadic");
    }

    #[test]
    fn test_classify_drift_pattern_chronic() {
        assert_eq!(classify_drift_pattern(3, 5), "chronic");
    }

    #[test]
    fn test_classify_drift_pattern_cascading() {
        assert_eq!(classify_drift_pattern(5, 5), "cascading");
    }
}
