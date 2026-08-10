//! GH-214: a selector must apply to EVERY part of a response.
//!
//! `machine_filter` was passed only to `print_plan`, so the text summary and
//! `--json` were computed from the UNFILTERED plan. Measured on 1.12.3:
//!
//! ```text
//!   $ forjar plan -m ghost            # a machine that does not exist
//!   Plan: 2 to add, ...               # empty body, count of 2
//!   $ forjar plan -m ghost --json
//!   to_create = 2
//! ```
//!
//! This module is the structural ratchet referenced by
//! contracts/selector-scope-v1.yaml FALSIFY-SEL-007. It must contain at
//! least one test, or the contract's only scaling guard is vacuous — a
//! `cargo test` name filter that matches nothing runs 0 tests and exits 0.

use crate::core::types::{ExecutionPlan, PlanAction, PlannedChange, ResourceType};

fn change(id: &str, machine: &str, action: PlanAction) -> PlannedChange {
    PlannedChange {
        resource_id: id.to_string(),
        machine: machine.to_string(),
        resource_type: ResourceType::File,
        action,
        description: String::new(),
    }
}

fn plan_of(changes: Vec<PlannedChange>) -> ExecutionPlan {
    ExecutionPlan {
        name: "sel".into(),
        changes,
        execution_order: Vec::new(),
        to_create: 0,
        to_update: 0,
        to_destroy: 0,
        unchanged: 0,
    }
}

/// Mirrors the filter+recount applied in `cmd_plan`.
fn filter_and_recount(plan: &mut ExecutionPlan, machine: &str) {
    plan.changes.retain(|c| c.machine == machine);
    plan.to_create = 0;
    plan.to_update = 0;
    plan.to_destroy = 0;
    plan.unchanged = 0;
    for c in &plan.changes {
        match c.action {
            PlanAction::Create => plan.to_create += 1,
            PlanAction::Update => plan.to_update += 1,
            PlanAction::Destroy => plan.to_destroy += 1,
            PlanAction::NoOp => plan.unchanged += 1,
        }
    }
}

fn two_machine_plan() -> ExecutionPlan {
    let mut p = plan_of(vec![
        change("a", "local", PlanAction::Create),
        change("b", "local", PlanAction::Create),
        change("c", "other", PlanAction::Update),
    ]);
    // The pre-filter counters, as the planner would have set them.
    p.to_create = 2;
    p.to_update = 1;
    p
}

#[test]
fn unknown_machine_yields_an_empty_body_and_zero_counts() {
    let mut p = two_machine_plan();
    filter_and_recount(&mut p, "ghost");
    assert!(
        p.changes.is_empty(),
        "body must be empty for an unknown machine"
    );
    assert_eq!(
        (p.to_create, p.to_update, p.to_destroy, p.unchanged),
        (0, 0, 0, 0),
        "the summary must agree with the body: an empty body beside a count \
         of 2 is one response contradicting itself (GH-214)"
    );
}

#[test]
fn a_real_machine_keeps_exactly_its_own_changes() {
    // The guard against "fixed" meaning "filters everything away".
    let mut p = two_machine_plan();
    filter_and_recount(&mut p, "local");
    assert_eq!(p.changes.len(), 2);
    assert_eq!(
        (p.to_create, p.to_update),
        (2, 0),
        "counts must match the surviving body"
    );
}

#[test]
fn counts_are_recomputed_per_action_kind() {
    let mut p = plan_of(vec![
        change("a", "m", PlanAction::Create),
        change("b", "m", PlanAction::Update),
        change("c", "m", PlanAction::Destroy),
        change("d", "m", PlanAction::NoOp),
        change("e", "z", PlanAction::Create),
    ]);
    filter_and_recount(&mut p, "m");
    assert_eq!(
        (p.to_create, p.to_update, p.to_destroy, p.unchanged),
        (1, 1, 1, 1)
    );
}
