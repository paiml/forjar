//! PMAT-088 (FALSIFICATION E9): Persist L3-L5 test-coverage results.
//!
//! `forjar test coverage` historically reported only the static L0-L2 level
//! derived from "has check script" + "has behavior spec". Nothing ever
//! promoted a resource to L3 (convergence tested), L4 (mutation tested), or
//! L5 (preservation tested), even after those sandbox tests actually passed.
//!
//! This module closes that gap with an append-only `test-coverage.jsonl` log
//! under the state dir, mirroring the `events.jsonl` provenance pattern:
//!
//! * Convergence/mutation/preservation runners **append** a per-resource
//!   record `{resource_id, level, passed, timestamp, config_hash}` when a
//!   test for that resource completes.
//! * `forjar test coverage` **reads back** the log and promotes each resource
//!   to the highest level with a `passed` record **whose `config_hash` matches
//!   the resource's CURRENT desired-state hash**. A changed resource (hash
//!   mismatch) falls back to its static L0-L2 level — a stale high-water mark
//!   is a correctness bug, not a feature.

use crate::core::types::CoverageLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

/// PMAT-088: A single persisted test-coverage result for one resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCoverageRecord {
    /// Resource identifier this result applies to.
    pub resource_id: String,
    /// Coverage level proven by the test (L3/L4/L5).
    pub level: CoverageLevel,
    /// Whether the test passed (only passing records promote).
    pub passed: bool,
    /// ISO 8601 timestamp the record was written.
    pub timestamp: String,
    /// Desired-state hash of the resource at the time the test ran.
    ///
    /// A record only counts toward promotion if this matches the resource's
    /// CURRENT hash — otherwise the resource changed and the result is stale.
    pub config_hash: String,
}

impl TestCoverageRecord {
    /// Build a record stamped with the current time.
    pub fn new(
        resource_id: impl Into<String>,
        level: CoverageLevel,
        passed: bool,
        config_hash: impl Into<String>,
    ) -> Self {
        Self {
            resource_id: resource_id.into(),
            level,
            passed,
            timestamp: crate::tripwire::eventlog::now_iso8601(),
            config_hash: config_hash.into(),
        }
    }
}

/// PMAT-088: Path to the test-coverage log within the state directory.
pub fn coverage_log_path(state_dir: &Path) -> PathBuf {
    state_dir.join("test-coverage.jsonl")
}

/// PMAT-088: Append a single coverage record to the log (append-only JSONL).
pub fn append_record(state_dir: &Path, record: &TestCoverageRecord) -> Result<(), String> {
    let path = coverage_log_path(state_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create state dir: {e}"))?;
    }

    let json = serde_json::to_string(record).map_err(|e| format!("JSON serialize error: {e}"))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("cannot open coverage log {}: {}", path.display(), e))?;
    writeln!(file, "{json}").map_err(|e| format!("write error: {e}"))?;
    Ok(())
}

/// PMAT-088: Append a batch of records. Best-effort: stops at the first error.
pub fn append_records(state_dir: &Path, records: &[TestCoverageRecord]) -> Result<(), String> {
    for record in records {
        append_record(state_dir, record)?;
    }
    Ok(())
}

/// PMAT-088: Load every coverage record from the log.
///
/// Missing log → empty vec. Malformed lines are skipped (tolerant read: a
/// partially-written final line must not poison the whole history).
pub fn load_records(state_dir: &Path) -> Vec<TestCoverageRecord> {
    let path = coverage_log_path(state_dir);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<TestCoverageRecord>(line).ok())
        .collect()
}

/// PMAT-088: Highest level a record set proves for one resource, hash-gated.
///
/// Returns `None` if no passing record matches `current_hash`. Only passing
/// records whose `config_hash == current_hash` are considered; the maximum of
/// their levels is returned (L5 implies L4/L3/... by `CoverageLevel` ordering).
pub fn proven_level(
    records: &[TestCoverageRecord],
    resource_id: &str,
    current_hash: &str,
) -> Option<CoverageLevel> {
    records
        .iter()
        .filter(|r| r.resource_id == resource_id && r.passed && r.config_hash == current_hash)
        .map(|r| r.level)
        .max()
}

/// PMAT-088: Promote a static base level using the persisted, hash-gated log.
///
/// The result is the maximum of the static base level and the highest proven
/// (passing, hash-matching) level. A resource never regresses below its static
/// L0-L2 assessment, and a stale (hash-mismatched) high-water mark is ignored.
pub fn promote_level(
    base: CoverageLevel,
    records: &[TestCoverageRecord],
    resource_id: &str,
    current_hash: &str,
) -> CoverageLevel {
    match proven_level(records, resource_id, current_hash) {
        Some(proven) => base.max(proven),
        None => base,
    }
}

