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
/// The machine filter and the resource, group and tag selectors are applied to
/// the plan afterwards so dependency order survives; the three positive
/// selectors are one set closed over `depends_on`, as `apply` resolves it. `-r` and `-g` are
/// validated against the config: a selector that matches no resource is an
/// error, not an empty successful plan.
pub(crate) fn plan_filtered(
    config: &ForjarConfig,
    locks: &HashMap<String, StateLock>,
    selectors: &PlanSelectors,
) -> Result<ExecutionPlan, String> {
    let execution_order = resolver::build_execution_order(config)?;
    // PMAT-890: `-t` is NOT given to the planner. The planner's tag filter
    // drops untagged prerequisites; `apply -t` keeps them (closure over
    // `depends_on`), so the tag is resolved below with `-r` and `-g`.
    let mut plan = planner::plan(config, &execution_order, locks, None);
    plan_selector::apply_machine_filter(&mut plan, selectors.machine.as_deref());
    // forjar#615: ONE positive set, closed over depends_on, as apply resolves it.
    plan_selector::apply_selection_filter(
        &mut plan,
        config,
        selectors.resource.as_deref(),
        selectors.group.as_deref(),
        selectors.tag.as_deref(),
    )?;
    Ok(plan)
}
