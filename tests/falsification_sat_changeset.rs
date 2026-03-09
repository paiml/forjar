//! FJ-045/046: SAT dependency solver and minimal changeset falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-045: SAT/SMT-based dependency resolution
//!   - build_sat_problem: variable mapping, clause generation
//!   - solve: satisfiable/unsatisfiable classification
//!   - DPLL backtracking, unit propagation
//!   - Conflict clause reporting
//! - FJ-046: Minimal change set computation
//!   - compute_minimal_changeset: new, unchanged, changed resources
//!   - Dependency propagation through transitive chains
//!   - verify_minimality predicate
//!   - Serialization roundtrips
//!
//! Usage: cargo test --test falsification_sat_changeset

use forjar::core::planner::minimal_changeset::{
    compute_minimal_changeset, verify_minimality, ChangeCandidate, MinimalChangeSet,
};
use forjar::core::planner::sat_deps::{build_sat_problem, solve, SatProblem, SatResult};
use std::collections::BTreeMap;

// ============================================================================
// FJ-045: build_sat_problem — variable mapping
// ============================================================================

#[test]
fn build_problem_maps_resources_to_vars() {
    let resources = vec!["nginx".into(), "mysql".into()];
    let deps = vec![];
    let problem = build_sat_problem(&resources, &deps);
    assert_eq!(problem.num_vars, 2);
    assert_eq!(problem.var_names[&1], "nginx");
    assert_eq!(problem.var_names[&2], "mysql");
}

#[test]
fn build_problem_generates_unit_clauses() {
    let resources = vec!["A".into(), "B".into()];
    let deps = vec![];
    let problem = build_sat_problem(&resources, &deps);
    // Each resource gets a unit clause requiring inclusion
    assert_eq!(problem.clauses.len(), 2);
    assert!(problem.clauses.contains(&vec![1]));
    assert!(problem.clauses.contains(&vec![2]));
}

#[test]
fn build_problem_generates_implication_clauses() {
    let resources = vec!["A".into(), "B".into()];
    let deps = vec![("B".into(), "A".into())]; // B depends on A
    let problem = build_sat_problem(&resources, &deps);
    // 1 implication (!B || A) + 2 unit clauses = 3 total
    assert_eq!(problem.clauses.len(), 3);
    // Implication B→A is encoded as (!B || A)
    assert!(problem.clauses.contains(&vec![-2, 1]));
}

#[test]
fn build_problem_skips_unknown_dependency_target() {
    let resources = vec!["A".into()];
    let deps = vec![("A".into(), "MISSING".into())];
    let problem = build_sat_problem(&resources, &deps);
    // Only unit clause for A, no implication because MISSING is not in var_map
    assert_eq!(problem.clauses.len(), 1);
}

// ============================================================================
// FJ-045: solve — satisfiable cases
// ============================================================================

#[test]
fn solve_no_deps_all_true() {
    let resources = vec!["X".into(), "Y".into(), "Z".into()];
    let problem = build_sat_problem(&resources, &[]);
    let result = solve(&problem);
    match result {
        SatResult::Satisfiable { assignment } => {
            assert_eq!(assignment.len(), 3);
            assert!(assignment["X"]);
            assert!(assignment["Y"]);
            assert!(assignment["Z"]);
        }
        _ => panic!("independent resources should be satisfiable"),
    }
}

#[test]
fn solve_linear_chain_satisfiable() {
    let resources = vec!["A".into(), "B".into(), "C".into()];
    let deps = vec![("B".into(), "A".into()), ("C".into(), "B".into())];
    let problem = build_sat_problem(&resources, &deps);
    let result = solve(&problem);
    assert!(matches!(result, SatResult::Satisfiable { .. }));
}

