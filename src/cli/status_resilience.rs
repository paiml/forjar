//! Phase 105 — Fleet Resilience & Convergence Analysis: status commands (FJ-1101, FJ-1104, FJ-1107).

use std::path::Path;

use super::helpers::discover_machines;
use crate::core::{state, types};

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
            println!(
                "  {}: {:.1}% success ({} converged / {} total)",
                m, pct, conv, tot,
            );
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
            println!("  {}: {} converged, {} drifted, {} failed", m, c, d, f,);
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
            println!("  fleet_average: {:.1}%", fleet_avg);
            for (m, pct, _c, _t) in &rows {
                let gap = fleet_avg - *pct;
                println!("  {}: {:.1}% (gap: {:.1}%)", m, pct, gap,);
            }
        }
    }
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
        std::fs::write(
            d.join("state.lock.yaml"),
            serde_yaml_ng::to_string(lock).unwrap(),
        )
        .unwrap();
    }

    // -- FJ-1101: Apply Success Trend ------------------------------------------

    #[test]
    fn test_apply_success_trend_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_apply_success_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_success_trend_all_converged() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_apply_success_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_success_trend_mixed() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Failed,
                    ),
                    (
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_apply_success_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_success_trend_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "n1",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "p",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "s",
                        types::ResourceType::Service,
                        types::ResourceStatus::Failed,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_apply_success_trend(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_apply_success_trend_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Failed,
                )],
            ),
        );
        assert!(
            cmd_status_fleet_resource_apply_success_trend(d.path(), Some("web"), false).is_ok()
        );
    }

    #[test]
    fn test_apply_success_trend_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(cmd_status_fleet_resource_apply_success_trend(d.path(), None, false).is_ok());
    }

    // -- FJ-1104: Drift Age Distribution ---------------------------------------

    #[test]
    fn test_drift_age_distribution_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_age_distribution(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_age_distribution_all_converged() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_machine_resource_drift_age_distribution(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_age_distribution_mixed() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Failed,
                    ),
                ],
            ),
        );
        assert!(cmd_status_machine_resource_drift_age_distribution(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_age_distribution_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "n1",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "p",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "s",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "f",
                        types::ResourceType::File,
                        types::ResourceStatus::Failed,
                    ),
                ],
            ),
        );
        assert!(cmd_status_machine_resource_drift_age_distribution(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_drift_age_distribution_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Drifted,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(
            cmd_status_machine_resource_drift_age_distribution(d.path(), Some("db"), false).is_ok()
        );
    }

    #[test]
    fn test_drift_age_distribution_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(cmd_status_machine_resource_drift_age_distribution(d.path(), None, false).is_ok());
    }

    // -- FJ-1107: Convergence Gap Analysis -------------------------------------

    #[test]
    fn test_convergence_gap_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_gap_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_gap_single_machine() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_convergence_gap_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_gap_multiple_machines() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Failed,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_convergence_gap_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_gap_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![(
                    "p",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![(
                    "s",
                    types::ResourceType::Service,
                    types::ResourceStatus::Failed,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_convergence_gap_analysis(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_convergence_gap_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Failed,
                )],
            ),
        );
        assert!(
            cmd_status_fleet_resource_convergence_gap_analysis(d.path(), Some("web"), false)
                .is_ok()
        );
    }

    #[test]
    fn test_convergence_gap_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(cmd_status_fleet_resource_convergence_gap_analysis(d.path(), None, false).is_ok());
    }

    // -- Helper unit tests -----------------------------------------------------

    #[test]
    fn test_classify_resources_all_statuses() {
        let lock = mk(
            "m",
            "2026-01-15T10:00:00Z",
            vec![
                (
                    "a",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                ),
                (
                    "b",
                    types::ResourceType::Service,
                    types::ResourceStatus::Drifted,
                ),
                (
                    "c",
                    types::ResourceType::File,
                    types::ResourceStatus::Failed,
                ),
                (
                    "d",
                    types::ResourceType::File,
                    types::ResourceStatus::Unknown,
                ),
            ],
        );
        assert_eq!(classify_resources(&lock), (1, 1, 1, 1));
    }

    #[test]
    fn test_classify_resources_empty() {
        let lock = mk("m", "2026-01-15T10:00:00Z", vec![]);
        assert_eq!(classify_resources(&lock), (0, 0, 0, 0));
    }
}
