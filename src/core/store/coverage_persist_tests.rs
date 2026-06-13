//! Tests for `coverage_persist` (PMAT-088 / #165 recency-aware demotion).
//!
//! Extracted from `coverage_persist.rs` to keep that file under the
//! 500-line limit; included via `#[path]` so the tests still live in the
//! module's namespace (`use super::*`).

use super::*;

fn rec(id: &str, level: CoverageLevel, passed: bool, hash: &str) -> TestCoverageRecord {
    TestCoverageRecord {
        resource_id: id.into(),
        level,
        passed,
        timestamp: "2026-06-13T00:00:00Z".into(),
        config_hash: hash.into(),
    }
}

/// Like `rec` but with an explicit timestamp, for recency-ordering tests.
fn rec_at(
    id: &str,
    level: CoverageLevel,
    passed: bool,
    hash: &str,
    ts: &str,
) -> TestCoverageRecord {
    TestCoverageRecord {
        resource_id: id.into(),
        level,
        passed,
        timestamp: ts.into(),
        config_hash: hash.into(),
    }
}

#[test]
fn append_then_load_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let r = rec("pkg", CoverageLevel::L3, true, "abc");
    append_record(dir.path(), &r).unwrap();
    let loaded = load_records(dir.path());
    assert_eq!(loaded, vec![r]);
}

#[test]
fn load_missing_log_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load_records(dir.path()).is_empty());
}

#[test]
fn load_skips_malformed_lines() {
    let dir = tempfile::tempdir().unwrap();
    let r = rec("pkg", CoverageLevel::L4, true, "h1");
    append_record(dir.path(), &r).unwrap();
    // Simulate a torn final write by appending garbage.
    let path = coverage_log_path(dir.path());
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    writeln!(f, "{{not valid json").unwrap();
    let loaded = load_records(dir.path());
    assert_eq!(loaded, vec![r]);
}

#[test]
fn append_records_batch() {
    let dir = tempfile::tempdir().unwrap();
    let recs = vec![
        rec("a", CoverageLevel::L3, true, "h"),
        rec("b", CoverageLevel::L4, true, "h"),
    ];
    append_records(dir.path(), &recs).unwrap();
    assert_eq!(load_records(dir.path()).len(), 2);
}

#[test]
fn proven_level_uses_latest_matching() {
    // The latest passing record at the current hash wins (recency-aware).
    let recs = vec![
        rec_at("pkg", CoverageLevel::L3, true, "h1", "2026-06-13T00:00:00Z"),
        rec_at("pkg", CoverageLevel::L4, true, "h1", "2026-06-13T00:00:01Z"),
    ];
    assert_eq!(
        proven_level(&recs, "pkg", "h1"),
        Some(CoverageLevel::L4),
        "should use the most recent passing matching record"
    );
}

#[test]
fn proven_level_ignores_hash_mismatch() {
    let recs = vec![rec("pkg", CoverageLevel::L5, true, "old-hash")];
    assert_eq!(proven_level(&recs, "pkg", "new-hash"), None);
}

#[test]
fn proven_level_ignores_failures() {
    let recs = vec![rec("pkg", CoverageLevel::L4, false, "h1")];
    assert_eq!(proven_level(&recs, "pkg", "h1"), None);
}

// ── #165: recency-aware demotion (regression at an unchanged hash) ──

#[test]
fn proven_level_demotes_when_latest_record_fails() {
    // A passing L3 record then a LATER failing record at the SAME hash:
    // recency wins, so the resource is demoted (no longer L3). This is the
    // exact "stale high-water mark survives forever" bug from #165.
    let recs = vec![
        rec_at("pkg", CoverageLevel::L3, true, "h1", "2026-06-13T00:00:00Z"),
        rec_at(
            "pkg",
            CoverageLevel::L3,
            false,
            "h1",
            "2026-06-13T00:00:05Z",
        ),
    ];
    assert_eq!(
        proven_level(&recs, "pkg", "h1"),
        None,
        "a later failing record at the same hash must demote the resource"
    );
    assert_eq!(
        promote_level(CoverageLevel::L2, &recs, "pkg", "h1"),
        CoverageLevel::L2,
        "promotion must fall back to the static base after a regression"
    );
}

#[test]
fn proven_level_repromotes_when_latest_record_passes() {
    // Fail then a LATER pass at the same hash re-promotes (recency wins).
    let recs = vec![
        rec_at(
            "pkg",
            CoverageLevel::L3,
            false,
            "h1",
            "2026-06-13T00:00:00Z",
        ),
        rec_at("pkg", CoverageLevel::L3, true, "h1", "2026-06-13T00:00:09Z"),
    ];
    assert_eq!(proven_level(&recs, "pkg", "h1"), Some(CoverageLevel::L3));
}

#[test]
fn proven_level_recency_is_hash_scoped() {
    // A LATER failing record at a STALE hash must not demote the current
    // hash's passing record (the hash gate isolates per-config history).
    let recs = vec![
        rec_at("pkg", CoverageLevel::L3, true, "h1", "2026-06-13T00:00:00Z"),
        rec_at(
            "pkg",
            CoverageLevel::L3,
            false,
            "stale",
            "2026-06-13T00:00:09Z",
        ),
    ];
    assert_eq!(
        proven_level(&recs, "pkg", "h1"),
        Some(CoverageLevel::L3),
        "a failing record at a different hash must be ignored"
    );
}