#[test]
fn solve_diamond_dependency_satisfiable() {
    let resources = vec!["A".into(), "B".into(), "C".into(), "D".into()];
    let deps = vec![
        ("B".into(), "A".into()),
        ("C".into(), "A".into()),
        ("D".into(), "B".into()),
        ("D".into(), "C".into()),
    ];
    let problem = build_sat_problem(&resources, &deps);
    let result = solve(&problem);
    assert!(matches!(result, SatResult::Satisfiable { .. }));
}

#[test]
fn solve_single_resource() {
    let resources = vec!["solo".into()];
    let problem = build_sat_problem(&resources, &[]);
    match solve(&problem) {
        SatResult::Satisfiable { assignment } => {
            assert_eq!(assignment.len(), 1);
            assert!(assignment["solo"]);
        }
        _ => panic!("single resource should be satisfiable"),
    }
}

#[test]
fn solve_ten_resource_chain() {
    let resources: Vec<String> = (0..10).map(|i| format!("r{i}")).collect();
    let deps: Vec<(String, String)> = (1..10)
        .map(|i| (format!("r{i}"), format!("r{}", i - 1)))
        .collect();
    let problem = build_sat_problem(&resources, &deps);
    match solve(&problem) {
        SatResult::Satisfiable { assignment } => {
            assert_eq!(assignment.len(), 10);
            assert!(assignment.values().all(|&v| v));
        }
        _ => panic!("linear chain should be satisfiable"),
    }
}

// ============================================================================
// FJ-045: solve — unsatisfiable cases
// ============================================================================

#[test]
fn solve_contradiction_unsatisfiable() {
    let mut var_names = BTreeMap::new();
    var_names.insert(1, "A".into());
    let problem = SatProblem {
        num_vars: 1,
        clauses: vec![vec![1], vec![-1]], // A AND !A
        var_names,
    };
    let result = solve(&problem);
    assert!(matches!(result, SatResult::Unsatisfiable { .. }));
}

#[test]
fn solve_unsatisfiable_reports_conflict_names() {
    let mut var_names = BTreeMap::new();
    var_names.insert(1, "pkg-a".into());
    var_names.insert(2, "pkg-b".into());
    let problem = SatProblem {
        num_vars: 2,
        clauses: vec![vec![1], vec![-1, 2], vec![-2]],
        var_names,
    };
    match solve(&problem) {
        SatResult::Unsatisfiable { conflict_clause } => {
            assert!(!conflict_clause.is_empty());
        }
        _ => panic!("expected unsatisfiable"),
    }
}

#[test]
fn solve_unsatisfiable_negative_literal_formatting() {
    let mut var_names = BTreeMap::new();
    var_names.insert(1, "svc".into());
    // Create unsatisfiable problem with only a negative unit clause
    let problem = SatProblem {
        num_vars: 1,
        clauses: vec![vec![1], vec![-1]],
        var_names,
    };
    match solve(&problem) {
        SatResult::Unsatisfiable { conflict_clause } => {
            // Should include "svc" or "!svc" in conflict names
            let has_name = conflict_clause.iter().any(|c| c.contains("svc"));
            assert!(
                has_name,
                "conflict should name the variable: {conflict_clause:?}"
            );
        }
        _ => panic!("expected unsatisfiable"),
    }
}

// ============================================================================
// FJ-045: SatResult serialization
// ============================================================================

#[test]
fn sat_result_satisfiable_serializes() {
    let result = SatResult::Satisfiable {
        assignment: BTreeMap::from([("A".into(), true), ("B".into(), false)]),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"Satisfiable\""));
    assert!(json.contains("\"A\":true"));
    assert!(json.contains("\"B\":false"));
}

#[test]
fn sat_result_unsatisfiable_serializes() {
    let result = SatResult::Unsatisfiable {
        conflict_clause: vec!["pkg-a".into(), "!pkg-b".into()],
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"Unsatisfiable\""));
    assert!(json.contains("pkg-a"));
}

// ============================================================================
// FJ-045: DPLL backtracking
// ============================================================================

