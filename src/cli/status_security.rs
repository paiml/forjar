//! Phase 99 — Fleet Security & Resource Freshness: status commands.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::core::{state, types};
use super::helpers::*;

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Count security-relevant properties for a single machine's lock file.
/// Returns (secret_refs_count, privileged_count, tls_count).
fn security_counts(lock: &types::StateLock) -> (usize, usize, usize) {
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
fn classify_posture(secret_refs: usize, privileged: usize) -> &'static str {
    if secret_refs == 0 && privileged == 0 {
        "good"
    } else if secret_refs > 5 || privileged > 3 {
        "needs-attention"
    } else {
        "moderate"
    }
}

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

/// Compute freshness index (0-100) based on age of `generated_at`.
fn freshness_score(generated_at: &str, now: u64) -> u64 {
    match parse_rfc3339_to_epoch(generated_at) {
        Some(epoch) if now >= epoch => {
            let age = now - epoch;
            if age < 3600 { 100 }
            else if age < 86_400 { 80 }
            else if age < 604_800 { 60 }
            else if age < 2_592_000 { 30 }
            else { 0 }
        }
        _ => 0,
    }
}

/// Load machines respecting an optional filter.
fn filtered_machines(state_dir: &Path, machine: Option<&str>) -> Vec<String> {
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
        println!("{}", serde_json::to_string_pretty(
            &serde_json::json!({"fleet_security_posture":{"machines":entries}})
        ).unwrap_or_default());
    } else {
        println!("=== Fleet Security Posture Summary ===");
        if rows.is_empty() { println!("  No machine state found."); }
        for (m, sr, p, t, pos) in &rows {
            let sym = match *pos { "good" => green("*"), "moderate" => yellow("~"), _ => red("!") };
            println!("  {} {} — secrets:{}, privileged:{}, tls:{}, posture:{}", sym, m, sr, p, t, pos);
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
        let entries: Vec<serde_json::Value> = rows.iter().map(|(m, s, ts)| {
            serde_json::json!({"machine":m,"freshness_index":s,"generated_at":ts})
        }).collect();
        println!("{}", serde_json::to_string_pretty(
            &serde_json::json!({"freshness_index":{"machines":entries}})
        ).unwrap_or_default());
    } else {
        println!("=== Machine Resource Freshness Index ===");
        if rows.is_empty() { println!("  No machine state found."); }
        for (m, score, ts) in &rows {
            let sym = if *score >= 60 { green("*") } else if *score >= 30 { yellow("~") } else { red("!") };
            println!("  {} {} — freshness:{}/100, generated_at:{}", sym, m, score, ts);
        }
    }
    Ok(())
}

// ── FJ-1059: Fleet Resource Type Coverage ──────────────────────────────────

