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

use crate::core::resolver;
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
#[cfg(test)]
pub(crate) fn apply_resource_filter(
    plan: &mut ExecutionPlan,
    config: &ForjarConfig,
    resource_filter: Option<&str>,
) -> Result<(), String> {
    apply_selection_filter(plan, config, resource_filter, None)
}

/// GH-214: `plan -g <GROUP>` (FJ-281) — "Filter to resources in this group".
///
/// Matches the executor's own group predicate (`resource.resource_group`), so
/// `plan -g x` and `apply -g x` select the same set.
#[cfg(test)]
pub(crate) fn apply_group_filter(
    plan: &mut ExecutionPlan,
    config: &ForjarConfig,
    group_filter: Option<&str>,
) -> Result<(), String> {
    apply_selection_filter(plan, config, None, group_filter)
}

/// forjar#615: `-r` and `-g` select a resource set AND ITS `depends_on` CLOSURE.
///
/// `apply -r x` resolves its selection once (`apply_selection::resolve_selection`,
/// PMAT-160): the resources the positive selectors match, closed over
/// `depends_on`, so a targeted apply never executes against a prerequisite
/// that was never converged. `plan -r x` kept the exact id only. The two
/// commands computed different sets, and the plan an operator gated on did not
/// describe the apply. Measured on paiml/infra's lambda-labs, forjar 1.32.0:
/// `plan -r ollama-model-qwen35-4b` said "1 to add", and `apply -r` of the
/// same resource prompted "Apply 2 change(s) (2 create)"; the second was
/// `ollama-binary`, whose command removes `/usr/local/lib/ollama` and restarts
/// the shared daemon.
///
/// So the plan uses the resolver's rule: the positive set is the resources
/// matching EVERY supplied selector, and the kept set is its
/// `resolver::goal_closure`, the same function the apply resolver calls.
pub(crate) fn apply_selection_filter(
    plan: &mut ExecutionPlan,
    config: &ForjarConfig,
    resource: Option<&str>,
    group: Option<&str>,
) -> Result<(), String> {
    if resource.is_none() && group.is_none() {
        return Ok(());
    }
    if let Some(id) = resource {
        if !config.resources.contains_key(id) {
            let mut known: Vec<&str> = config.resources.keys().map(String::as_str).collect();
            known.sort_unstable();
            return Err(format!(
                "--resource '{id}' matches no resource in this config. Known: {}",
                known.join(", ")
            ));
        }
    }
    let in_group = |r: &crate::core::types::Resource| {
        group.is_none_or(|g| r.resource_group.as_deref() == Some(g))
    };
    if let Some(g) = group {
        if !config.resources.values().any(in_group) {
            return Err(format!("--group '{g}' matches no resource in this config"));
        }
    }
    let positive: Vec<String> = config
        .resources
        .iter()
        .filter(|(id, r)| resource.is_none_or(|want| want == id.as_str()) && in_group(r))
        .map(|(id, _)| id.clone())
        .collect();
    if positive.is_empty() {
        return Err(format!(
            "no resources match the selectors: --resource '{}', --group '{}'",
            resource.unwrap_or_default(),
            group.unwrap_or_default()
        ));
    }
    let keep = resolver::goal_closure(config, &positive)?;
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