#[test]
fn solve_requires_backtracking() {
    let mut var_names = BTreeMap::new();
    var_names.insert(1, "X".into());
    var_names.insert(2, "Y".into());
    var_names.insert(3, "Z".into());
    let problem = SatProblem {
        num_vars: 3,
        clauses: vec![
            vec![1, 2],  // X OR Y
            vec![-1, 3], // !X OR Z
            vec![2, -3], // Y OR !Z
            vec![1, -2], // X OR !Y
        ],
        var_names,
    };
    let result = solve(&problem);
    assert!(
        matches!(result, SatResult::Satisfiable { .. }),
        "should find solution via backtracking"
    );
}

// ============================================================================
// FJ-046: compute_minimal_changeset — no changes needed
// ============================================================================

#[test]
fn changeset_all_converged_no_changes() {
    let resources = vec![
        ("A".into(), "m1".into(), "h1".into()),
        ("B".into(), "m1".into(), "h2".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "h1".into());
    locks.insert("B@m1".into(), "h2".into());

    let result = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(result.changes_needed, 0);
    assert_eq!(result.changes_skipped, 2);
    assert_eq!(result.total_resources, 2);
    assert!(result.is_provably_minimal);
}

// ============================================================================
// FJ-046: compute_minimal_changeset — single hash change
// ============================================================================

#[test]
fn changeset_single_hash_change() {
    let resources = vec![
        ("A".into(), "m1".into(), "h1-new".into()),
        ("B".into(), "m1".into(), "h2".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "h1-old".into());
    locks.insert("B@m1".into(), "h2".into());

    let result = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(result.changes_needed, 1);
    assert!(result.candidates[0].necessary);
    assert!(!result.candidates[1].necessary);
}

// ============================================================================
// FJ-046: compute_minimal_changeset — new resource
// ============================================================================

#[test]
fn changeset_new_resource_is_necessary() {
    let resources = vec![("NEW".into(), "m1".into(), "hash-new".into())];
    let locks = BTreeMap::new();

    let result = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(result.changes_needed, 1);
    assert!(result.candidates[0].necessary);
    assert!(result.candidates[0].current_hash.is_none());
}

// ============================================================================
// FJ-046: dependency propagation
// ============================================================================

#[test]
fn changeset_dependency_propagation() {
    let resources = vec![
        ("A".into(), "m1".into(), "h-a-new".into()),
        ("B".into(), "m1".into(), "h-b".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "h-a-old".into());
    locks.insert("B@m1".into(), "h-b".into());
    let deps = vec![("B".into(), "A".into())]; // B depends on A

    let result = compute_minimal_changeset(&resources, &locks, &deps);
    assert_eq!(result.changes_needed, 2);
    assert!(result.candidates[0].necessary, "A changed directly");
    assert!(result.candidates[1].necessary, "B depends on changed A");
}

#[test]
fn changeset_transitive_propagation() {
    let resources = vec![
        ("A".into(), "m1".into(), "h-a-new".into()),
        ("B".into(), "m1".into(), "h-b".into()),
        ("C".into(), "m1".into(), "h-c".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "h-a-old".into());
    locks.insert("B@m1".into(), "h-b".into());
    locks.insert("C@m1".into(), "h-c".into());
    // C depends on B, B depends on A — all should propagate
    let deps = vec![("B".into(), "A".into()), ("C".into(), "B".into())];

    let result = compute_minimal_changeset(&resources, &locks, &deps);
    assert_eq!(result.changes_needed, 3);
}

#[test]
fn changeset_no_propagation_without_change() {
    let resources = vec![
        ("A".into(), "m1".into(), "h-a".into()),
        ("B".into(), "m1".into(), "h-b".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A@m1".into(), "h-a".into());
    locks.insert("B@m1".into(), "h-b".into());
    let deps = vec![("B".into(), "A".into())];

    let result = compute_minimal_changeset(&resources, &locks, &deps);
    assert_eq!(result.changes_needed, 0, "no change means no propagation");
}

// ============================================================================
// FJ-046: verify_minimality
// ============================================================================

#[test]
fn verify_minimality_nonempty_true() {
    let cs = MinimalChangeSet {
        total_resources: 2,
        changes_needed: 1,
        changes_skipped: 1,
        candidates: vec![ChangeCandidate {
            resource: "A".into(),
            machine: "m1".into(),
            current_hash: None,
            desired_hash: "h".into(),
            necessary: true,
        }],
        is_provably_minimal: true,
    };
    assert!(verify_minimality(&cs));
}

#[test]
fn verify_minimality_empty_changeset() {
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
fn verify_minimality_all_converged() {
    let cs = MinimalChangeSet {
        total_resources: 3,
        changes_needed: 0,
        changes_skipped: 3,
        candidates: vec![
            ChangeCandidate {
                resource: "A".into(),
                machine: "m1".into(),
                current_hash: Some("h1".into()),
                desired_hash: "h1".into(),
                necessary: false,
            },
            ChangeCandidate {
                resource: "B".into(),
                machine: "m1".into(),
                current_hash: Some("h2".into()),
                desired_hash: "h2".into(),
                necessary: false,
            },
            ChangeCandidate {
                resource: "C".into(),
                machine: "m1".into(),
                current_hash: Some("h3".into()),
                desired_hash: "h3".into(),
                necessary: false,
            },
        ],
        is_provably_minimal: true,
    };
    assert!(verify_minimality(&cs));
}

// ============================================================================
// FJ-046: MinimalChangeSet serialization
// ============================================================================

#[test]
fn changeset_serializes_to_json() {
    let cs = MinimalChangeSet {
        total_resources: 1,
        changes_needed: 1,
        changes_skipped: 0,
        candidates: vec![ChangeCandidate {
            resource: "nginx".into(),
            machine: "web-01".into(),
            current_hash: Some("old".into()),
            desired_hash: "new".into(),
            necessary: true,
        }],
        is_provably_minimal: true,
    };
    let json = serde_json::to_string(&cs).unwrap();
    assert!(json.contains("\"is_provably_minimal\":true"));
    assert!(json.contains("\"nginx\""));
    assert!(json.contains("\"web-01\""));
}

// ============================================================================
// FJ-046: ChangeCandidate fields
// ============================================================================

#[test]
fn change_candidate_fields() {
    let c = ChangeCandidate {
        resource: "pkg-nginx".into(),
        machine: "web-01".into(),
        current_hash: Some("abc".into()),
        desired_hash: "def".into(),
        necessary: true,
    };
    assert_eq!(c.resource, "pkg-nginx");
    assert_eq!(c.machine, "web-01");
    assert_eq!(c.current_hash.as_deref(), Some("abc"));
    assert_eq!(c.desired_hash, "def");
    assert!(c.necessary);
}

#[test]
fn change_candidate_new_resource_no_current_hash() {
    let c = ChangeCandidate {
        resource: "new-svc".into(),
        machine: "m1".into(),
        current_hash: None,
        desired_hash: "h1".into(),
        necessary: true,
    };
    assert!(c.current_hash.is_none());
    assert!(c.necessary);
}

// ============================================================================
// FJ-046: multi-machine changeset
// ============================================================================

#[test]
fn changeset_multi_machine_different_names() {
    let resources = vec![
        ("A-m1".into(), "m1".into(), "h-a".into()),
        ("A-m2".into(), "m2".into(), "h-a-new".into()),
    ];
    let mut locks = BTreeMap::new();
    locks.insert("A-m1@m1".into(), "h-a".into());
    locks.insert("A-m2@m2".into(), "h-a-old".into());

    let result = compute_minimal_changeset(&resources, &locks, &[]);
    assert_eq!(result.changes_needed, 1);
    // m1 is converged, m2 changed
    assert!(!result.candidates[0].necessary);
    assert!(result.candidates[1].necessary);
}
