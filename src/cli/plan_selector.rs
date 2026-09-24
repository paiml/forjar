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

use super::apply_selection::selected_ids;
use crate::core::types::{ExecutionPlan, ForjarConfig, PlanAction};

/// Recompute the action counters from whatever survived a filter.
///
/// The summary line and `--json` read these PRECOMPUTED fields, not
/// `plan.changes.len()`, so a filter that retains the vector alone leaves
/// "2 to add" sitting next to an empty body.
fn recount(plan: &mut ExecutionPlan) {
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

/// GH-214 / #615: `plan -r <RESOURCE>` and `plan -g <GROUP>`.
///
/// GH-214 made them filter at all (they shipped as "not yet implemented. Flag
/// ignored."). But each kept its own predicate — exact id, exact group — and
/// FJ-331 (#468) then gave `apply` the `depends_on` closure, so on
/// `leaf-file → base-dir` `plan -r leaf-file` said "1 to add" and
/// `apply -r leaf-file` converged two (#615). The set now comes from
/// [`selected_ids`], which IS apply's resolver, so the plan an operator reviews
/// is the set the apply converges — by construction, not by two predicates
/// agreeing.
///
/// Filtering the plan (rather than the config) keeps the dependency order
/// intact and makes body, summary and `--json` agree. A selector that matches
/// nothing is an error (FJ-2723): `plan -r a-fil` must not print an empty,
/// successful plan for `a-file`.
pub(crate) fn apply_selection_filter(
    plan: &mut ExecutionPlan,
    config: &ForjarConfig,
    resource: Option<&str>,
    group: Option<&str>,
) -> Result<(), String> {
    let Some(keep) = selected_ids(config, resource, group)? else {
        return Ok(());
    };
    plan.changes.retain(|c| keep.contains(&c.resource_id));
    plan.execution_order.retain(|r| keep.contains(r));
    recount(plan);
    Ok(())
}

/// Restrict a plan to one machine, keeping every part of it consistent.
///
/// The summary line and `--json` read PRECOMPUTED counters, not
/// `plan.changes.len()`, so retaining the vector alone leaves "2 to add" sitting
/// next to an empty body. The counters are recomputed from what survives.
pub(crate) fn apply_machine_filter(plan: &mut ExecutionPlan, machine_filter: Option<&str>) {
    let Some(m) = machine_filter else { return };
    plan.changes.retain(|c| c.machine == m);
    recount(plan);
}
