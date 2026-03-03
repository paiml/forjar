//! Phase 99 — Fleet Security & Resource Freshness: status commands.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::helpers::*;
use crate::core::{state, types};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Count security-relevant properties for a single machine's lock file.
/// Returns (secret_refs_count, privileged_count, tls_count).
pub(super) fn security_counts(lock: &types::StateLock) -> (usize, usize, usize) {
    let mut secret_refs = 0usize;
    let mut privileged = 0usize;
    let mut tls = 0usize;
    for (id, rl) in &lock.resources {
        if let Some(val) = rl.details.get("secret_refs") {
            secret_refs += match val {
                serde_yaml_ng::Value::Number(n) => n.as_u64().unwrap_or(1) as usize,
                _ => 1,
            };
        }
        if rl.resource_type == types::ResourceType::Service {
            privileged += 1;
        }
        let id_lower = id.to_lowercase();
        if id_lower.contains("tls") || id_lower.contains("ssl") || id_lower.contains("cert") {
            tls += 1;
        }
    }
    (secret_refs, privileged, tls)
}

/// Classify overall security posture based on counts.
pub(super) fn classify_posture(secret_refs: usize, privileged: usize) -> &'static str {
    if secret_refs == 0 && privileged == 0 {
        "good"
    } else if secret_refs > 5 || privileged > 3 {
        "needs-attention"
    } else {
        "moderate"
    }
}

/// Minimal RFC-3339 timestamp parser returning seconds since Unix epoch.
pub(super) fn parse_rfc3339_to_epoch(s: &str) -> Option<u64> {
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
pub(super) fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Compute freshness index (0-100) based on age of `generated_at`.
pub(super) fn freshness_score(generated_at: &str, now: u64) -> u64 {
    match parse_rfc3339_to_epoch(generated_at) {
        Some(epoch) if now >= epoch => {
            let age = now - epoch;
            if age < 3600 {
                100
            } else if age < 86_400 {
                80
            } else if age < 604_800 {
                60
            } else if age < 2_592_000 {
                30
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// Load machines respecting an optional filter.
pub(super) fn filtered_machines(state_dir: &Path, machine: Option<&str>) -> Vec<String> {
    let all = discover_machines(state_dir);
    match machine {
        Some(m) => all.into_iter().filter(|n| n == m).collect(),
        None => all,
    }
}

// ── FJ-1053: Fleet Security Posture Summary ────────────────────────────────

/// FJ-1053: `status --fleet-security-posture-summary`
pub(crate) fn cmd_status_fleet_security_posture_summary(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let mut rows: Vec<(String, usize, usize, usize, &str)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let (sr, priv_c, tls) = security_counts(&lock);
            rows.push((m.clone(), sr, priv_c, tls, classify_posture(sr, priv_c)));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, sr, p, t, pos)| {
            serde_json::json!({"machine":m,"secret_refs":sr,"privileged":p,"tls_resources":t,"posture":pos})
        }).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"fleet_security_posture":{"machines":entries}})
            )
            .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Security Posture Summary ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, sr, p, t, pos) in &rows {
            let sym = match *pos {
                "good" => green("*"),
                "moderate" => yellow("~"),
                _ => red("!"),
            };
            println!(
                "  {} {} — secrets:{}, privileged:{}, tls:{}, posture:{}",
                sym, m, sr, p, t, pos
            );
        }
    }
    Ok(())
}

// ── FJ-1056: Machine Resource Freshness Index ──────────────────────────────

/// FJ-1056: `status --machine-resource-freshness-index`
pub(crate) fn cmd_status_machine_resource_freshness_index(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let now = now_epoch();
    let mut rows: Vec<(String, u64, String)> = Vec::new();
    for m in &machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            let score = freshness_score(&lock.generated_at, now);
            rows.push((m.clone(), score, lock.generated_at.clone()));
        }
    }
    if json {
        let entries: Vec<serde_json::Value> = rows
            .iter()
            .map(
                |(m, s, ts)| serde_json::json!({"machine":m,"freshness_index":s,"generated_at":ts}),
            )
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"freshness_index":{"machines":entries}})
            )
            .unwrap_or_default()
        );
    } else {
        println!("=== Machine Resource Freshness Index ===");
        if rows.is_empty() {
            println!("  No machine state found.");
        }
        for (m, score, ts) in &rows {
            let sym = if *score >= 60 {
                green("*")
            } else if *score >= 30 {
                yellow("~")
            } else {
                red("!")
            };
            println!(
                "  {} {} — freshness:{}/100, generated_at:{}",
                sym, m, score, ts
            );
        }
    }
    Ok(())
}

// ── FJ-1059: Fleet Resource Type Coverage ──────────────────────────────────

/// Collect resource types per machine.
pub(super) fn collect_type_coverage(
    state_dir: &Path,
    machines: &[String],
) -> BTreeMap<String, BTreeSet<String>> {
    let mut coverage: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            for rl in lock.resources.values() {
                coverage
                    .entry(rl.resource_type.to_string())
                    .or_default()
                    .insert(m.clone());
            }
        }
    }
    coverage
}

/// FJ-1059: `status --fleet-resource-type-coverage`
pub(crate) fn cmd_status_fleet_resource_type_coverage(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = filtered_machines(state_dir, machine);
    let coverage = collect_type_coverage(state_dir, &machines);
    if json {
        let entries: Vec<serde_json::Value> = coverage
            .iter()
            .map(|(rt, ms)| {
                let names: Vec<&str> = ms.iter().map(|s| s.as_str()).collect();
                serde_json::json!({"resource_type":rt,"machine_count":ms.len(),"machines":names})
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"resource_type_coverage":{"types":entries}})
            )
            .unwrap_or_default()
        );
    } else {
        println!("=== Fleet Resource Type Coverage ===");
        if coverage.is_empty() {
            println!("  No resources found.");
        }
        for (rt, ms) in &coverage {
            let names: Vec<&str> = ms.iter().map(|s| s.as_str()).collect();
            println!(
                "  {:>10} | {} machine(s): {}",
                rt,
                ms.len(),
                names.join(", ")
            );
        }
    }
    Ok(())
}
