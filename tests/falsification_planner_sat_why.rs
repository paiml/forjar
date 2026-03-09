//! FJ-045/1379: SAT dependency resolution and why explanation falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-045: SAT dependency resolution (build_sat_problem, solve)
//! - FJ-1379: Why explanation (explain_why, format_why)
//!
//! Usage: cargo test --test falsification_planner_sat_why

use forjar::core::planner::hash_desired_state;
use forjar::core::planner::sat_deps::{build_sat_problem, solve, SatProblem, SatResult};
use forjar::core::planner::why::{explain_why, format_why};
use forjar::core::types::*;
use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};

fn resource(rtype: ResourceType) -> Resource {
    Resource {
        resource_type: rtype,
        ..Default::default()
    }
}

fn lock_with(resources: &[(&str, ResourceLock)]) -> StateLock {
    let mut map = IndexMap::new();
    for (id, rl) in resources {
        map.insert(id.to_string(), rl.clone());
    }
    StateLock {
        schema: "1".into(),
        machine: "m1".into(),
        hostname: "host".into(),
        generated_at: "2026-03-09T00:00:00Z".into(),
        generator: "test".into(),
        blake3_version: "1".into(),
        resources: map,
    }
}

fn rl(rtype: ResourceType, status: ResourceStatus, hash: &str) -> ResourceLock {
    ResourceLock {
        resource_type: rtype,
        status,
        applied_at: None,
        duration_seconds: None,
        hash: hash.into(),
        details: HashMap::new(),
    }
}

// ============================================================================
// FJ-045: SAT — build_sat_problem structure
// ============================================================================

#[test]
fn sat_build_problem_structure() {
    let resources = vec!["A".into(), "B".into(), "C".into()];
    let deps = vec![("B".into(), "A".into()), ("C".into(), "B".into())];
    let problem = build_sat_problem(&resources, &deps);
    assert_eq!(problem.num_vars, 3);
    assert_eq!(problem.clauses.len(), 5); // 2 deps + 3 units
    assert_eq!(problem.var_names.len(), 3);
}

#[test]
fn sat_satisfiable_linear_chain() {
    let resources: Vec<String> = (0..5).map(|i| format!("r{i}")).collect();
    let deps: Vec<(String, String)> = (1..5)
        .map(|i| (format!("r{i}"), format!("r{}", i - 1)))
        .collect();
    if let SatResult::Satisfiable { assignment } = solve(&build_sat_problem(&resources, &deps)) {
        assert!(assignment.values().all(|&v| v));
    } else {
        panic!("linear chain should be satisfiable");
    }
}

#[test]
fn sat_satisfiable_no_deps() {
    assert!(matches!(
        solve(&build_sat_problem(&["X".into(), "Y".into()], &[])),
        SatResult::Satisfiable { .. }
    ));
}

#[test]
fn sat_satisfiable_diamond() {
    let resources = vec!["A".into(), "B".into(), "C".into(), "D".into()];
    let deps = vec![
        ("B".into(), "A".into()),
        ("C".into(), "A".into()),
        ("D".into(), "B".into()),
        ("D".into(), "C".into()),
    ];
    assert!(matches!(
        solve(&build_sat_problem(&resources, &deps)),
        SatResult::Satisfiable { .. }
    ));
}

#[test]
fn sat_unsatisfiable_contradiction() {
    let problem = SatProblem {
        num_vars: 1,
        clauses: vec![vec![1], vec![-1]],
        var_names: BTreeMap::from([(1, "A".into())]),
    };
    assert!(matches!(solve(&problem), SatResult::Unsatisfiable { .. }));
}

#[test]
fn sat_unsatisfiable_chain_conflict() {
    let problem = SatProblem {
        num_vars: 2,
        clauses: vec![vec![1], vec![-1, 2], vec![-2]],
        var_names: BTreeMap::from([(1, "pkg-a".into()), (2, "pkg-b".into())]),
    };
    match solve(&problem) {
        SatResult::Unsatisfiable { conflict_clause } => assert!(!conflict_clause.is_empty()),
        _ => panic!("should be unsatisfiable"),
    }
}

#[test]
fn sat_single_resource() {
    if let SatResult::Satisfiable { assignment } = solve(&build_sat_problem(&["solo".into()], &[]))
    {
        assert!(assignment["solo"]);
    } else {
        panic!("single resource should be satisfiable");
    }
}

#[test]
fn sat_unknown_dep_skipped() {
    let problem = build_sat_problem(&["A".into()], &[("A".into(), "MISSING".into())]);
    assert_eq!(problem.clauses.len(), 1);
}

#[test]
fn sat_result_serializes() {
    let result = SatResult::Satisfiable {
        assignment: BTreeMap::from([("A".into(), true)]),
    };
    assert!(serde_json::to_string(&result)
        .unwrap()
        .contains("Satisfiable"));
}

// ============================================================================
// FJ-1379: explain_why — absent state
// ============================================================================

#[test]
fn why_absent_no_lock_noop() {
    let mut r = resource(ResourceType::File);
    r.state = Some("absent".into());
    let reason = explain_why("f1", &r, "m1", &HashMap::new());
    assert_eq!(reason.action, PlanAction::NoOp);
    assert!(reason.reasons[0].contains("not in lock"));
}

#[test]
fn why_absent_in_lock_destroy() {
    let mut r = resource(ResourceType::File);
    r.state = Some("absent".into());
    let sl = lock_with(&[(
        "f1",
        rl(ResourceType::File, ResourceStatus::Converged, "abc"),
    )]);
    let mut locks = HashMap::new();
    locks.insert("m1".into(), sl);
    let reason = explain_why("f1", &r, "m1", &locks);
    assert_eq!(reason.action, PlanAction::Destroy);
    assert!(reason.reasons[0].contains("will be removed"));
}

