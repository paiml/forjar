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

/// PMAT-088 (#165): Level proven for one resource, hash-gated AND recency-aware.
///
/// Returns `None` unless the resource's MOST RECENT record at `current_hash`
/// passed. Only records whose `config_hash == current_hash` are considered (a
/// record for a different hash is a stale config and ignored). Among those, the
/// latest record by `timestamp` wins: a failing latest record is an explicit
/// DEMOTION (returns `None`) so a regression at an unchanged config can no
/// longer be masked by an earlier pass. A passing latest record promotes to its
/// `level` (L5 implies L4/L3/... by `CoverageLevel` ordering).
///
/// This closes the "stale high-water mark survives forever" bug: promotion is
/// no longer `max()` over ALL passing records — a later failure supersedes an
/// earlier pass for the same (resource, config_hash).
pub fn proven_level(
    records: &[TestCoverageRecord],
    resource_id: &str,
    current_hash: &str,
) -> Option<CoverageLevel> {
    let latest = records
        .iter()
        .filter(|r| r.resource_id == resource_id && r.config_hash == current_hash)
        .max_by(|a, b| a.timestamp.cmp(&b.timestamp))?;
    latest.passed.then_some(latest.level)
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

/// PMAT-088 (#165): Build a convergence coverage record for one resource.
///
/// Records L5 when pairwise preservation passed, else L3 when convergence +
/// idempotency passed (`passed=true`). When the resource did NOT reach L3, a
/// **failing L3 record** (`passed=false`) is written instead so a later
/// regression supersedes an earlier pass under recency-aware `proven_level`
/// (a stale high-water mark must not survive a regression at an unchanged
/// config_hash). Always returns `Some`.
pub fn convergence_record(
    resource_id: &str,
    outcome: ConvergenceOutcome,
    config_hash: &str,
) -> Option<TestCoverageRecord> {
    match outcome.proven_level() {
        Some(level) => Some(TestCoverageRecord::new(
            resource_id,
            level,
            true,
            config_hash,
        )),
        // A failing convergence run records an explicit L3 demotion so the
        // latest result wins over any earlier passing record.
        None => Some(TestCoverageRecord::new(
            resource_id,
            CoverageLevel::L3,
            false,
            config_hash,
        )),
    }
}

/// PMAT-088 (#165): Build an L4 (mutation) coverage record for one resource.
///
/// A resource is L4 (`passed=true`) when at least one mutation was attempted
/// and every applicable mutation was detected (zero survivors, zero errors).
/// When mutations were attempted but some survived/errored, a **failing L4
/// record** (`passed=false`) is written so a regression supersedes an earlier
/// L4 pass under recency-aware `proven_level`. When NOTHING was attempted
/// (`attempted == 0`) there is no signal at all, so no record is written.
pub fn mutation_record(
    resource_id: &str,
    attempted: usize,
    detected: usize,
    errored: usize,
    config_hash: &str,
) -> Option<TestCoverageRecord> {
    if attempted == 0 {
        return None;
    }
    let passed = errored == 0 && detected == attempted;
    Some(TestCoverageRecord::new(
        resource_id,
        CoverageLevel::L4,
        passed,
        config_hash,
    ))
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
#[path = "coverage_persist_tests.rs"]
mod tests;
