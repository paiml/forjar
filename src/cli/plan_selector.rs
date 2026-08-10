//! GH-214: selector scope for `plan`.
//!
//! A selector must apply to EVERY part of a response. `machine_filter` was handed
//! to `print_plan` only, so the text summary and `--json` were computed from the
//! UNFILTERED plan:
//!
//! ```text
//!   $ forjar plan -m ghost          # a machine that does not exist
//!   Plan: 2 to add, ...             # empty body, count of 2
//!   $ forjar plan -m ghost --json
//!   to_create = 2
//! ```
//!
//! — one response contradicting itself. Filtering the plan itself makes body,
//! summary and JSON agree by construction rather than by three call sites each
//! remembering to.

use crate::core::types::{ExecutionPlan, PlanAction};

/// Restrict a plan to one machine, keeping every part of it consistent.
///
/// The summary line and `--json` read PRECOMPUTED counters, not
/// `plan.changes.len()`, so retaining the vector alone leaves "2 to add" sitting
/// next to an empty body. The counters are recomputed from what survives.
pub(crate) fn apply_machine_filter(plan: &mut ExecutionPlan, machine_filter: Option<&str>) {
    let Some(m) = machine_filter else { return };
    plan.changes.retain(|c| c.machine == m);
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
