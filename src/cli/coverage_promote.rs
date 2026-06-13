//! PMAT-088 (FALSIFICATION E9): L3-L5 coverage persistence + promotion helpers.
//!
//! These helpers bridge the convergence/mutation test runners to the
//! `test-coverage.jsonl` log (see `core::store::coverage_persist`) and
//! drive the read-back + hash-gated promotion used by `forjar test coverage`.
//! Extracted from `check_test_runners.rs` to keep both files under the
//! 500-line limit.

use crate::core::store::convergence_runner::ConvergenceResult;
use crate::core::store::coverage_persist::{
    append_records, convergence_record, mutation_record, promote_level, ConvergenceOutcome,
    TestCoverageRecord,
};
use crate::core::types::{
    CoverageLevel, CoverageReport, ForjarConfig, MutationReport, Resource, ResourceCoverage,
};
use crate::core::{codegen, resolver};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// PMAT-088: Derive the coverage state dir for a config file.
///
/// Test commands operate on a config file and have no `--state-dir` flag, so
/// the persisted coverage log lives in a `state/` subdirectory next to the
/// config (matching the `--state-dir state` default used elsewhere). This
/// keeps the log co-located with the config and makes tests hermetic
/// (tempdir config → tempdir state).
pub(crate) fn coverage_state_dir(file: &Path) -> PathBuf {
    file.parent().unwrap_or(Path::new(".")).join("state")
}

/// PMAT-088: Map each resource id to its current desired-state hash.
///
/// This is the hash a persisted L3-L5 record must match to still count.
fn resource_hashes(config: &ForjarConfig) -> HashMap<String, String> {
    config
        .resources
        .iter()
        .map(|(rid, resource)| {
            let resolved =
                resolver::resolve_resource_templates(resource, &config.params, &config.machines)
                    .unwrap_or_else(|_| resource.clone());
            (
                rid.clone(),
                crate::core::planner::hash_desired_state(&resolved),
            )
        })
        .collect()
}

/// PMAT-088: Persist convergence/preservation results as coverage records.
///
/// Each passing result becomes an L3 record (or L5 when pairwise preservation
/// was enabled and passed), stamped with the resource's current desired-state
/// hash so a later config change invalidates the high-water mark.
pub(crate) fn persist_convergence_coverage(
    file: &Path,
    config: &ForjarConfig,
    results: &[ConvergenceResult],
    pairwise: bool,
) {
    let hashes = resource_hashes(config);
    let records: Vec<_> = results
        .iter()
        .filter_map(|r| {
            let hash = hashes.get(&r.resource_id)?;
            let outcome = ConvergenceOutcome {
                converged: r.converged,
                idempotent: r.idempotent,
                preserved: r.preserved,
                errored: r.error.is_some(),
                pairwise_enabled: pairwise,
            };
            convergence_record(&r.resource_id, outcome, hash)
        })
        .collect();
    if records.is_empty() {
        return;
    }
    let _ = append_records(&coverage_state_dir(file), &records);
}

/// Per-resource mutation tallies: (attempted, detected, errored).
type MutationTally = (usize, usize, usize);

/// PMAT-088: Aggregate mutation results into per-resource tallies.
fn tally_mutations(report: &MutationReport) -> HashMap<String, MutationTally> {
    let mut tallies: HashMap<String, MutationTally> = HashMap::new();
    for r in &report.results {
        let entry = tallies.entry(r.resource_id.clone()).or_default();
        entry.0 += 1;
        if r.error.is_some() {
            entry.2 += 1;
        } else if r.detected {
            entry.1 += 1;
        }
    }
    tallies
}

/// PMAT-088: Persist mutation results as L4 coverage records.
///
/// A resource earns an L4 record when every applicable mutation was detected
/// (no survivors, no errors), stamped with its current desired-state hash.
pub(crate) fn persist_mutation_coverage(
    file: &Path,
    config: &ForjarConfig,
    report: &MutationReport,
) {
    let hashes = resource_hashes(config);
    let tallies = tally_mutations(report);
    let records: Vec<_> = tallies
        .iter()
        .filter_map(|(rid, (attempted, detected, errored))| {
            let hash = hashes.get(rid)?;
            mutation_record(rid, *attempted, *detected, *errored, hash)
        })
        .collect();
    if records.is_empty() {
        return;
    }
    let _ = append_records(&coverage_state_dir(file), &records);
}

/// Compute the static L0-L2 base level for a resource.
fn static_coverage_level(resolved: &Resource, has_spec: bool) -> CoverageLevel {
    let has_check = codegen::check_script(resolved).is_ok();
    match (has_spec, has_check) {
        (true, true) => CoverageLevel::L2,
        (_, true) => CoverageLevel::L1,
        _ => CoverageLevel::L0,
    }
}

/// PMAT-088: Build a promoted coverage entry for one resource.
///
/// Starts from the static L0-L2 base, then promotes to the highest persisted,
/// passing, hash-matching L3-L5 record (stale records are ignored).
pub(crate) fn coverage_entry(
    rid: &str,
    resource: &Resource,
    config: &ForjarConfig,
    spec_resources: &HashSet<String>,
    records: &[TestCoverageRecord],
) -> ResourceCoverage {
    let resolved = resolver::resolve_resource_templates(resource, &config.params, &config.machines)
        .unwrap_or_else(|_| resource.clone());
    let base = static_coverage_level(&resolved, spec_resources.contains(rid));
    let current_hash = crate::core::planner::hash_desired_state(&resolved);
    let level = promote_level(base, records, rid, &current_hash);
    ResourceCoverage {
        resource_id: rid.to_string(),
        level,
        resource_type: format!("{:?}", resource.resource_type).to_lowercase(),
    }
}

/// PMAT-088: Print the coverage summary including L3-L5 buckets.
pub(crate) fn print_coverage_summary(report: &CoverageReport) {
    let total = report.resources.len();
    let h = &report.histogram;
    let at_l3_plus = h[3] + h[4] + h[5];
    let at_l4_plus = h[4] + h[5];
    println!(
        "\nMin: {}, Avg: {:.1}, L0: {}, L1: {}, L2: {}, L3: {}, L4: {}, L5: {}",
        report.min_level.label(),
        report.avg_level,
        h[0],
        h[1],
        h[2],
        h[3],
        h[4],
        h[5],
    );
    println!("Coverage: {at_l3_plus}/{total} at L3+, {at_l4_plus}/{total} at L4+");
}

#[cfg(test)]
#[path = "coverage_promote_tests.rs"]
mod tests;
