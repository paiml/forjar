//! FJ-046/216: Minimal changeset computation and DAG execution order
//! falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-046: Minimal changeset
//!   - compute_minimal_changeset: hash comparison, new resources, skip unchanged
//!   - verify_minimality: empty, non-empty, all-necessary
//!   - Dependency propagation: transitive marking
//! - FJ-216: DAG execution order
//!   - build_execution_order: topological sort, cycle detection, determinism
//!   - compute_parallel_waves: wave grouping, concurrency, cycle detection
//!
//! Usage: cargo test --test falsification_dag_changeset

use forjar::core::planner::minimal_changeset::{
    compute_minimal_changeset, verify_minimality, MinimalChangeSet,
};
use forjar::core::resolver::{build_execution_order, compute_parallel_waves};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::BTreeMap;

// ============================================================================
// FJ-046: compute_minimal_changeset — hash comparison
// ============================================================================

#[test]
fn changeset_all_unchanged_skips_all() {
    let resources = vec![
        ("nginx".into(), "web-01".into(), "blake3:aaa".into()),
        ("pg".into(), "db-01".into(), "blake3:bbb".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("nginx@web-01".into(), "blake3:aaa".into());
    locks.insert("pg@db-01".into(), "blake3:bbb".into());

    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(cs.changes_needed, 0);
    assert_eq!(cs.changes_skipped, 2);
    assert_eq!(cs.total_resources, 2);
    assert!(cs.is_provably_minimal);
}

#[test]
fn changeset_single_hash_diff_marks_necessary() {
    let resources = vec![
        ("nginx".into(), "web-01".into(), "blake3:new".into()),
        ("pg".into(), "db-01".into(), "blake3:bbb".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("nginx@web-01".into(), "blake3:old".into());
    locks.insert("pg@db-01".into(), "blake3:bbb".into());

    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(cs.changes_needed, 1);
    assert!(cs.candidates[0].necessary);
    assert!(!cs.candidates[1].necessary);
}

#[test]
fn changeset_all_changed() {
    let resources = vec![
        ("a".into(), "m1".into(), "h1-new".into()),
        ("b".into(), "m1".into(), "h2-new".into()),
        ("c".into(), "m1".into(), "h3-new".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("a@m1".into(), "h1-old".into());
    locks.insert("b@m1".into(), "h2-old".into());
    locks.insert("c@m1".into(), "h3-old".into());

    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(cs.changes_needed, 3);
    assert_eq!(cs.changes_skipped, 0);
    assert!(cs.candidates.iter().all(|c| c.necessary));
}

// ============================================================================
// FJ-046: new resources (no lock entry)
// ============================================================================

#[test]
fn changeset_new_resource_always_necessary() {
    let resources = vec![("brand-new".into(), "m1".into(), "blake3:xyz".into())];
    let locks = BTreeMap::new();

    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(cs.changes_needed, 1);
    assert!(cs.candidates[0].necessary);
    assert!(cs.candidates[0].current_hash.is_none());
}

#[test]
fn changeset_mix_new_and_unchanged() {
    let resources = vec![
        ("existing".into(), "m1".into(), "blake3:same".into()),
        ("new-svc".into(), "m1".into(), "blake3:fresh".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("existing@m1".into(), "blake3:same".into());

    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(cs.changes_needed, 1);
    assert!(!cs.candidates[0].necessary); // existing: unchanged
    assert!(cs.candidates[1].necessary); // new-svc: no lock entry
}

// ============================================================================
// FJ-046: dependency propagation
// ============================================================================

#[test]
fn changeset_dependency_propagation_marks_dependent() {
    let resources = vec![
        ("config".into(), "m1".into(), "blake3:new-cfg".into()),
        ("service".into(), "m1".into(), "blake3:same-svc".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("config@m1".into(), "blake3:old-cfg".into());
    locks.insert("service@m1".into(), "blake3:same-svc".into());
    // service depends on config
    let deps = vec![("service".into(), "config".into())];

    let cs = compute_minimal_changeset(&resources, &locks, &deps);
    assert_eq!(cs.changes_needed, 2);
    // config changed → service marked necessary via propagation
    assert!(cs.candidates[0].necessary);
    assert!(cs.candidates[1].necessary);
}

#[test]
fn changeset_transitive_dependency_chain() {
    // A → B → C: if A changes, B and C should be marked
    let resources = vec![
        ("A".into(), "m1".into(), "new-a".into()),
        ("B".into(), "m1".into(), "same-b".into()),
        ("C".into(), "m1".into(), "same-c".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "old-a".into());
    locks.insert("B@m1".into(), "same-b".into());
    locks.insert("C@m1".into(), "same-c".into());
    let deps = vec![
        ("B".into(), "A".into()), // B depends on A
        ("C".into(), "B".into()), // C depends on B
    ];

    let cs = compute_minimal_changeset(&resources, &locks, &deps);
    assert_eq!(cs.changes_needed, 3, "transitive chain must propagate");
}

#[test]
fn changeset_independent_no_propagation() {
    let resources = vec![
        ("X".into(), "m1".into(), "new-x".into()),
        ("Y".into(), "m1".into(), "same-y".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("X@m1".into(), "old-x".into());
    locks.insert("Y@m1".into(), "same-y".into());
    // No dependency between X and Y
    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(cs.changes_needed, 1);
    assert!(cs.candidates[0].necessary);
    assert!(!cs.candidates[1].necessary);
}

// ============================================================================
// FJ-046: verify_minimality
// ============================================================================

#[test]
fn verify_minimality_empty_is_minimal() {
    let cs = MinimalChangeSet {
        total_resources: 0,
        changes_needed: 0,
        changes_skipped: 0,
        candidates: vec![],
        is_provably_minimal: true,
    };
    assert!(verify_minimality(&cs));
}

#[test]
fn verify_minimality_nonempty_is_minimal() {
    let resources = vec![("A".into(), "m1".into(), "new".into())];
    let locks = BTreeMap::new();
    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    assert!(verify_minimality(&cs));
}

#[test]
fn verify_minimality_all_skipped() {
    let resources = vec![("A".into(), "m1".into(), "same".into())];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "same".into());
    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    assert!(verify_minimality(&cs));
}

// ============================================================================
// FJ-046: serialization roundtrip
// ============================================================================

#[test]
fn changeset_serializes_to_json() {
    let resources = vec![("svc".into(), "m1".into(), "h1".into())];
    let locks = BTreeMap::new();
    let cs = compute_minimal_changeset(&resources, &locks, &[]);
    let json = serde_json::to_string(&cs).unwrap();
    assert!(json.contains("\"is_provably_minimal\":true"));
    assert!(json.contains("\"resource\":\"svc\""));
}

// ============================================================================
// FJ-216: build_execution_order — linear chain
// ============================================================================

fn config_with_deps(names: &[&str], deps: &[(&str, &[&str])]) -> ForjarConfig {
    let mut resources = IndexMap::new();
    for name in names {
        resources.insert(
            name.to_string(),
            Resource {
                resource_type: ResourceType::File,
                ..Default::default()
            },
        );
    }
    for (name, dep_list) in deps {
        if let Some(r) = resources.get_mut(*name) {
            r.depends_on = dep_list.iter().map(|s| s.to_string()).collect();
        }
    }
    ForjarConfig {
        name: "test".into(),
        resources,
        ..Default::default()
    }
}

#[test]
fn dag_no_deps_alphabetical() {
    let config = config_with_deps(&["charlie", "alice", "bob"], &[]);
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order, vec!["alice", "bob", "charlie"]);
}

#[test]
fn dag_linear_chain() {
    // A → B → C (C depends on B, B depends on A)
    let config = config_with_deps(&["A", "B", "C"], &[("B", &["A"]), ("C", &["B"])]);
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order, vec!["A", "B", "C"]);
}

#[test]
fn dag_diamond() {
    // A → B, A → C, B → D, C → D
    let config = config_with_deps(
        &["A", "B", "C", "D"],
        &[("B", &["A"]), ("C", &["A"]), ("D", &["B", "C"])],
    );
    let order = build_execution_order(&config).unwrap();
    // A must come first, D must come last, B and C in between
    assert_eq!(order[0], "A");
    assert_eq!(order[3], "D");
    // B and C must both precede D
    let pos_b = order.iter().position(|x| x == "B").unwrap();
    let pos_c = order.iter().position(|x| x == "C").unwrap();
    assert!(pos_b < 3);
    assert!(pos_c < 3);
}

#[test]
fn dag_cycle_detected() {
    let config = config_with_deps(&["A", "B"], &[("A", &["B"]), ("B", &["A"])]);
    let err = build_execution_order(&config).unwrap_err();
    assert!(err.contains("cycle"), "error: {err}");
}

#[test]
fn dag_unknown_dep_errors() {
    let config = config_with_deps(&["A"], &[("A", &["nonexistent"])]);
    let err = build_execution_order(&config).unwrap_err();
    assert!(err.contains("unknown"), "error: {err}");
}

#[test]
fn dag_single_resource() {
    let config = config_with_deps(&["only"], &[]);
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order, vec!["only"]);
}

#[test]
fn dag_empty_config() {
    let config = ForjarConfig::default();
    let order = build_execution_order(&config).unwrap();
    assert!(order.is_empty());
}

#[test]
fn dag_deterministic_same_input() {
    let config = config_with_deps(&["z", "a", "m", "b"], &[("m", &["a"]), ("z", &["b"])]);
    let o1 = build_execution_order(&config).unwrap();
    let o2 = build_execution_order(&config).unwrap();
    assert_eq!(o1, o2, "must be deterministic");
}

// ============================================================================
// FJ-216: compute_parallel_waves
// ============================================================================

#[test]
fn waves_no_deps_single_wave() {
    let config = config_with_deps(&["a", "b", "c"], &[]);
    let waves = compute_parallel_waves(&config).unwrap();
    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0], vec!["a", "b", "c"]);
}

#[test]
fn waves_linear_chain_separate_waves() {
    let config = config_with_deps(&["A", "B", "C"], &[("B", &["A"]), ("C", &["B"])]);
    let waves = compute_parallel_waves(&config).unwrap();
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0], vec!["A"]);
    assert_eq!(waves[1], vec!["B"]);
    assert_eq!(waves[2], vec!["C"]);
}

#[test]
fn waves_diamond_three_waves() {
    let config = config_with_deps(
        &["A", "B", "C", "D"],
        &[("B", &["A"]), ("C", &["A"]), ("D", &["B", "C"])],
    );
    let waves = compute_parallel_waves(&config).unwrap();
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0], vec!["A"]);
    // B and C can run concurrently
    assert_eq!(waves[1].len(), 2);
    assert!(waves[1].contains(&"B".to_string()));
    assert!(waves[1].contains(&"C".to_string()));
    assert_eq!(waves[2], vec!["D"]);
}

#[test]
fn waves_cycle_detected() {
    let config = config_with_deps(&["A", "B"], &[("A", &["B"]), ("B", &["A"])]);
    let err = compute_parallel_waves(&config).unwrap_err();
    assert!(err.contains("cycle"), "error: {err}");
}

#[test]
fn waves_empty_config() {
    let config = ForjarConfig::default();
    let waves = compute_parallel_waves(&config).unwrap();
    assert!(waves.is_empty());
}

#[test]
fn waves_two_independent_chains() {
    // Chain1: A → B, Chain2: C → D (independent)
    let config = config_with_deps(&["A", "B", "C", "D"], &[("B", &["A"]), ("D", &["C"])]);
    let waves = compute_parallel_waves(&config).unwrap();
    assert_eq!(waves.len(), 2);
    // Wave 1: A and C (both have 0 in-degree)
    assert_eq!(waves[0].len(), 2);
    assert!(waves[0].contains(&"A".to_string()));
    assert!(waves[0].contains(&"C".to_string()));
    // Wave 2: B and D (both depend on wave 1 resources)
    assert_eq!(waves[1].len(), 2);
    assert!(waves[1].contains(&"B".to_string()));
    assert!(waves[1].contains(&"D".to_string()));
}

#[test]
fn waves_sorted_within_wave() {
    let config = config_with_deps(&["z-svc", "a-pkg", "m-file"], &[]);
    let waves = compute_parallel_waves(&config).unwrap();
    assert_eq!(waves[0], vec!["a-pkg", "m-file", "z-svc"]);
}

// ============================================================================
// Cross-cutting: changeset + DAG consistency
// ============================================================================

#[test]
fn dag_order_consistent_with_changeset_deps() {
    // If B depends on A and A changed, changeset propagation and DAG ordering agree
    let config = config_with_deps(&["A", "B"], &[("B", &["A"])]);
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order[0], "A");
    assert_eq!(order[1], "B");

    let resources = vec![
        ("A".into(), "m1".into(), "new-a".into()),
        ("B".into(), "m1".into(), "same-b".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "old-a".into());
    locks.insert("B@m1".into(), "same-b".into());
    let deps = vec![("B".into(), "A".into())];
    let cs = compute_minimal_changeset(&resources, &locks, &deps);
    assert_eq!(cs.changes_needed, 2);
}
