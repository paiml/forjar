//! FJ-046/216: Minimal changeset, DAG ordering, and parallel waves.
//!
//! Popperian rejection criteria for:
//! - FJ-046: Minimal changeset computation + dependency propagation
//! - FJ-216: DAG topological ordering + parallel wave computation
//!
//! Usage: cargo test --test falsification_changeset_dag_staleness

use forjar::core::planner::minimal_changeset::{compute_minimal_changeset, verify_minimality};
use forjar::core::resolver::{build_execution_order, compute_parallel_waves};
use forjar::core::types::*;
use std::collections::BTreeMap;

fn cfg_with_deps(resources: &[(&str, &[&str])]) -> ForjarConfig {
    let mut cfg = ForjarConfig::default();
    for (id, deps) in resources {
        let mut r = Resource::default();
        r.depends_on = deps.iter().map(|s| s.to_string()).collect();
        cfg.resources.insert(id.to_string(), r);
    }
    cfg
}

// ============================================================================
// FJ-046: compute_minimal_changeset — no changes
// ============================================================================

#[test]
fn changeset_no_changes_needed() {
    let resources = vec![
        ("A".into(), "m1".into(), "h1".into()),
        ("B".into(), "m1".into(), "h2".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "h1".into());
    locks.insert("B@m1".into(), "h2".into());
    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(cs.changes_needed, 0);
    assert_eq!(cs.changes_skipped, 2);
    assert!(cs.is_provably_minimal);
    assert!(verify_minimality(&cs));
}

// ============================================================================
// FJ-046: compute_minimal_changeset — single change
// ============================================================================

#[test]
fn changeset_single_change() {
    let resources = vec![
        ("A".into(), "m1".into(), "new".into()),
        ("B".into(), "m1".into(), "h2".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "old".into());
    locks.insert("B@m1".into(), "h2".into());
    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(cs.changes_needed, 1);
    assert!(cs.candidates[0].necessary);
    assert!(!cs.candidates[1].necessary);
}

// ============================================================================
// FJ-046: compute_minimal_changeset — new resource
// ============================================================================

#[test]
fn changeset_new_resource() {
    let resources = vec![("NEW".into(), "m1".into(), "hash".into())];
    let cs = compute_minimal_changeset(&resources, &BTreeMap::new(), &[]);
    assert_eq!(cs.changes_needed, 1);
    assert!(cs.candidates[0].current_hash.is_none());
    assert!(cs.candidates[0].necessary);
}

// ============================================================================
// FJ-046: compute_minimal_changeset — dependency propagation
// ============================================================================

#[test]
fn changeset_dep_propagation() {
    let resources = vec![
        ("A".into(), "m1".into(), "new-a".into()),
        ("B".into(), "m1".into(), "h-b".into()),
        ("C".into(), "m1".into(), "h-c".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "old-a".into());
    locks.insert("B@m1".into(), "h-b".into());
    locks.insert("C@m1".into(), "h-c".into());
    // B depends on A, C depends on B → transitive
    let deps = vec![("B".into(), "A".into()), ("C".into(), "B".into())];
    let cs = compute_minimal_changeset(&resources, &locks, &deps);
    assert_eq!(cs.changes_needed, 3);
    assert!(cs.candidates.iter().all(|c| c.necessary));
}

#[test]
fn changeset_dep_no_propagation_when_clean() {
    let resources = vec![
        ("A".into(), "m1".into(), "h-a".into()),
        ("B".into(), "m1".into(), "h-b".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "h-a".into());
    locks.insert("B@m1".into(), "h-b".into());
    let deps = vec![("B".into(), "A".into())];
    let cs = compute_minimal_changeset(&resources, &locks, &deps);
    assert_eq!(cs.changes_needed, 0);
}

// ============================================================================
// FJ-046: verify_minimality
// ============================================================================

#[test]
fn changeset_verify_empty() {
    let cs = forjar::core::planner::minimal_changeset::MinimalChangeSet {
        total_resources: 0,
        changes_needed: 0,
        changes_skipped: 0,
        candidates: vec![],
        is_provably_minimal: true,
    };
    assert!(verify_minimality(&cs));
}

#[test]
fn changeset_serializes() {
    let resources = vec![("X".into(), "m1".into(), "hash".into())];
    let cs = compute_minimal_changeset(&resources, &BTreeMap::new(), &[]);
    let json = serde_json::to_string(&cs).unwrap();
    assert!(json.contains("is_provably_minimal"));
}

// ============================================================================
// FJ-216: build_execution_order — linear chain
// ============================================================================

#[test]
fn dag_linear_order() {
    let cfg = cfg_with_deps(&[("c", &["b"]), ("b", &["a"]), ("a", &[])]);
    let order = build_execution_order(&cfg).unwrap();
    let pos_a = order.iter().position(|x| x == "a").unwrap();
    let pos_b = order.iter().position(|x| x == "b").unwrap();
    let pos_c = order.iter().position(|x| x == "c").unwrap();
    assert!(pos_a < pos_b && pos_b < pos_c);
}

#[test]
fn dag_no_deps_alphabetical() {
    let cfg = cfg_with_deps(&[("z", &[]), ("a", &[]), ("m", &[])]);
    let order = build_execution_order(&cfg).unwrap();
    assert_eq!(order, vec!["a", "m", "z"]);
}

#[test]
fn dag_diamond() {
    let cfg = cfg_with_deps(&[("d", &["b", "c"]), ("b", &["a"]), ("c", &["a"]), ("a", &[])]);
    let order = build_execution_order(&cfg).unwrap();
    let pos_a = order.iter().position(|x| x == "a").unwrap();
    let pos_d = order.iter().position(|x| x == "d").unwrap();
    assert!(pos_a < pos_d);
    assert_eq!(order.len(), 4);
}

#[test]
fn dag_cycle_detected() {
    let cfg = cfg_with_deps(&[("a", &["b"]), ("b", &["a"])]);
    let result = build_execution_order(&cfg);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cycle"));
}

#[test]
fn dag_unknown_dep_error() {
    let cfg = cfg_with_deps(&[("a", &["missing"])]);
    let result = build_execution_order(&cfg);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown"));
}

// ============================================================================
// FJ-216: compute_parallel_waves
// ============================================================================

#[test]
fn waves_linear_chain() {
    let cfg = cfg_with_deps(&[("c", &["b"]), ("b", &["a"]), ("a", &[])]);
    let waves = compute_parallel_waves(&cfg).unwrap();
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0], vec!["a"]);
    assert_eq!(waves[1], vec!["b"]);
    assert_eq!(waves[2], vec!["c"]);
}

#[test]
fn waves_independent_single_wave() {
    let cfg = cfg_with_deps(&[("x", &[]), ("y", &[]), ("z", &[])]);
    let waves = compute_parallel_waves(&cfg).unwrap();
    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0].len(), 3);
}

#[test]
fn waves_diamond_three_waves() {
    let cfg = cfg_with_deps(&[("d", &["b", "c"]), ("b", &["a"]), ("c", &["a"]), ("a", &[])]);
    let waves = compute_parallel_waves(&cfg).unwrap();
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0], vec!["a"]);
    assert!(waves[1].contains(&"b".to_string()) && waves[1].contains(&"c".to_string()));
    assert_eq!(waves[2], vec!["d"]);
}

#[test]
fn waves_cycle_error() {
    let cfg = cfg_with_deps(&[("a", &["b"]), ("b", &["a"])]);
    assert!(compute_parallel_waves(&cfg).is_err());
}

#[test]
fn waves_empty_config() {
    let cfg = ForjarConfig::default();
    let waves = compute_parallel_waves(&cfg).unwrap();
    assert!(waves.is_empty());
}