#[test]
fn proven_level_ignores_other_resources() {
    let recs = vec![rec("other", CoverageLevel::L5, true, "h1")];
    assert_eq!(proven_level(&recs, "pkg", "h1"), None);
}

// ── Falsification-style tests (PMAT-088 / E9) ──

#[test]
fn falsify_a_fresh_resource_stays_at_base() {
    // (a) No records → a fresh L1 resource is NOT promoted.
    let recs: Vec<TestCoverageRecord> = vec![];
    assert_eq!(
        promote_level(CoverageLevel::L1, &recs, "pkg", "h1"),
        CoverageLevel::L1
    );
    assert_eq!(
        promote_level(CoverageLevel::L0, &recs, "pkg", "h1"),
        CoverageLevel::L0
    );
}

#[test]
fn falsify_b_passing_l3_promotes() {
    // (b) A passing L3 record (hash match) promotes L2 → L3.
    let recs = vec![rec("pkg", CoverageLevel::L3, true, "h1")];
    assert_eq!(
        promote_level(CoverageLevel::L2, &recs, "pkg", "h1"),
        CoverageLevel::L3
    );
}

#[test]
fn falsify_c_config_change_demotes() {
    // (c) Config change (hash mismatch) demotes back to static base.
    let recs = vec![rec("pkg", CoverageLevel::L3, true, "old-hash")];
    assert_eq!(
        promote_level(CoverageLevel::L2, &recs, "pkg", "new-hash"),
        CoverageLevel::L2,
        "stale high-water mark must not survive a config change"
    );
}

#[test]
fn falsify_d_l5_implies_l3_and_l4() {
    // (d) An L5 record promotes a resource to L5 (which subsumes L3/L4).
    let recs = vec![rec("pkg", CoverageLevel::L5, true, "h1")];
    let level = promote_level(CoverageLevel::L1, &recs, "pkg", "h1");
    assert_eq!(level, CoverageLevel::L5);
    assert!(level >= CoverageLevel::L3, "L5 implies L3");
    assert!(level >= CoverageLevel::L4, "L5 implies L4");
}

#[test]
fn promote_never_regresses_below_base() {
    // A lower proven level than the static base does not regress the base.
    let recs = vec![rec("pkg", CoverageLevel::L3, true, "h1")];
    assert_eq!(
        promote_level(CoverageLevel::L4, &recs, "pkg", "h1"),
        CoverageLevel::L4,
        "base L4 must not regress to a proven L3"
    );
}

fn outcome(
    converged: bool,
    idempotent: bool,
    preserved: bool,
    errored: bool,
    pairwise_enabled: bool,
) -> ConvergenceOutcome {
    ConvergenceOutcome {
        converged,
        idempotent,
        preserved,
        errored,
        pairwise_enabled,
    }
}

#[test]
fn convergence_record_l3_on_pass() {
    let r = convergence_record("pkg", outcome(true, true, true, false, false), "h");
    assert_eq!(r.unwrap().level, CoverageLevel::L3);
}

#[test]
fn convergence_record_l5_when_pairwise_preserved() {
    let r = convergence_record("pkg", outcome(true, true, true, false, true), "h");
    assert_eq!(r.unwrap().level, CoverageLevel::L5);
}

#[test]
fn convergence_record_l3_when_pairwise_off_even_if_preserved() {
    // preserved=true but pairwise not enabled → only L3, not L5.
    let r = convergence_record("pkg", outcome(true, true, true, false, false), "h");
    assert_eq!(r.unwrap().level, CoverageLevel::L3);
}

#[test]
fn convergence_record_writes_failing_l3_on_failure() {
    // #165: a failing convergence run now writes an explicit failing L3
    // record (passed=false) so a later failure supersedes an earlier pass.
    for o in [
        outcome(false, true, false, false, false),
        outcome(true, false, false, false, false),
        outcome(true, true, true, true, false),
    ] {
        let r = convergence_record("pkg", o, "h").expect("failure must record a demotion");
        assert_eq!(r.level, CoverageLevel::L3);
        assert!(
            !r.passed,
            "a failing convergence run must record passed=false"
        );
    }
}

#[test]
fn mutation_record_l4_when_all_detected() {
    let r = mutation_record("pkg", 5, 5, 0, "h");
    let r = r.unwrap();
    assert_eq!(r.level, CoverageLevel::L4);
    assert!(r.passed);
}

#[test]
fn mutation_record_writes_failing_l4_on_survivor_or_error() {
    // #165: survivors/errors now write a failing L4 record so the latest
    // result supersedes an earlier L4 pass; nothing-attempted writes none.
    let survivor = mutation_record("pkg", 5, 4, 0, "h").expect("survivor must record");
    assert_eq!(survivor.level, CoverageLevel::L4);
    assert!(!survivor.passed);
    let errored = mutation_record("pkg", 5, 5, 1, "h").expect("error must record");
    assert!(!errored.passed);
    assert!(
        mutation_record("pkg", 0, 0, 0, "h").is_none(),
        "no mutations attempted means no signal — write nothing"
    );
}

#[test]
fn latest_hash_by_resource_tracks_last_seen() {
    let recs = vec![
        rec("pkg", CoverageLevel::L3, true, "h1"),
        rec("pkg", CoverageLevel::L3, true, "h2"),
    ];
    let map = latest_hash_by_resource(&recs);
    assert_eq!(map.get("pkg"), Some(&"h2".to_string()));
}
