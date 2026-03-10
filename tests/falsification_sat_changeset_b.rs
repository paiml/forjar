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
