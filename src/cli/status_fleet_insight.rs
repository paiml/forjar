//! Phase 101 — Fleet Insight & Dependency Quality: status commands (FJ-1069, FJ-1072, FJ-1075).

use std::path::Path;

use crate::core::{state, types};
use super::helpers::discover_machines;

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Minimal RFC-3339 timestamp parser returning seconds since Unix epoch.
fn parse_rfc3339_to_epoch(s: &str) -> Option<u64> {
    if s.len() < 19 { return None; }
    let year: u64 = s.get(0..4)?.parse().ok()?;
    let month: u64 = s.get(5..7)?.parse().ok()?;
    let day: u64 = s.get(8..10)?.parse().ok()?;
    let hour: u64 = s.get(11..13)?.parse().ok()?;
    let min: u64 = s.get(14..16)?.parse().ok()?;
    let sec: u64 = s.get(17..19)?.parse().ok()?;
    let mut days: u64 = 0;
    for y in 1970..year {
        days += if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 { 366 } else { 365 };
    }
    let table = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30];
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mut md: u64 = 0;
    for m in 1..month.min(13) {
        md += table[m as usize];
        if m == 2 && leap { md += 1; }
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

/// Default staleness threshold in seconds (7 days).
const STALENESS_THRESHOLD_SECS: u64 = 7 * 24 * 3600;

// ── FJ-1069: Fleet Resource Staleness Report ────────────────────────────────

/// Compute staleness for a single machine lock file.
fn staleness_row(state_dir: &Path, m: &str, now: u64) -> Option<(String, String, u64, bool)> {
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
    let rows: Vec<_> = machines.iter().filter_map(|m| staleness_row(state_dir, m, now)).collect();
    if json {
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, ts, age, stale)| {
            serde_json::json!({"machine": m, "generated_at": ts, "age_days": *age / 86_400, "stale": *stale})
        }).collect();
        println!("{}", serde_json::to_string_pretty(
            &serde_json::json!({"staleness_report": entries})
        ).unwrap_or_default());
    } else {
        println!("=== Fleet Resource Staleness Report ===");
        if rows.is_empty() { println!("  No machine state found."); }
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
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, counts)| {
            serde_json::json!({"machine": m, "types": counts})
        }).collect();
        println!("{}", serde_json::to_string_pretty(
            &serde_json::json!({"type_distribution": entries})
        ).unwrap_or_default());
    } else {
        println!("=== Machine Resource Type Distribution ===");
        if rows.is_empty() { println!("  No machine state found."); }
        for (m, counts) in &rows {
            let parts: Vec<String> = counts
                .iter()
                .map(|(t, c)| format!("{}={}", t, c))
                .collect();
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
            rows.push((m.clone(), score, converged_pct * 100.0, drifted_pct * 100.0, failed_pct * 100.0));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, score, conv, drift, fail)| {
            serde_json::json!({
                "machine": m,
                "health_score": (*score * 10.0).round() / 10.0,
                "converged_pct": (*conv * 10.0).round() / 10.0,
                "drifted_pct": (*drift * 10.0).round() / 10.0,
                "failed_pct": (*fail * 10.0).round() / 10.0,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(
            &serde_json::json!({"health_scores": entries})
        ).unwrap_or_default());
    } else {
        println!("=== Fleet Machine Health Score ===");
        if rows.is_empty() { println!("  No machine state found."); }
        for (m, score, conv, drift, fail) in &rows {
            println!(
                "  {}: score={:.1}, converged={:.1}%, drifted={:.1}%, failed={:.1}%",
                m, score, conv, drift, fail,
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

    // ── FJ-1069: Staleness Report ─────────────────────────────────────────

    #[test]
    fn test_staleness_report_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_staleness_report_recent_data() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-02-28T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_staleness_report_stale_data() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2024-01-01T00:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_staleness_report_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2024-01-01T00:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_staleness_report_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2024-01-01T00:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        wr(d.path(), &mk("db", "2026-02-28T10:00:00Z", vec![
            ("svc", types::ResourceType::Service, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), Some("web"), false).is_ok());
    }

    // ── FJ-1072: Type Distribution ────────────────────────────────────────

    #[test]
    fn test_type_distribution_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_type_distribution_mixed_types() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg1", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("pkg2", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Converged),
            ("cfg", types::ResourceType::File, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_type_distribution_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("f1", types::ResourceType::File, types::ResourceStatus::Converged),
            ("f2", types::ResourceType::File, types::ResourceStatus::Converged),
            ("s1", types::ResourceType::Service, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_type_distribution_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("svc", types::ResourceType::Service, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_machine_resource_type_distribution(d.path(), Some("db"), false).is_ok());
    }

    #[test]
    fn test_type_distribution_multiple_machines() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("cfg", types::ResourceType::File, types::ResourceStatus::Converged),
        ]));
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("svc", types::ResourceType::Service, types::ResourceStatus::Converged),
            ("mnt", types::ResourceType::Mount, types::ResourceStatus::Converged),
            ("net", types::ResourceType::Network, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, false).is_ok());
    }

    // ── FJ-1075: Health Score ─────────────────────────────────────────────

    #[test]
    fn test_health_score_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_health_score_all_converged() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Converged),
        ]));
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_health_score_mixed_status() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("svc", types::ResourceType::Service, types::ResourceStatus::Drifted),
            ("cfg", types::ResourceType::File, types::ResourceStatus::Failed),
        ]));
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_health_score_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("n1", "2026-01-15T10:00:00Z", vec![
            ("p", types::ResourceType::Package, types::ResourceStatus::Converged),
            ("s", types::ResourceType::Service, types::ResourceStatus::Drifted),
        ]));
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_health_score_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![
            ("pkg", types::ResourceType::Package, types::ResourceStatus::Converged),
        ]));
        wr(d.path(), &mk("db", "2026-01-15T10:00:00Z", vec![
            ("svc", types::ResourceType::Service, types::ResourceStatus::Failed),
        ]));
        assert!(cmd_status_fleet_machine_health_score(d.path(), Some("web"), false).is_ok());
    }

    #[test]
    fn test_health_score_empty_resources() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("web", "2026-01-15T10:00:00Z", vec![]));
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, false).is_ok());
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

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
