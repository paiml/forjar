//! Phase 100 — Fleet Apply Cadence, Error Classification, Convergence Summary.

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

// ── FJ-1061: Fleet Apply Cadence ────────────────────────────────────────────

/// FJ-1061: `status --fleet-apply-cadence`
///
/// Reports the age of each machine's lock file based on `generated_at`.
pub(crate) fn cmd_status_fleet_apply_cadence(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let now = now_epoch();
    let mut rows: Vec<(String, String, f64)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let age_hours = match parse_rfc3339_to_epoch(&lock.generated_at) {
                Some(epoch) if now >= epoch => (now - epoch) as f64 / 3600.0,
                _ => 0.0,
            };
            rows.push((m.clone(), lock.generated_at.clone(), age_hours));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, ts, age)| {
            serde_json::json!({"machine": m, "last_apply": ts, "age_hours": (*age as u64)})
        }).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"fleet_apply_cadence":{"machines": entries}})
            )
            .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Apply Cadence ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, _ts, age) in &rows {
            println!("  {}: last apply {}h ago", m, *age as u64);
        }
    }
    Ok(())
}

// ── FJ-1064: Machine Resource Error Classification ──────────────────────────

/// FJ-1064: `status --machine-resource-error-classification`
///
/// For each machine, classify resources by status: converged, drifted, failed, unknown.
pub(crate) fn cmd_status_machine_resource_error_classification(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, usize, usize, usize, usize)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (c, d, f, u) = classify_resources(&lock);
            rows.push((m.clone(), c, d, f, u));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, c, d, f, u)| {
            serde_json::json!({"machine": m, "converged": c, "drifted": d, "failed": f, "unknown": u})
        }).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"error_classification":{"machines": entries}})
            )
            .unwrap_or_default()
        );
    } else {
        println!("=== Resource Error Classification ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, c, d, f, u) in &rows {
            println!(
                "  {}: converged={}, drifted={}, failed={}, unknown={}",
                m, c, d, f, u
            );
        }
    }
    Ok(())
}

// ── FJ-1067: Fleet Resource Convergence Summary ─────────────────────────────

/// FJ-1067: `status --fleet-resource-convergence-summary`
///
/// Compute overall fleet convergence: total resources, converged count, percentage.
pub(crate) fn cmd_status_fleet_resource_convergence_summary(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut total = 0usize;
    let mut converged = 0usize;
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (c, _d, _f, _u) = classify_resources(&lock);
            total += lock.resources.len();
            converged += c;
        }
    }
    let pct = if total > 0 {
        converged as f64 / total as f64 * 100.0
    } else {
        0.0
    };
    if json {
        println!("{}", serde_json::to_string_pretty(
            &serde_json::json!({"convergence_summary":{"total": total, "converged": converged, "percentage": (pct * 10.0).round() / 10.0}})
        ).unwrap_or_default());
    } else {
        println!("=== Fleet Convergence Summary ===");
        println!(
            "  Total: {}, Converged: {}, Convergence: {:.1}%",
            total, converged, pct
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mk(
        machine: &str,
        res: Vec<(&str, types::ResourceType, types::ResourceStatus)>,
    ) -> types::StateLock {
        let mut m = indexmap::IndexMap::new();
        for (id, rt, st) in res {
            m.insert(
                id.to_string(),
                types::ResourceLock {
                    resource_type: rt,
                    status: st,
                    applied_at: Some("2026-01-15T10:00:00Z".into()),
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
            generated_at: "2026-01-15T10:00:00Z".into(),
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

    // ── FJ-1061: Fleet Apply Cadence ────────────────────────────────────────

    #[test]
    fn test_apply_cadence_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_apply_cadence(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_cadence_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_apply_cadence(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_cadence_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_apply_cadence(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_apply_cadence_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
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
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_apply_cadence(d.path(), Some("web"), false).is_ok());
    }

    // ── FJ-1064: Error Classification ───────────────────────────────────────

    #[test]
    fn test_error_classification_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_classification(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_error_classification_mixed_status() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
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
        assert!(cmd_status_machine_resource_error_classification(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_error_classification_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "db",
                vec![
                    (
                        "f1",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "f2",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_machine_resource_error_classification(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_error_classification_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
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
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Drifted,
                )],
            ),
        );
        assert!(
            cmd_status_machine_resource_error_classification(d.path(), Some("db"), false).is_ok()
        );
    }

    // ── FJ-1067: Convergence Summary ────────────────────────────────────────

    #[test]
    fn test_convergence_summary_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_summary_all_converged() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
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
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_summary_partial() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
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
        wr(
            d.path(),
            &mk(
                "db",
                vec![(
                    "f1",
                    types::ResourceType::File,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_summary_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "n1",
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
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, true).is_ok());
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    #[test]
    fn test_classify_resources_all_statuses() {
        let lock = mk(
            "m",
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
        let lock = mk("m", vec![]);
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
