//! GH-210: the `--dry-run` / `make -n` body.
//!
//! Split out of `apply_output.rs` to keep that file under the 500-line
//! quality gate.

use super::helpers::*;
use super::helpers_state::*;
use crate::core::{planner, resolver, types};
use std::path::Path;

/// GH-210: show what WOULD run.
///
/// `apply --dry-run` and `make -n` printed exactly one line — "Dry run — no
/// changes applied." — and nothing else, though `--dry-run` is documented as
/// "Show what would be executed without running" and `-n` as "Print what would
/// run without running it (make -n)". Neither showed a single action, so the
/// flag was indistinguishable from a no-op and answered none of the question it
/// exists for. `--dry-run-graph` on the same config already printed the three
/// resources, which is what made this a defect rather than a design choice.
pub(super) fn print_dry_run_actions(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machine_filter: Option<&str>,
    tag_filter: Option<&str>,
) -> Result<(), String> {
    let execution_order = resolver::build_execution_order(config)?;
    let plan_locks = load_machine_locks(config, state_dir, machine_filter)?;
    let mut plan = planner::plan(config, &execution_order, &plan_locks, tag_filter);
    super::plan_selector::apply_machine_filter(&mut plan, machine_filter);
    print!("{}", render_dry_run_actions(&plan));
    Ok(())
}

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
