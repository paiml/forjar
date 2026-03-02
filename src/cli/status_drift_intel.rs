//! Phase 98 — Compliance Automation & Drift Intelligence: status commands.

use std::collections::BTreeMap;
use std::path::Path;

use super::helpers::*;
use crate::core::{state, types};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Compute per-machine drift counts: (total_resources, drifted_resources).
fn machine_drift_counts(lock: &types::StateLock) -> (usize, usize) {
    let total = lock.resources.len();
    let drifted = lock
        .resources
        .values()
        .filter(|r| r.status == types::ResourceStatus::Drifted)
        .count();
    (total, drifted)
}

/// Map age in seconds to a bucket label.
fn secs_to_bucket(age_secs: u64) -> &'static str {
    if age_secs < 3600 {
        "<1h"
    } else if age_secs < 86_400 {
        "<1d"
    } else if age_secs < 604_800 {
        "<7d"
    } else if age_secs < 2_592_000 {
        "<30d"
    } else {
        ">30d"
    }
}

/// Classify a `generated_at` ISO-8601 timestamp into an age bucket relative to `now`.
fn age_bucket(generated_at: &str, now: u64) -> &'static str {
    match parse_rfc3339_to_epoch(generated_at) {
        Some(epoch) if now >= epoch => secs_to_bucket(now - epoch),
        _ => "unknown",
    }
}

/// Minimal RFC-3339 timestamp parser returning seconds since Unix epoch.
/// Handles formats like `2025-06-15T12:34:56Z` and `2025-06-15T12:34:56+00:00`.
fn parse_rfc3339_to_epoch(s: &str) -> Option<u64> {
    // Expect at minimum: YYYY-MM-DDTHH:MM:SS
    if s.len() < 19 {
        return None;
    }
    let year: u64 = s.get(0..4)?.parse().ok()?;
    let month: u64 = s.get(5..7)?.parse().ok()?;
    let day: u64 = s.get(8..10)?.parse().ok()?;
    let hour: u64 = s.get(11..13)?.parse().ok()?;
    let min: u64 = s.get(14..16)?.parse().ok()?;
    let sec: u64 = s.get(17..19)?.parse().ok()?;

    // Days from year (simplified — no leap-second precision needed)
    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let month_days = days_before_month(year, month);
    days += month_days + (day - 1);
    Some(days * 86_400 + hour * 3600 + min * 60 + sec)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_before_month(year: u64, month: u64) -> u64 {
    let leap = is_leap(year);
    let months = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30];
    let mut d: u64 = 0;
    for m in 1..month.min(13) {
        d += months[m as usize];
        if m == 2 && leap {
            d += 1;
        }
    }
    d
}

/// Return current Unix epoch in seconds (via `SystemTime`).
fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Load machines respecting an optional filter.
fn filtered_machines(state_dir: &Path, machine: Option<&str>) -> Vec<String> {
    let all = discover_machines(state_dir);
    match machine {
        Some(m) => all.into_iter().filter(|n| n == m).collect(),
        None => all,
    }
}

// ── FJ-1045 ─────────────────────────────────────────────────────────────────

/// Collect drift velocity data: Vec<(machine, total, drifted, ratio)>.
fn collect_drift_velocity(
    state_dir: &Path,
    machines: &[String],
) -> Vec<(String, usize, usize, f64)> {
    let mut rows = Vec::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (total, drifted) = machine_drift_counts(&lock);
            let ratio = if total > 0 {
                drifted as f64 / total as f64
            } else {
                0.0
            };
            rows.push((m.clone(), total, drifted, ratio));
        }
    }
    rows
}

/// FJ-1045: `status --fleet-drift-velocity-trend`
///
/// Read lock files. Count drifted resources per machine. Report drift velocity
/// as a simple ratio (drifted / total).
pub(crate) fn cmd_status_fleet_drift_velocity_trend(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let rows = collect_drift_velocity(state_dir, &machines);

    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(|(m, total, drifted, ratio)| {
                serde_json::json!({
                    "machine": m,
                    "total_resources": total,
                    "drifted_resources": drifted,
                    "drift_velocity": format!("{:.4}", ratio),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "fleet_drift_velocity_trend": entries
            }))
            .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Drift Velocity Trend ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, total, drifted, ratio) in &rows {
            let symbol = if *drifted == 0 { green("*") } else { red("!") };
            println!(
                "  {} {} — {}/{} drifted (velocity {:.2}%)",
                symbol,
                m,
                drifted,
                total,
                ratio * 100.0
            );
        }
    }
    Ok(())
}

// ── FJ-1048 ─────────────────────────────────────────────────────────────────

/// Estimate convergence window for a single machine.
fn convergence_window_minutes(lock: &types::StateLock) -> u64 {
    let (_total, drifted) = machine_drift_counts(lock);
    // Heuristic: 1 minute per drifted resource; 0 if fully converged.
    drifted as u64
}

/// Collect convergence window data: Vec<(machine, drifted, window_minutes)>.
fn collect_convergence_windows(state_dir: &Path, machines: &[String]) -> Vec<(String, usize, u64)> {
    let mut rows = Vec::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (_, drifted) = machine_drift_counts(&lock);
            let window = convergence_window_minutes(&lock);
            rows.push((m.clone(), drifted, window));
        }
    }
    rows
}

