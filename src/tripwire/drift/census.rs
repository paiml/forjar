//! forjar#380: THE DENOMINATOR — what a drift run looked at, and what it did not.
//!
//! `forjar drift` printed `No drift detected.` and said nothing about the
//! population that produced that verdict. Measured on paiml/infra's gx10:
//! the config declares 62 resources, the lock holds 30, and the guard whose
//! `completion_check` asserts the runner's org scope and five labels — every
//! one of those assertions false on the box — is in neither number the
//! operator sees. The command was right that it had found no drift. It was
//! silent about having barely looked.
//!
//! # Why the counters live in the detectors and not in a second pass
//!
//! A census computed by re-walking the lock with its own copy of the skip
//! rules is two hand-maintained lists with nothing tying them together — the
//! shape of bashrs#266, where a stdlib whitelist and an emitter dispatch
//! drifted apart and the tests asserted the wrong half. So every `continue`
//! in the detectors records WHY, and the counts are a by-product of the
//! decisions actually taken. A skip that forgets to say so is then a missing
//! resource in the total, which the `in_scope` figure makes visible.

use crate::core::types::ResourceType;
use std::collections::BTreeMap;

/// Why a resource that was in scope did not get inspected.
///
/// A closed set on purpose: the summary renders these and nothing else, so a
/// new skip path cannot invent an unreviewed category on its way to the
/// operator's terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SkipReason {
    /// Lock status is neither `converged` nor `drifted` — there is no baseline
    /// an apply ever established, so there is nothing to have drifted from.
    NotConverged,
    /// `lifecycle.ignore_drift: ["*"]` — the operator asked for this one.
    IgnoreDrift,
    /// Converged, but the lock records no observed state: nothing ever asked
    /// the target. `--refresh` seeding writes exactly this entry.
    NoObservedState,
    /// In the lock, absent from the config — forjar cannot regenerate the
    /// query that would observe it.
    NotInConfig,
    /// Declared in the config for this machine, absent from the lock. Never
    /// applied here (or applied through a different `--state-dir`).
    NotInLock,
    /// A file lock entry carrying neither `path` nor `content_hash`.
    NoLockedHash,
    /// No config was loaded, so only file hashes could be compared.
    NoConfigLoaded,
    /// `--no-task-checks`: the operator declined to execute completion checks.
    TaskChecksDisabled,
    /// forjar#385: there is no lock AT ALL — the state dir is absent, so
    /// nothing was ever applied through it. Distinct from `NotInLock`, which
    /// is one resource missing from a lock that exists: this is the whole
    /// baseline missing, which is the routine state of every CI checkout of a
    /// repo that gitignores `state/`. Only assertions can be measured here.
    NoLock,
}

impl SkipReason {
    /// Operator-facing wording. Phrased as what did NOT happen, because a
    /// skipped resource is an absence of evidence and must not read as a pass.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotConverged => "not converged in the lock",
            Self::IgnoreDrift => "lifecycle.ignore_drift",
            Self::NoObservedState => "no observed state in the lock",
            Self::NotInConfig => "in the lock, not in the config",
            Self::NotInLock => "declared here, absent from the lock",
            Self::NoLockedHash => "no hash recorded in the lock",
            Self::NoConfigLoaded => "no config loaded (file hashes only)",
            Self::TaskChecksDisabled => "--no-task-checks",
            Self::NoLock => "no lock (never applied from here)",
        }
    }
}

/// What one detector decided about one resource.
#[derive(Debug, Clone)]
struct Entry {
    resource_type: ResourceType,
    /// `None` once any detector has inspected it. INSPECTED WINS: a file that
    /// the content-hash path skipped and the state-query path examined was
    /// looked at, and reporting it as skipped would understate the coverage
    /// exactly as badly as the silence this module replaces overstates it.
    skipped: Option<SkipReason>,
}

/// Per-resource record of a drift run's coverage over one machine.
#[derive(Debug, Clone, Default)]
pub struct DriftCensus {
    entries: BTreeMap<String, Entry>,
}

impl DriftCensus {
    /// An empty census.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a detector queried the target about this resource.
    pub(super) fn inspected(&mut self, id: &str, resource_type: &ResourceType) {
        self.entries.insert(
            id.to_string(),
            Entry {
                resource_type: resource_type.clone(),
                skipped: None,
            },
        );
    }

    /// Record that a detector declined to query the target, and why.
    ///
    /// Never downgrades an inspected entry, and keeps the FIRST reason when a
    /// second detector skips the same resource again: detector order is fixed
    /// (files, tasks, state queries, images), so the reported reason is
    /// deterministic rather than a race between two equally true answers.
    pub(super) fn skipped(&mut self, id: &str, resource_type: &ResourceType, reason: SkipReason) {
        self.entries.entry(id.to_string()).or_insert_with(|| Entry {
            resource_type: resource_type.clone(),
            skipped: Some(reason),
        });
    }

    /// Resources this run had an opinion about — inspected plus skipped.
    pub fn in_scope(&self) -> usize {
        self.entries.len()
    }

    /// Resources a detector actually asked the target about.
    pub fn inspected_total(&self) -> usize {
        self.entries
            .values()
            .filter(|e| e.skipped.is_none())
            .count()
    }

    /// Resources in scope that nothing asked the target about.
    pub fn skipped_total(&self) -> usize {
        self.in_scope() - self.inspected_total()
    }

    /// Inspected counts keyed by resource type, e.g. `{"file": 6, "task": 2}`.
    pub fn inspected_by_type(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for entry in self.entries.values().filter(|e| e.skipped.is_none()) {
            *counts
                .entry(entry.resource_type.to_string())
                .or_insert(0usize) += 1;
        }
        counts
    }

    /// Skipped counts keyed by reason.
    pub fn skipped_by_reason(&self) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        for reason in self.entries.values().filter_map(|e| e.skipped) {
            *counts.entry(reason.as_str()).or_insert(0usize) += 1;
        }
        counts
    }

    /// The two lines every drift run prints, drift or no drift.
    ///
    /// The second line is omitted when nothing was skipped — that is the only
    /// case in which `No drift detected.` on its own is the whole truth.
    pub fn summary_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "inspected {} of {} resource(s) in scope: {}",
            self.inspected_total(),
            self.in_scope(),
            render_counts(self.inspected_by_type().into_iter())
        )];
        if self.skipped_total() > 0 {
            lines.push(format!(
                "skipped {}: {}",
                self.skipped_total(),
                render_counts(self.skipped_by_reason().into_iter())
            ));
        }
        lines
    }

    /// The same numbers `--json` consumers need. A machine reading the JSON
    /// report was as blind as a human reading the text one, so the denominator
    /// ships in both or in neither.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "in_scope": self.in_scope(),
            "inspected": self.inspected_total(),
            "skipped": self.skipped_total(),
            "inspected_by_type": self.inspected_by_type(),
            "skipped_by_reason": self.skipped_by_reason(),
        })
    }
}

/// `a 2, b 1` — or `none` for an empty map, which must not render as blank.
fn render_counts<K: std::fmt::Display>(counts: impl Iterator<Item = (K, usize)>) -> String {
    let rendered: Vec<String> = counts.map(|(k, v)| format!("{k} {v}")).collect();
    if rendered.is_empty() {
        "none".to_string()
    } else {
        rendered.join(", ")
    }
}