#[test]
fn why_absent_different_resource_noop() {
    let mut r = resource(ResourceType::File);
    r.state = Some("absent".into());
    let sl = lock_with(&[(
        "other",
        rl(ResourceType::File, ResourceStatus::Converged, "abc"),
    )]);
    let mut locks = HashMap::new();
    locks.insert("m1".into(), sl);
    assert_eq!(explain_why("f1", &r, "m1", &locks).action, PlanAction::NoOp);
}

// ============================================================================
// FJ-1379: explain_why — present state
// ============================================================================

#[test]
fn why_present_no_lock_first_apply() {
    let reason = explain_why("f1", &resource(ResourceType::File), "m1", &HashMap::new());
    assert_eq!(reason.action, PlanAction::Create);
    assert!(reason.reasons[0].contains("first apply"));
}

#[test]
fn why_present_new_resource() {
    let sl = lock_with(&[(
        "other",
        rl(ResourceType::File, ResourceStatus::Converged, "abc"),
    )]);
    let mut locks = HashMap::new();
    locks.insert("m1".into(), sl);
    let reason = explain_why("f1", &resource(ResourceType::File), "m1", &locks);
    assert_eq!(reason.action, PlanAction::Create);
    assert!(reason.reasons[0].contains("new resource"));
}

#[test]
fn why_present_failed_retry() {
    let hash = hash_desired_state(&resource(ResourceType::File));
    let sl = lock_with(&[("f1", rl(ResourceType::File, ResourceStatus::Failed, &hash))]);
    let mut locks = HashMap::new();
    locks.insert("m1".into(), sl);
    let reason = explain_why("f1", &resource(ResourceType::File), "m1", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason.reasons[0].contains("retry"));
}

#[test]
fn why_present_drifted() {
    let hash = hash_desired_state(&resource(ResourceType::File));
    let sl = lock_with(&[("f1", rl(ResourceType::File, ResourceStatus::Drifted, &hash))]);
    let mut locks = HashMap::new();
    locks.insert("m1".into(), sl);
    let reason = explain_why("f1", &resource(ResourceType::File), "m1", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason.reasons[0].contains("drifted"));
}

// ============================================================================
// FJ-1379: explain_why — hash match / change
// ============================================================================

#[test]
fn why_present_hash_unchanged_noop() {
    let r = resource(ResourceType::File);
    let hash = hash_desired_state(&r);
    let sl = lock_with(&[(
        "f1",
        rl(ResourceType::File, ResourceStatus::Converged, &hash),
    )]);
    let mut locks = HashMap::new();
    locks.insert("m1".into(), sl);
    let reason = explain_why("f1", &r, "m1", &locks);
    assert_eq!(reason.action, PlanAction::NoOp);
    assert!(reason.reasons[0].contains("hash unchanged"));
}

#[test]
fn why_present_hash_changed_update() {
    let sl = lock_with(&[(
        "f1",
        rl(ResourceType::File, ResourceStatus::Converged, "oldhash"),
    )]);
    let mut locks = HashMap::new();
    locks.insert("m1".into(), sl);
    let reason = explain_why("f1", &resource(ResourceType::File), "m1", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason.reasons[0].contains("hash changed"));
}

#[test]
fn why_hash_changed_with_content_diff() {
    let mut r = resource(ResourceType::File);
    r.content = Some("new content".into());
    let mut rl_entry = rl(ResourceType::File, ResourceStatus::Converged, "oldhash");
    rl_entry.details.insert(
        "content_hash".into(),
        serde_yaml_ng::Value::String("old_hash".into()),
    );
    let sl = lock_with(&[("f1", rl_entry)]);
    let mut locks = HashMap::new();
    locks.insert("m1".into(), sl);
    let reason = explain_why("f1", &r, "m1", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason.reasons.iter().any(|r| r.contains("content changed")));
}

#[test]
fn why_hash_changed_with_version_diff() {
    let mut r = resource(ResourceType::Package);
    r.version = Some("2.0".into());
    let mut rl_entry = rl(ResourceType::Package, ResourceStatus::Converged, "oldhash");
    rl_entry
        .details
        .insert("version".into(), serde_yaml_ng::Value::String("1.0".into()));
    let sl = lock_with(&[("p1", rl_entry)]);
    let mut locks = HashMap::new();
    locks.insert("m1".into(), sl);
    let reason = explain_why("p1", &r, "m1", &locks);
    assert_eq!(reason.action, PlanAction::Update);
    assert!(reason.reasons.iter().any(|r| r.contains("version changed")));
}

// ============================================================================
// FJ-1379: format_why
// ============================================================================

#[test]
fn why_format_output() {
    let reason = explain_why("f1", &resource(ResourceType::File), "m1", &HashMap::new());
    let formatted = format_why(&reason);
    assert!(formatted.contains("f1") && formatted.contains("m1") && formatted.contains("Create"));
}

#[test]
fn why_format_multiple_reasons() {
    let mut r = resource(ResourceType::File);
    r.content = Some("data".into());
    r.path = Some("/etc/new".into());
    let mut rl_entry = rl(ResourceType::File, ResourceStatus::Converged, "oldhash");
    rl_entry.details.insert(
        "content_hash".into(),
        serde_yaml_ng::Value::String("old".into()),
    );
    rl_entry.details.insert(
        "path".into(),
        serde_yaml_ng::Value::String("/etc/old".into()),
    );
    let sl = lock_with(&[("f1", rl_entry)]);
    let mut locks = HashMap::new();
    locks.insert("m1".into(), sl);
    let formatted = format_why(&explain_why("f1", &r, "m1", &locks));
    assert!(formatted.contains("  - "));
}
