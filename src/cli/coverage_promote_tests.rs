//! PMAT-088: End-to-end tests for coverage persistence + hash-gated promotion.
//!
//! Hermetic: parse a synthetic config, write synthetic records into a tempdir
//! state directory, then assert the promoted coverage level. No sandbox,
//! network, or docker — the persist/read/promote logic is exercised directly.

use super::*;
use crate::core::parser::parse_config;
use crate::core::store::convergence_runner::ConvergenceResult;
use crate::core::store::coverage_persist::{append_record, load_records, TestCoverageRecord};
use crate::core::types::{CoverageLevel, ForjarConfig, MutationReport, MutationResult};
use std::collections::HashSet;

const CONFIG_YAML: &str = r#"
version: "1.0"
name: cov-stack
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: file
    machine: m
    path: /etc/forjar/pkg.conf
    content: "original"
"#;

fn parse() -> ForjarConfig {
    parse_config(CONFIG_YAML).expect("config parses")
}

/// Current desired-state hash for resource `rid` in the parsed config.
fn current_hash(config: &ForjarConfig, rid: &str) -> String {
    let r = config.resources.get(rid).expect("resource exists");
    let resolved =
        crate::core::resolver::resolve_resource_templates(r, &config.params, &config.machines)
            .unwrap_or_else(|_| r.clone());
    crate::core::planner::hash_desired_state(&resolved)
}

/// Build a coverage entry for `pkg` against the given state dir.
fn entry_for(config: &ForjarConfig, state_dir: &std::path::Path) -> CoverageLevel {
    let records = load_records(state_dir);
    let spec: HashSet<String> = HashSet::new();
    let r = config.resources.get("pkg").unwrap();
    coverage_entry("pkg", r, config, &spec, &records).level
}

// ── (a) A fresh resource (no records) reports its static L0-L2 level ──

#[test]
fn falsify_a_fresh_resource_is_static_level() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse();
    // file resource with a path → has a check script → static L1.
    let level = entry_for(&config, dir.path());
    assert_eq!(level, CoverageLevel::L1, "fresh resource must be static L1");
    assert!(level < CoverageLevel::L3, "fresh resource is never L3+");
}

// ── (b) A passing L3 record (hash match) promotes to L3 ──

#[test]
fn falsify_b_passing_l3_record_promotes() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse();
    let hash = current_hash(&config, "pkg");
    append_record(
        dir.path(),
        &TestCoverageRecord::new("pkg", CoverageLevel::L3, true, hash),
    )
    .unwrap();
    assert_eq!(entry_for(&config, dir.path()), CoverageLevel::L3);
}

// ── (c) A config change (hash mismatch) demotes back to the static level ──

#[test]
fn falsify_c_config_change_demotes() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse();
    // Record stamped with a STALE hash (resource has since changed).
    append_record(
        dir.path(),
        &TestCoverageRecord::new("pkg", CoverageLevel::L5, true, "stale-hash"),
    )
    .unwrap();
    let level = entry_for(&config, dir.path());
    assert_eq!(
        level,
        CoverageLevel::L1,
        "stale high-water mark must not survive a config change"
    );
}

// ── (d) An L5 record implies L3/L4 (subsumes lower levels) ──

#[test]
fn falsify_d_l5_record_implies_l3_l4() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse();
    let hash = current_hash(&config, "pkg");
    append_record(
        dir.path(),
        &TestCoverageRecord::new("pkg", CoverageLevel::L5, true, hash),
    )
    .unwrap();
    let level = entry_for(&config, dir.path());
    assert_eq!(level, CoverageLevel::L5);
    assert!(level >= CoverageLevel::L3, "L5 implies L3");
    assert!(level >= CoverageLevel::L4, "L5 implies L4");
}

// ── persist_convergence_coverage writes L3 / L5 records ──

fn conv_result(rid: &str, converged: bool, idempotent: bool, preserved: bool) -> ConvergenceResult {
    ConvergenceResult {
        resource_id: rid.into(),
        resource_type: "file".into(),
        converged,
        idempotent,
        preserved,
        duration_ms: 1,
        error: None,
    }
}

#[test]
fn persist_convergence_writes_l3_on_pass() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse();
    let file = dir.path().join("forjar.yaml");
    // coverage_state_dir(file) == dir/state — write the record there.
    let results = vec![conv_result("pkg", true, true, true)];
    persist_convergence_coverage(&file, &config, &results, false);
    let records = load_records(&coverage_state_dir(&file));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].level, CoverageLevel::L3);
    assert_eq!(records[0].config_hash, current_hash(&config, "pkg"));
}

#[test]
fn persist_convergence_writes_l5_when_pairwise() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse();
    let file = dir.path().join("forjar.yaml");
    let results = vec![conv_result("pkg", true, true, true)];
    persist_convergence_coverage(&file, &config, &results, true);
    let records = load_records(&coverage_state_dir(&file));
    assert_eq!(records[0].level, CoverageLevel::L5);
}

#[test]
fn persist_convergence_writes_nothing_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse();
    let file = dir.path().join("forjar.yaml");
    let results = vec![conv_result("pkg", false, true, false)];
    persist_convergence_coverage(&file, &config, &results, false);
    assert!(load_records(&coverage_state_dir(&file)).is_empty());
}

// ── persist_mutation_coverage writes L4 records ──

fn mut_result(rid: &str, detected: bool) -> MutationResult {
    use crate::core::types::MutationOperator;
    MutationResult {
        resource_id: rid.into(),
        resource_type: "file".into(),
        operator: MutationOperator::DeleteFile,
        detected,
        reconverged: None,
        duration_ms: 1,
        error: None,
    }
}

#[test]
fn persist_mutation_writes_l4_when_all_detected() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse();
    let file = dir.path().join("forjar.yaml");
    let report =
        MutationReport::from_results(vec![mut_result("pkg", true), mut_result("pkg", true)]);
    persist_mutation_coverage(&file, &config, &report);
    let records = load_records(&coverage_state_dir(&file));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].level, CoverageLevel::L4);
}

#[test]
fn persist_mutation_writes_nothing_with_survivor() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse();
    let file = dir.path().join("forjar.yaml");
    let report =
        MutationReport::from_results(vec![mut_result("pkg", true), mut_result("pkg", false)]);
    persist_mutation_coverage(&file, &config, &report);
    assert!(load_records(&coverage_state_dir(&file)).is_empty());
}

// ── End-to-end: convergence then coverage read-back promotes ──

#[test]
fn end_to_end_convergence_then_coverage_promotes() {
    let dir = tempfile::tempdir().unwrap();
    let config = parse();
    let file = dir.path().join("forjar.yaml");
    let results = vec![conv_result("pkg", true, true, true)];
    persist_convergence_coverage(&file, &config, &results, false);

    // Now read back from the SAME state dir coverage_state_dir(file) resolves to.
    let records = load_records(&coverage_state_dir(&file));
    let spec: HashSet<String> = HashSet::new();
    let r = config.resources.get("pkg").unwrap();
    let level = coverage_entry("pkg", r, &config, &spec, &records).level;
    assert_eq!(level, CoverageLevel::L3);
}
