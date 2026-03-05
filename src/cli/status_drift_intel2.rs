//! Phase 106 — Drift Intelligence & Recovery Analytics: status commands (FJ-1109, FJ-1112, FJ-1115).

use std::collections::BTreeMap;
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

// -- FJ-1109: Fleet Resource Type Drift Correlation -------------------------

/// FJ-1109: `status --fleet-resource-type-drift-correlation`
///
/// Correlate drift rates by resource type across the entire fleet.
/// Groups resources by their `resource_type`, counts drifted vs total per type.
fn gather_type_totals(state_dir: &Path, machines: &[String]) -> BTreeMap<String, (usize, usize)> {
    let mut type_totals: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            for rl in lock.resources.values() {
                let entry = type_totals
                    .entry(rl.resource_type.to_string())
                    .or_insert((0, 0));
                if rl.status == types::ResourceStatus::Drifted {
                    entry.0 += 1;
                }
                entry.1 += 1;
            }
        }
    }
    type_totals
}
fn drift_pct(drifted: usize, total: usize) -> f64 {
    if total > 0 {
        drifted as f64 / total as f64 * 100.0
    } else {
        0.0
    }
}
pub(crate) fn cmd_status_fleet_resource_type_drift_correlation(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let type_totals = gather_type_totals(state_dir, &filtered_machines(state_dir, machine));
    if json {
        let entries: Vec<serde_json::Value> = type_totals.iter()
            .map(|(t, (d, tot))| serde_json::json!({"type": t, "drift_pct": (drift_pct(*d, *tot) * 10.0).round() / 10.0, "drifted": *d, "total": *tot}))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"fleet_resource_type_drift_correlation": entries})
            )
            .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Resource Type Drift Correlation ===");
        if type_totals.is_empty() {
            println!("  No resource data found.");
        }
        for (t, (d, tot)) in &type_totals {
            println!(
                "  {}: {:.1}% drift ({} drifted / {} total)",
                t,
                drift_pct(*d, *tot),
                d,
                tot
            );
        }
    }
    Ok(())
}

// -- FJ-1112: Machine Resource Apply Cadence Report -------------------------

/// FJ-1112: `status --machine-resource-apply-cadence-report`
///
/// Report per-machine apply frequency based on lock data. Uses `generated_at`
/// timestamps from the state lock to determine when applies last occurred.
pub(crate) fn cmd_status_machine_resource_apply_cadence_report(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, usize, String)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let resource_count = lock.resources.len();
            let last_apply = lock.generated_at.clone();
            rows.push((m.clone(), resource_count, last_apply));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(|(m, count, last)| {
                serde_json::json!({
                    "machine": m,
                    "resource_count": *count,
                    "last_apply": last,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"machine_resource_apply_cadence_report": entries})
            )
            .unwrap_or_default()
        );
    } else {
        println!("=== Machine Resource Apply Cadence Report ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, count, last) in &rows {
            println!("  {m}: {count} resources, last apply: {last}",);
        }
    }
    Ok(())
}

// -- FJ-1115: Fleet Resource Drift Recovery Trend ---------------------------

/// FJ-1115: `status --fleet-resource-drift-recovery-trend`
///
/// Track drift recovery trends across the fleet. Computes the ratio of
/// converged resources to total resources per machine as a recovery rate.
fn gather_recovery_rows(state_dir: &Path, machines: &[String]) -> Vec<(String, f64, usize, usize)> {
    let mut rows = Vec::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let total = lock.resources.len();
            let converged = lock
                .resources
                .values()
                .filter(|r| r.status == types::ResourceStatus::Converged)
                .count();
            let pct = if total > 0 {
                converged as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            rows.push((m.clone(), pct, converged, total));
        }
    }
    rows
}
pub(crate) fn cmd_status_fleet_resource_drift_recovery_trend(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let rows = gather_recovery_rows(state_dir, &filtered_machines(state_dir, machine));
    if json {
        let entries: Vec<serde_json::Value> = rows.iter()
            .map(|(m, pct, conv, tot)| serde_json::json!({"machine": m, "recovery_pct": (*pct * 10.0).round() / 10.0, "converged": *conv, "total": *tot}))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"fleet_resource_drift_recovery_trend": entries})
            )
            .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Resource Drift Recovery Trend ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, pct, conv, tot) in &rows {
            println!("  {m}: {pct:.1}% recovery ({conv} converged / {tot} total)");
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

    // -- FJ-1109: Fleet Resource Type Drift Correlation -------------------------

    #[test]
    fn test_type_drift_correlation_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_drift_correlation(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_type_drift_correlation_mixed_types() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg1",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "pkg2",
                        types::ResourceType::Package,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "svc1",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "cfg1",
                        types::ResourceType::File,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "cfg2",
                        types::ResourceType::File,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_type_drift_correlation(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_type_drift_correlation_json() {
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
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "s",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_type_drift_correlation(d.path(), None, true).is_ok());
    }

    // -- FJ-1112: Machine Resource Apply Cadence Report -------------------------

    #[test]
    fn test_apply_cadence_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_cadence_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_cadence_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-02-20T14:30:00Z",
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
                    (
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-02-19T08:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_machine_resource_apply_cadence_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_cadence_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "n1",
                "2026-01-15T10:00:00Z",
                vec![(
                    "p",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_machine_resource_apply_cadence_report(d.path(), None, true).is_ok());
    }

    // -- FJ-1115: Fleet Resource Drift Recovery Trend ---------------------------

    #[test]
    fn test_drift_recovery_trend_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_recovery_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_recovery_trend_with_data() {
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
                    (
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Drifted,
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
        assert!(cmd_status_fleet_resource_drift_recovery_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_recovery_trend_json() {
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
        assert!(cmd_status_fleet_resource_drift_recovery_trend(d.path(), None, true).is_ok());
    }
}
