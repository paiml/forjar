//! Phase 102 — Resource Intelligence & Topology Insight: status commands (FJ-1077, FJ-1080, FJ-1083).

use std::path::Path;

use super::helpers::discover_machines;
use crate::core::{state, types};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Minimal RFC-3339 timestamp parser returning seconds since Unix epoch.
fn parse_rfc3339_to_epoch(s: &str) -> Option<u64> {
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
fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

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

    // ── FJ-1077: Dependency Lag ──────────────────────────────────────────

    #[test]
    fn test_dependency_lag_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_dependency_lag_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_dependency_lag_all_converged() {
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
        assert!(cmd_status_fleet_resource_dependency_lag_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_dependency_lag_mixed_status() {
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
        assert!(cmd_status_fleet_resource_dependency_lag_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_dependency_lag_json() {
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
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_dependency_lag_report(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_dependency_lag_filter() {
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
            cmd_status_fleet_resource_dependency_lag_report(d.path(), Some("db"), false).is_ok()
        );
    }

    #[test]
    fn test_dependency_lag_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(cmd_status_fleet_resource_dependency_lag_report(d.path(), None, false).is_ok());
    }

    // ── FJ-1080: Convergence Rate Trend ──────────────────────────────────

    #[test]
    fn test_convergence_rate_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_rate_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_rate_all_converged() {
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
        assert!(cmd_status_machine_resource_convergence_rate_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_rate_mixed() {
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
                    (
                        "mnt",
                        types::ResourceType::Mount,
                        types::ResourceStatus::Unknown,
                    ),
                ],
            ),
        );
        assert!(cmd_status_machine_resource_convergence_rate_trend(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_rate_json() {
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
                ],
            ),
        );
        assert!(cmd_status_machine_resource_convergence_rate_trend(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_convergence_rate_filter() {
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
            cmd_status_machine_resource_convergence_rate_trend(d.path(), Some("web"), false)
                .is_ok()
        );
    }

    #[test]
    fn test_convergence_rate_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(cmd_status_machine_resource_convergence_rate_trend(d.path(), None, false).is_ok());
    }

    // ── FJ-1083: Apply Lag ───────────────────────────────────────────────

    #[test]
    fn test_apply_lag_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_apply_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_lag_recent_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-02-28T10:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_apply_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_lag_old_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2024-01-01T00:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_apply_lag(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_lag_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2024-06-15T12:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_apply_lag(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_apply_lag_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2024-01-01T00:00:00Z",
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
                "2026-02-28T10:00:00Z",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_apply_lag(d.path(), Some("web"), false).is_ok());
    }

    #[test]
    fn test_apply_lag_multiple_machines() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-01T00:00:00Z",
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
                "2026-02-15T06:00:00Z",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "cache",
                "2025-12-01T00:00:00Z",
                vec![(
                    "cfg",
                    types::ResourceType::File,
                    types::ResourceStatus::Drifted,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_apply_lag(d.path(), None, false).is_ok());
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

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

    #[test]
    fn test_parse_rfc3339_valid() {
        let e = parse_rfc3339_to_epoch("2024-01-01T00:00:00Z");
        assert!(e.is_some());
        assert!(e.unwrap() > 1_700_000_000 && e.unwrap() < 1_800_000_000);
    }

    #[test]
    fn test_parse_rfc3339_invalid() {
        assert!(parse_rfc3339_to_epoch("").is_none());
        assert!(parse_rfc3339_to_epoch("short").is_none());
        assert!(parse_rfc3339_to_epoch("not-a-timestamp!!").is_none());
    }
}