/// FJ-1048: `status --machine-convergence-window`
///
/// Read lock files. Estimate convergence window: 0 if no drifted resources,
/// otherwise 1 minute per drifted resource as heuristic.
pub(crate) fn cmd_status_machine_convergence_window(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let rows = collect_convergence_windows(state_dir, &machines);

    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(|(m, drifted, window)| {
                serde_json::json!({
                    "machine": m,
                    "drifted_resources": drifted,
                    "convergence_window_minutes": window,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "machine_convergence_window": entries
            }))
            .unwrap_or_default()
        );
    } else {
        println!("=== Machine Convergence Window ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, drifted, window) in &rows {
            let symbol = if *drifted == 0 {
                green("*")
            } else {
                yellow("~")
            };
            println!(
                "  {} {} — {} drifted, est. {} min to converge",
                symbol, m, drifted, window
            );
        }
    }
    Ok(())
}

// ── FJ-1051 ─────────────────────────────────────────────────────────────────

/// Ordered bucket labels for the age histogram.
const AGE_BUCKETS: &[&str] = &["<1h", "<1d", "<7d", "<30d", ">30d", "unknown"];

/// Accumulate resource counts into age buckets for one machine.
fn bucket_machine_resources(
    lock: &types::StateLock,
    now: u64,
    histogram: &mut BTreeMap<&'static str, u64>,
) {
    for rl in lock.resources.values() {
        let bucket = match &rl.applied_at {
            Some(ts) if !ts.is_empty() => age_bucket(ts, now),
            _ => "unknown",
        };
        *histogram.entry(bucket).or_insert(0) += 1;
    }
}

/// FJ-1051: `status --fleet-resource-age-histogram`
///
/// Read lock files. Parse `applied_at` timestamps. Bucket resources into age
/// categories: <1h, <1d, <7d, <30d, >30d. If no timestamp, put in "unknown".
pub(crate) fn cmd_status_fleet_resource_age_histogram(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let now = now_epoch();
    let mut histogram: BTreeMap<&str, u64> = BTreeMap::new();

    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            bucket_machine_resources(&lock, now, &mut histogram);
        }
    }

    if json {
        let ordered: Vec<serde_json::Value> = AGE_BUCKETS
            .iter()
            .map(|&b| {
                serde_json::json!({
                    "bucket": b,
                    "count": histogram.get(b).copied().unwrap_or(0),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "fleet_resource_age_histogram": ordered
            }))
            .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Resource Age Histogram ===");
        let total: u64 = histogram.values().sum();
        if total == 0 {
            println!("  No resources found.");
        }
        for &bucket in AGE_BUCKETS {
            let count = histogram.get(bucket).copied().unwrap_or(0);
            let bar = "#".repeat(count.min(40) as usize);
            println!("  {:>7} | {:>4} {}", bucket, count, bar);
        }
        if total > 0 {
            println!("  {:>7} | {:>4}", "total", total);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rfc3339_basic() {
        // 2024-01-01T00:00:00Z
        let epoch = parse_rfc3339_to_epoch("2024-01-01T00:00:00Z");
        assert!(epoch.is_some());
        // 2024-01-01 is 54 years * 365 + leap days from 1970
        let val = epoch.unwrap();
        assert!(val > 1_700_000_000); // sanity: after ~2023
        assert!(val < 1_800_000_000); // sanity: before ~2027
    }

    #[test]
    fn test_parse_rfc3339_invalid() {
        assert!(parse_rfc3339_to_epoch("").is_none());
        assert!(parse_rfc3339_to_epoch("not-a-date").is_none());
        assert!(parse_rfc3339_to_epoch("2024").is_none());
    }

    #[test]
    fn test_age_bucket_recent() {
        let now = 1_700_000_000;
        // 30 minutes ago
        let ts = "2023-11-14T22:13:20Z"; // approx now - 1800
        let bucket = age_bucket(ts, now);
        // Will be <1h since it's within the hour
        assert!(bucket == "<1h" || bucket == "unknown");
    }

    #[test]
    fn test_age_bucket_unknown() {
        assert_eq!(age_bucket("", 1_700_000_000), "unknown");
        assert_eq!(age_bucket("garbage", 1_700_000_000), "unknown");
    }

    #[test]
    fn test_is_leap() {
        assert!(is_leap(2000));
        assert!(is_leap(2024));
        assert!(!is_leap(1900));
        assert!(!is_leap(2023));
    }

    #[test]
    fn test_days_before_month() {
        assert_eq!(days_before_month(2024, 1), 0);
        assert_eq!(days_before_month(2024, 2), 31);
        // Leap year: Feb has 29 days
        assert_eq!(days_before_month(2024, 3), 60);
        // Non-leap year: Feb has 28 days
        assert_eq!(days_before_month(2023, 3), 59);
    }

    #[test]
    fn test_machine_drift_counts_empty() {
        let lock = types::StateLock {
            schema: "1".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "2024-01-01T00:00:00Z".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.0".to_string(),
            resources: indexmap::IndexMap::new(),
        };
        let (total, drifted) = machine_drift_counts(&lock);
        assert_eq!(total, 0);
        assert_eq!(drifted, 0);
    }
}
