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

/// GH-214: `plan -r <RESOURCE>` — "Target specific resource".
///
/// Shipped as `Warning: --resource filter is not yet implemented for plan.
/// Flag ignored.` followed by the WHOLE plan, while `apply -r` on the same
/// config filtered correctly. Filtering the plan (rather than the config) keeps
/// the dependency order intact and makes body, summary and `--json` agree by
/// construction.
///
/// A selector that matches nothing is an error, following the same house rule
/// as the apply scope selectors: `plan -r a-fil` must not print an empty,
/// successful plan for `a-file`.
pub(crate) fn apply_resource_filter(
    plan: &mut ExecutionPlan,
    config: &ForjarConfig,
    resource_filter: Option<&str>,
) -> Result<(), String> {
    let Some(id) = resource_filter else {
        return Ok(());
    };
    if !config.resources.contains_key(id) {
        let mut known: Vec<&str> = config.resources.keys().map(String::as_str).collect();
        known.sort_unstable();
        return Err(format!(
            "--resource '{id}' matches no resource in this config. Known: {}",
            known.join(", ")
        ));
    }
    plan.changes.retain(|c| c.resource_id == id);
    plan.execution_order.retain(|r| r == id);
    recount(plan);
    Ok(())
}

/// GH-214: `plan -g <GROUP>` (FJ-281) — "Filter to resources in this group".
///
/// Matches the executor's own group predicate (`resource.resource_group`), so
/// `plan -g x` and `apply -g x` select the same set.
pub(crate) fn apply_group_filter(
    plan: &mut ExecutionPlan,
    config: &ForjarConfig,
    group_filter: Option<&str>,
) -> Result<(), String> {
    let Some(group) = group_filter else {
        return Ok(());
    };
    let in_group = |id: &String| {
        config
            .resources
            .get(id)
            .and_then(|r| r.resource_group.as_deref())
            == Some(group)
    };
    if !config.resources.keys().any(in_group) {
        return Err(format!(
            "--group '{group}' matches no resource in this config"
        ));
    }
    plan.changes.retain(|c| in_group(&c.resource_id));
    plan.execution_order.retain(in_group);
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
