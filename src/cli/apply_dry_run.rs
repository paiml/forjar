//! GH-210: the `--dry-run` / `make -n` body.
//!
//! Split out of `apply_output.rs` to keep that file under the 500-line
//! quality gate.
//!
//! PMAT-160: the body is SCOPED by the selectors the executor honours.
//! `apply --dry-run -r stack-tool-forjar` printed every resource in the config
//! as "would execute" — all 139 of one fleet manifest, "2 to add, 30 to change"
//! on another for a one-resource `-r` — while the executor
//! (`executor::resource_ops`) skipped everything but the named id and the
//! confirmation prompt (`apply_gates::scoped_action_counts`) already counted
//! only it. `plan -r` scoped correctly, through `plan_selector`. Measured
//! 2026-09-05 against 1.24.0 and read unchanged in 1.25.2. A dry run that
//! reports work the apply would not do fails in the worst direction for the
//! flag that exists to preview the blast radius, so the plan is narrowed HERE,
//! once, with the `plan_selector` filters `plan` already uses, and the text
//! body, its summary line and `--json` are all rendered from that one plan —
//! agreement by construction rather than by three call sites each remembering
//! to (the GH-214 argument, applied to `apply`).

use super::apply_drift::GateScope;
use super::helpers::*;
use super::helpers_state::*;
use super::plan_selector;
use crate::core::{planner, resolver, types};
use std::path::Path;

/// The plan a dry run would execute, under every selector the apply honours.
///
/// `scope` is the same [`GateScope`] the run hands to the drift gate and the
/// ControlMaster opener — one value, so the dry run cannot be narrower or
/// wider than the gate it precedes. `tag` is applied by the planner itself;
/// machine, resource and group are applied to the finished plan with the
/// predicates `plan -m/-r/-g` use, so `apply --dry-run -r x` and `plan -r x`
/// select the same set.
pub(super) fn scoped_dry_run_plan(
    config: &types::ForjarConfig,
    state_dir: &Path,
    scope: &GateScope<'_>,
) -> Result<types::ExecutionPlan, String> {
    let execution_order = resolver::build_execution_order(config)?;
    let plan_locks = load_machine_locks(config, state_dir, scope.machine)?;
    let mut plan = planner::plan(config, &execution_order, &plan_locks, scope.tag);
    scope_plan(&mut plan, config, scope)?;
    Ok(plan)
}

/// PMAT-160: narrow a plan to what `-m`, `-r` and `-g` select.
///
/// The pure half of [`scoped_dry_run_plan`], so the scoping can be asserted
/// against a fixture plan with no state dir. A `-r` or `-g` naming nothing is
/// an error, not an empty success — the house rule from FJ-2723 and GH-214,
/// and `reject_empty_selection` upstream of the dry run says the same.
pub(super) fn scope_plan(
    plan: &mut types::ExecutionPlan,
    config: &types::ForjarConfig,
    scope: &GateScope<'_>,
) -> Result<(), String> {
    plan_selector::apply_machine_filter(plan, scope.machine);
    plan_selector::apply_resource_filter(plan, config, scope.resource)?;
    plan_selector::apply_group_filter(plan, config, scope.group)
}

/// GH-210: show what WOULD run.
///
/// `apply --dry-run` and `make -n` printed exactly one line — "Dry run — no
/// changes applied." — and nothing else, though `--dry-run` is documented as
/// "Show what would be executed without running" and `-n` as "Print what would
/// run without running it (make -n)". Neither showed a single action, so the
/// flag was indistinguishable from a no-op and answered none of the question it
/// exists for. `--dry-run-graph` on the same config already printed the three
/// resources, which is what made this a defect rather than a design choice.
///
/// The dry-run body, as text, so it can be asserted without capturing stdout.
pub(super) fn render_dry_run_actions(plan: &types::ExecutionPlan) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "{}", bold("Dry run — would execute:"));
    for change in &plan.changes {
        let icon = match change.action {
            types::PlanAction::Create => green("+"),
            types::PlanAction::Update => yellow("~"),
            types::PlanAction::Destroy => red("-"),
            types::PlanAction::NoOp => dim("="),
        };
        let _ = writeln!(
            out,
            "  {} {} on {}: {}",
            icon, change.resource_id, change.machine, change.description
        );
    }
    if plan.changes.is_empty() {
        let _ = writeln!(out, "  {}", dim("(nothing selected)"));
    }
    let _ = writeln!(
        out,
        "\n{} to add, {} to change, {} to destroy, {} unchanged. No changes applied.",
        plan.to_create, plan.to_update, plan.to_destroy, plan.unchanged
    );
    out
}

/// The `--json` dry-run body, rendered from the SAME scoped plan as the text
/// body, so the two cannot disagree about what would run.
pub(super) fn render_dry_run_json(plan: &types::ExecutionPlan) -> serde_json::Value {
    let changes: Vec<serde_json::Value> = plan
        .changes
        .iter()
        .map(|c| {
            serde_json::json!({
                "resource": c.resource_id,
                "machine": c.machine,
                "type": c.resource_type.to_string(),
                "action": format!("{:?}", c.action).to_lowercase(),
                "description": c.description,
            })
        })
        .collect();
    serde_json::json!({
        "dry_run": true,
        "name": plan.name,
        "to_create": plan.to_create,
        "to_update": plan.to_update,
        "to_destroy": plan.to_destroy,
        "unchanged": plan.unchanged,
        "changes": changes,
    })
}