/// Collect resource types per machine.
fn collect_type_coverage(
    state_dir: &Path,
    machines: &[String],
) -> BTreeMap<String, BTreeSet<String>> {
    let mut coverage: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for m in machines {
        if let Ok(Some(lock)) = state::load_lock(state_dir, m) {
            for rl in lock.resources.values() {
                coverage.entry(rl.resource_type.to_string()).or_default().insert(m.clone());
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
        let entries: Vec<serde_json::Value> = coverage.iter().map(|(rt, ms)| {
            let names: Vec<&str> = ms.iter().map(|s| s.as_str()).collect();
            serde_json::json!({"resource_type":rt,"machine_count":ms.len(),"machines":names})
        }).collect();
        println!("{}", serde_json::to_string_pretty(
            &serde_json::json!({"resource_type_coverage":{"types":entries}})
        ).unwrap_or_default());
    } else {
        println!("=== Fleet Resource Type Coverage ===");
        if coverage.is_empty() { println!("  No resources found."); }
        for (rt, ms) in &coverage {
            let names: Vec<&str> = ms.iter().map(|s| s.as_str()).collect();
            println!("  {:>10} | {} machine(s): {}", rt, ms.len(), names.join(", "));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mk(machine: &str, res: Vec<(&str, types::ResourceType)>) -> types::StateLock {
        let mut m = indexmap::IndexMap::new();
        for (id, rt) in res {
            m.insert(id.to_string(), types::ResourceLock {
                resource_type: rt, status: types::ResourceStatus::Converged,
                applied_at: Some("2026-01-15T10:00:00Z".into()),
                duration_seconds: Some(1.0), hash: "abc".into(), details: HashMap::new(),
            });
        }
        types::StateLock {
            schema: "1".into(), machine: machine.into(), hostname: machine.into(),
            generated_at: "2026-01-15T10:00:00Z".into(), generator: "test".into(),
            blake3_version: "1.0".into(), resources: m,
        }
    }

    fn mk_secrets(machine: &str, n: u64) -> types::StateLock {
        let mut det = HashMap::new();
        det.insert("secret_refs".into(), serde_yaml_ng::Value::Number(n.into()));
        let mut m = indexmap::IndexMap::new();
        m.insert("f".into(), types::ResourceLock {
            resource_type: types::ResourceType::File, status: types::ResourceStatus::Converged,
            applied_at: Some("2026-01-15T10:00:00Z".into()), duration_seconds: Some(0.5),
            hash: "d".into(), details: det,
        });
        types::StateLock {
            schema: "1".into(), machine: machine.into(), hostname: machine.into(),
            generated_at: "2026-01-15T10:00:00Z".into(), generator: "test".into(),
            blake3_version: "1.0".into(), resources: m,
        }
    }

    fn wr(dir: &Path, lock: &types::StateLock) {
        let d = dir.join(&lock.machine);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("state.lock.yaml"), serde_yaml_ng::to_string(lock).unwrap()).unwrap();
    }

    // ── FJ-1053 ────────────────────────────────────────────────────────────

    #[test]
    fn test_security_posture_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_security_posture_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_security_posture_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("w1", vec![("svc", types::ResourceType::Service), ("tls-c", types::ResourceType::File)]));
        assert!(cmd_status_fleet_security_posture_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_security_posture_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk_secrets("db1", 3));
        assert!(cmd_status_fleet_security_posture_summary(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_security_counts_empty_and_mixed() {
        let empty = mk("e", vec![]);
        assert_eq!(security_counts(&empty), (0, 0, 0));
        let mixed = mk("m", vec![("nginx", types::ResourceType::Service), ("ssl-c", types::ResourceType::File)]);
        let (sr, p, t) = security_counts(&mixed);
        assert_eq!((sr, p, t), (0, 1, 1));
    }

    #[test]
    fn test_classify_posture_variants() {
        assert_eq!(classify_posture(0, 0), "good");
        assert_eq!(classify_posture(2, 1), "moderate");
        assert_eq!(classify_posture(6, 0), "needs-attention");
        assert_eq!(classify_posture(0, 4), "needs-attention");
    }

    // ── FJ-1056 ────────────────────────────────────────────────────────────

    #[test]
    fn test_freshness_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_freshness_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_freshness_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("s1", vec![("p", types::ResourceType::Package)]));
        assert!(cmd_status_machine_resource_freshness_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_freshness_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("s2", vec![("f", types::ResourceType::File)]));
        assert!(cmd_status_machine_resource_freshness_index(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_freshness_score_values() {
        let now = 1_700_000_000u64;
        assert!(freshness_score("2023-11-14T22:03:20Z", now) >= 80);
        assert_eq!(freshness_score("2020-01-01T00:00:00Z", 1_900_000_000), 0);
        assert_eq!(freshness_score("", now), 0);
        assert_eq!(freshness_score("garbage", now), 0);
    }

    #[test]
    fn test_freshness_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("a", vec![("p", types::ResourceType::Package)]));
        wr(d.path(), &mk("b", vec![("s", types::ResourceType::Service)]));
        assert!(cmd_status_machine_resource_freshness_index(d.path(), Some("a"), false).is_ok());
    }

    // ── FJ-1059 ────────────────────────────────────────────────────────────

    #[test]
    fn test_coverage_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_coverage(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_coverage_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("n1", vec![("p", types::ResourceType::Package), ("s", types::ResourceType::Service)]));
        assert!(cmd_status_fleet_resource_type_coverage(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_coverage_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("w1", vec![("p", types::ResourceType::Package)]));
        wr(d.path(), &mk("w2", vec![("p2", types::ResourceType::Package), ("f", types::ResourceType::File)]));
        assert!(cmd_status_fleet_resource_type_coverage(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_collect_coverage_multi() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("m1", vec![("p", types::ResourceType::Package)]));
        wr(d.path(), &mk("m2", vec![("p2", types::ResourceType::Package), ("s", types::ResourceType::Service)]));
        let c = collect_type_coverage(d.path(), &["m1".into(), "m2".into()]);
        assert_eq!(c["package"].len(), 2);
        assert_eq!(c["service"].len(), 1);
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_rfc3339() {
        let e = parse_rfc3339_to_epoch("2024-01-01T00:00:00Z");
        assert!(e.is_some());
        assert!(e.unwrap() > 1_700_000_000 && e.unwrap() < 1_800_000_000);
        assert!(parse_rfc3339_to_epoch("").is_none());
        assert!(parse_rfc3339_to_epoch("short").is_none());
    }
}
