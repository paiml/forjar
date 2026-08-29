//! Refs #358 — the ONE composition of "the planner, plus the four selectors".
//!
//! `forjar plan` filtered its plan in five statements (`planner::plan` with the
//! tag filter, then the machine, resource and group filters from
//! [`super::plan_selector`]). `apply --plan-file` now has to reproduce the plan a
//! saved document claims to be, in order to compare against it — and a SECOND
//! spelling of those five statements is how the two drift apart, which would
//! turn the comparison into a permanent false alarm on filtered plans.
//!
//! So there is one function and both callers use it. That is the same rule the
//! ambient-inputs work (#244) settled on for `hash_declared_inputs`: two
//! compositions of the same idea is how you get an eternal "it changed" pump.

use super::plan_selector;
use crate::core::plan_selectors::PlanSelectors;
use crate::core::types::{ExecutionPlan, ForjarConfig, StateLock};
use crate::core::{planner, resolver};
use std::collections::HashMap;

/// Plan `config` against `locks`, restricted by `selectors`.
///
/// The tag filter goes to the planner (that is where it has always been
/// applied, and the executor does the same), and the other three are applied to
/// the plan afterwards so dependency order survives. `-r` and `-g` are
/// validated against the config: a selector that matches no resource is an
/// error, not an empty successful plan.
pub(crate) fn plan_filtered(
    config: &ForjarConfig,
    locks: &HashMap<String, StateLock>,
    selectors: &PlanSelectors,
) -> Result<ExecutionPlan, String> {
    let execution_order = resolver::build_execution_order(config)?;
    let mut plan = planner::plan(config, &execution_order, locks, selectors.tag.as_deref());
    plan_selector::apply_machine_filter(&mut plan, selectors.machine.as_deref());
    plan_selector::apply_resource_filter(&mut plan, config, selectors.resource.as_deref())?;
    plan_selector::apply_group_filter(&mut plan, config, selectors.group.as_deref())?;
    Ok(plan)
}