/// PMAT-088: The convergence outcome for one resource, used to derive a level.
///
/// Decouples `coverage_persist` from the runner's `ConvergenceResult` type and
/// keeps record-building to a single argument (clippy `too_many_arguments`).
#[derive(Debug, Clone, Copy)]
pub struct ConvergenceOutcome {
    /// First apply reached the desired state.
    pub converged: bool,
    /// Second apply was a no-op.
    pub idempotent: bool,
    /// State was preserved after co-located resources applied.
    pub preserved: bool,
    /// The test itself errored (environment/script failure).
    pub errored: bool,
    /// Pairwise preservation testing was actually enabled for this run.
    pub pairwise_enabled: bool,
}

impl ConvergenceOutcome {
    /// Whether this outcome earns the L3 (convergence) level.
    ///
    /// L3 requires convergence + idempotency with no execution error.
    /// Preservation is tracked separately (L5).
    pub fn proves_l3(&self) -> bool {
        self.converged && self.idempotent && !self.errored
    }

    /// Whether this outcome earns the L5 (preservation) level.
    ///
    /// Only meaningful when pairwise preservation testing was actually run; a
    /// single-resource run cannot prove preservation against co-located ones.
    pub fn proves_l5(&self) -> bool {
        self.pairwise_enabled && self.proves_l3() && self.preserved
    }

    /// The highest level this outcome proves, if any.
    pub fn proven_level(&self) -> Option<CoverageLevel> {
        if self.proves_l5() {
            Some(CoverageLevel::L5)
        } else if self.proves_l3() {
            Some(CoverageLevel::L3)
        } else {
            None
        }
    }
}

/// PMAT-088: Build a convergence coverage record for one resource.
///
/// Records L5 when pairwise preservation passed, else L3 when convergence +
/// idempotency passed. Returns `None` when the resource did not even reach L3
/// (no record is written for a non-result, keeping the log signal-only).
pub fn convergence_record(
    resource_id: &str,
    outcome: ConvergenceOutcome,
    config_hash: &str,
) -> Option<TestCoverageRecord> {
    outcome
        .proven_level()
        .map(|level| TestCoverageRecord::new(resource_id, level, true, config_hash))
}

/// PMAT-088: Build an L4 (mutation) coverage record for one resource.
///
/// A resource is L4 when at least one mutation was attempted and every
/// applicable mutation was detected (zero survivors, zero errors). A resource
/// with no mutations attempted, survivors, or errors earns no record.
pub fn mutation_record(
    resource_id: &str,
    attempted: usize,
    detected: usize,
    errored: usize,
    config_hash: &str,
) -> Option<TestCoverageRecord> {
    if attempted > 0 && errored == 0 && detected == attempted {
        Some(TestCoverageRecord::new(
            resource_id,
            CoverageLevel::L4,
            true,
            config_hash,
        ))
    } else {
        None
    }
}

/// PMAT-088: Index records by resource id for repeated promotion lookups.
///
/// Builds a `resource_id -> highest passing level per config_hash` map is
/// unnecessary here; promotion needs the current hash to gate, so callers pass
/// the loaded records directly. This helper instead groups the latest hash seen
/// per resource, used only for diagnostics/tests.
pub fn latest_hash_by_resource(records: &[TestCoverageRecord]) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for r in records {
        map.insert(r.resource_id.clone(), r.config_hash.clone());
    }
    map
}

#[cfg(test)]
mod tests {
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
    fn proven_level_picks_max_matching() {
        let recs = vec![
            rec("pkg", CoverageLevel::L3, true, "h1"),
            rec("pkg", CoverageLevel::L4, true, "h1"),
        ];
        assert_eq!(
            proven_level(&recs, "pkg", "h1"),
            Some(CoverageLevel::L4),
            "should pick the highest passing matching level"
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
    fn convergence_record_none_on_failure() {
        assert!(
            convergence_record("pkg", outcome(false, true, false, false, false), "h").is_none()
        );
        assert!(
            convergence_record("pkg", outcome(true, false, false, false, false), "h").is_none()
        );
        assert!(convergence_record("pkg", outcome(true, true, true, true, false), "h").is_none());
    }

    #[test]
    fn mutation_record_l4_when_all_detected() {
        let r = mutation_record("pkg", 5, 5, 0, "h");
        assert_eq!(r.unwrap().level, CoverageLevel::L4);
    }

    #[test]
    fn mutation_record_none_on_survivor_or_error_or_empty() {
        assert!(mutation_record("pkg", 5, 4, 0, "h").is_none()); // survivor
        assert!(mutation_record("pkg", 5, 5, 1, "h").is_none()); // errored
        assert!(mutation_record("pkg", 0, 0, 0, "h").is_none()); // nothing attempted
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
}
