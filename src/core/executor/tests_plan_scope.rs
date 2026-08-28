//! Unit tests for the reviewed-delta scope (Refs #358).

use super::*;
use crate::core::types::{PlannedChange, ResourceType};

fn change(id: &str, machine: &str, action: PlanAction) -> PlannedChange {
    PlannedChange {
        resource_id: id.to_string(),
        machine: machine.to_string(),
        resource_type: ResourceType::File,
        description: format!("{id}: {action}"),
        action,
    }
}

fn plan(changes: Vec<PlannedChange>) -> ExecutionPlan {
    let mut p = ExecutionPlan {
        name: "scope".to_string(),
        execution_order: changes.iter().map(|c| c.resource_id.clone()).collect(),
        changes,
        to_create: 0,
        to_update: 0,
        to_destroy: 0,
        unchanged: 0,
    };
    recount(&mut p);
    p
}

#[test]
fn a_scope_is_the_non_noop_pairs() {
    let scope = PlanScope::from_plan(&plan(vec![
        change("a", "web", PlanAction::Update),
        change("b", "web", PlanAction::NoOp),
        change("c", "db", PlanAction::Create),
    ]));
    assert_eq!(scope.len(), 2);
    assert!(scope.covers("web", "a"));
    assert!(
        !scope.covers("web", "b"),
        "a NoOp was not reviewed as a change"
    );
    assert!(scope.covers("db", "c"));
    assert!(
        !scope.covers("db", "a"),
        "the machine is part of the identity"
    );
    assert_eq!(scope.machine_names(), vec!["db", "web"]);
    assert!(scope.covers_machine("web"));
    assert!(!scope.covers_machine("ghost"));
}

#[test]
fn an_empty_plan_yields_an_empty_scope() {
    let scope = PlanScope::from_plan(&plan(vec![change("a", "web", PlanAction::NoOp)]));
    assert!(scope.is_empty());
    assert_eq!(scope.len(), 0);
}

#[test]
fn restricting_demotes_only_what_the_plan_did_not_ask_for() {
    let reviewed = plan(vec![change("a", "web", PlanAction::Update)]);
    let scope = PlanScope::from_plan(&reviewed);

    // The live plan has drifted: 'b' now wants converging too.
    let live = plan(vec![
        change("a", "web", PlanAction::Update),
        change("b", "web", PlanAction::Update),
    ]);
    assert_eq!(live.to_update, 2);

    let restricted = restrict(live, Some(&scope));
    assert_eq!(restricted.changes[0].action, PlanAction::Update, "reviewed");
    assert_eq!(
        restricted.changes[1].action,
        PlanAction::NoOp,
        "drifted after the plan was made — not in the reviewed delta"
    );
    assert_eq!(restricted.to_update, 1);
    assert_eq!(restricted.unchanged, 1);
    assert_eq!(
        restricted.to_update + restricted.to_create + restricted.to_destroy + restricted.unchanged,
        restricted.changes.len() as u32,
        "the counters must still partition the change list"
    );
}

#[test]
fn no_scope_leaves_the_plan_alone() {
    let live = plan(vec![
        change("a", "web", PlanAction::Update),
        change("b", "web", PlanAction::Create),
    ]);
    let restricted = restrict(live, None);
    assert_eq!(restricted.to_update, 1);
    assert_eq!(restricted.to_create, 1);
}

#[test]
fn a_scope_from_another_machine_does_not_leak() {
    let reviewed = plan(vec![change("a", "web", PlanAction::Update)]);
    let scope = PlanScope::from_plan(&reviewed);
    let live = plan(vec![change("a", "db", PlanAction::Update)]);
    let restricted = restrict(live, Some(&scope));
    assert_eq!(restricted.changes[0].action, PlanAction::NoOp);
}
