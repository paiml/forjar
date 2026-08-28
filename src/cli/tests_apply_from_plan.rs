//! Unit tests for the plan-file apply path (Refs #358).

use super::*;
use crate::core::types::{PlanAction, PlannedChange, ResourceType};

#[test]
fn a_sealed_plan_may_legitimately_report_no_changes() {
    assert!(check_empty_plan_is_trustworthy(true).is_ok());
}

#[test]
fn an_unsealed_plan_reporting_no_changes_is_refused() {
    let err = check_empty_plan_is_trustworthy(false).unwrap_err();
    assert!(err.contains(plan_file::FORMAT_V1), "{err}");
    assert!(err.contains(plan_file::FORMAT_V2), "{err}");
    assert!(err.contains("unauthenticated counter"), "{err}");
}

#[test]
fn prepare_config_reports_a_missing_file_rather_than_panicking() {
    let err = prepare_config(Path::new("/nonexistent/forjar.yaml"), None, None).unwrap_err();
    assert!(!err.is_empty());
}

fn change(id: &str, machine: &str, action: PlanAction) -> PlannedChange {
    PlannedChange {
        resource_id: id.to_string(),
        machine: machine.to_string(),
        resource_type: ResourceType::File,
        action,
        description: format!("{id}: planned"),
    }
}

fn plan_of(changes: Vec<PlannedChange>) -> types::ExecutionPlan {
    let mut plan = types::ExecutionPlan {
        name: "t".to_string(),
        execution_order: changes.iter().map(|c| c.resource_id.clone()).collect(),
        changes,
        to_create: 0,
        to_update: 0,
        to_destroy: 0,
        unchanged: 0,
    };
    for c in &plan.changes {
        match c.action {
            PlanAction::Create => plan.to_create += 1,
            PlanAction::Update => plan.to_update += 1,
            PlanAction::Destroy => plan.to_destroy += 1,
            PlanAction::NoOp => plan.unchanged += 1,
        }
    }
    plan
}

// ── check_plan_still_holds ──

/// THE ADVERSARY, at the unit level: an empty body over a world with a create
/// pending. Every leg of a re-sealed plan verifies; this is the check that does
/// not.
#[test]
fn an_empty_body_is_refused_when_the_planner_still_has_work() {
    let fresh = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    let err = check_plan_still_holds(&plan_of(vec![]), &fresh).unwrap_err();
    assert!(err.starts_with(PLAN_STALE), "{err}");
    assert!(err.contains("alpha on web"), "{err}");
}

/// The same attack with the list KEPT and the pending create relabelled, so the
/// counters still partition their list and `check_body_partition` is silent.
#[test]
fn a_create_relabelled_as_a_no_op_is_refused() {
    let fresh = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    let lie = plan_of(vec![change("alpha", "web", PlanAction::NoOp)]);
    let err = check_plan_still_holds(&lie, &fresh).unwrap_err();
    assert!(err.starts_with(PLAN_STALE), "{err}");
    assert!(err.contains("NO-OP") && err.contains("CREATE"), "{err}");
}

/// A plan naming a pair the planner does not produce at all.
#[test]
fn a_pair_the_planner_does_not_produce_is_refused() {
    let fresh = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    let lie = plan_of(vec![change("ghost", "web", PlanAction::Create)]);
    let err = check_plan_still_holds(&lie, &fresh).unwrap_err();
    assert!(err.starts_with(PLAN_STALE), "{err}");
    assert!(err.contains("ghost"), "{err}");
}

/// GREEN GUARD: a FILTERED plan is narrower than the config by design, and must
/// not be refused for it. `plan -r alpha` names one of two resources.
#[test]
fn a_plan_narrower_than_the_config_is_honoured() {
    let fresh = plan_of(vec![
        change("alpha", "web", PlanAction::Create),
        change("bravo", "db", PlanAction::Create),
    ]);
    let filtered = plan_of(vec![change("alpha", "web", PlanAction::Create)]);
    assert!(check_plan_still_holds(&filtered, &fresh).is_ok());
}

/// GREEN GUARD: an honest zero — a converged stack still LISTS its resources as
/// `NoOp`, and the planner agrees.
#[test]
fn an_honest_converged_plan_is_honoured() {
    let converged = plan_of(vec![change("alpha", "web", PlanAction::NoOp)]);
    assert!(check_plan_still_holds(&converged, &converged).is_ok());
}

/// GREEN GUARD: an empty body over an empty world is honestly empty.
#[test]
fn an_empty_body_over_an_empty_world_is_honoured() {
    assert!(check_plan_still_holds(&plan_of(vec![]), &plan_of(vec![])).is_ok());
}

/// A world holding only no-ops is not "pending", so an empty body over it is
/// not a lie about anything that would have been done.
#[test]
fn an_empty_body_over_a_world_of_no_ops_is_honoured() {
    let fresh = plan_of(vec![change("alpha", "web", PlanAction::NoOp)]);
    assert!(check_plan_still_holds(&plan_of(vec![]), &fresh).is_ok());
}

// ── check_machine_is_in_scope ──

#[test]
fn no_machine_selector_is_always_in_scope() {
    let scope =
        executor::PlanScope::from_plan(&plan_of(vec![change("alpha", "web", PlanAction::Create)]));
    assert!(check_machine_is_in_scope(&scope, None).is_ok());
}

#[test]
fn a_machine_the_plan_names_narrows_it() {
    let scope = executor::PlanScope::from_plan(&plan_of(vec![
        change("alpha", "web", PlanAction::Create),
        change("bravo", "db", PlanAction::Create),
    ]));
    assert!(check_machine_is_in_scope(&scope, Some("web")).is_ok());
}

#[test]
fn a_machine_outside_the_plan_is_an_error_naming_what_is_covered() {
    let scope = executor::PlanScope::from_plan(&plan_of(vec![
        change("alpha", "web", PlanAction::Create),
        change("bravo", "db", PlanAction::Create),
    ]));
    let err = check_machine_is_in_scope(&scope, Some("ghost")).unwrap_err();
    assert!(err.contains("ghost"), "{err}");
    assert!(err.contains("db, web"), "the plan's machines: {err}");
    assert!(err.contains("narrow"), "{err}");
}

/// A machine whose only change is a `NoOp` is not IN the scope — the plan asked
/// for nothing there — so `-m` on it is refused rather than silently green.
#[test]
fn a_machine_the_plan_only_no_ops_is_not_in_scope() {
    let scope = executor::PlanScope::from_plan(&plan_of(vec![
        change("alpha", "web", PlanAction::Create),
        change("bravo", "db", PlanAction::NoOp),
    ]));
    assert!(check_machine_is_in_scope(&scope, Some("db")).is_err());
}

// ── sigil ──

#[test]
fn every_action_has_a_distinct_sigil() {
    let all = [
        sigil(&PlanAction::Create),
        sigil(&PlanAction::Update),
        sigil(&PlanAction::Destroy),
        sigil(&PlanAction::NoOp),
    ];
    let unique: std::collections::HashSet<&&str> = all.iter().collect();
    assert_eq!(unique.len(), all.len(), "{all:?}");
}
