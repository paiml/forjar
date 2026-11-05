//! The output half of `forjar plan` (Refs forjar#853): split out of `plan.rs`
//! so `cmd_plan_with_pins` stays under the complexity gate and the file under
//! 500 lines. Moved whole; no logic changed.

use super::plan::{print_plan_cost, PlanVersionPins};
use super::print_helpers::*;
use crate::core::types;

/// How the plan is rendered; grouped so `render_plan` stays under the argument
/// count (Refs forjar#853).
pub(super) struct PlanRender<'a> {
    pub(super) json: bool,
    pub(super) no_diff: bool,
    pub(super) cost: bool,
    pub(super) machine_filter: Option<&'a str>,
    pub(super) version_pins: PlanVersionPins,
}

/// The output half of `cmd_plan_with_pins`, moved out whole so the caller stays
/// under the complexity gate (Refs forjar#853). No logic changed.
pub(super) fn render_plan(
    plan: &types::ExecutionPlan,
    config: &types::ForjarConfig,
    locks: &std::collections::HashMap<String, types::StateLock>,
    opts: &PlanRender<'_>,
    refreshed: Option<&super::plan_refresh::RefreshOutcome>,
) -> Result<(), String> {
    // forjar#342: ONE binding, so both arms range over the same count and the
    // TTY rendering and `--json` cannot disagree about the blind spot.
    // forjar#415: under `--refresh` the blind spot is what the refresh did NOT
    // measure, and the lock-relative sentence (printed for a nonzero count)
    // would be false, so it gets 0 and the refreshed disclosure follows.
    let pin_rows = super::plan_version_pins::measure(config, plan, opts.version_pins);
    let unconsulted = match refreshed {
        Some(r) => r.unconsulted,
        None => super::print_helpers::unconsulted_observations(locks),
    };
    if opts.json {
        super::plan_json::print_plan_json(
            plan,
            config,
            unconsulted,
            refreshed,
            pin_rows.as_deref().map(super::plan_version_pins::to_json),
        )?;
    } else {
        print_plan_text(
            plan,
            config,
            opts,
            refreshed,
            unconsulted,
            pin_rows.as_deref(),
        );
    }

    if opts.cost && !plan.changes.is_empty() {
        print_plan_cost(plan);
    }

    Ok(())
}

fn print_plan_text(
    plan: &types::ExecutionPlan,
    config: &types::ForjarConfig,
    opts: &PlanRender<'_>,
    refreshed: Option<&super::plan_refresh::RefreshOutcome>,
    unconsulted: usize,
    pin_rows: Option<&[super::plan_version_pins::PinRow]>,
) {
    print_plan(
        plan,
        opts.machine_filter,
        if opts.no_diff { None } else { Some(config) },
        if refreshed.is_some() { 0 } else { unconsulted },
    );
    print_refresh_footer(refreshed);
    if let Some(rows) = pin_rows {
        super::plan_version_pins::print_text(rows);
    }
}

/// The two `--refresh` lines that follow the text plan.
fn print_refresh_footer(refreshed: Option<&super::plan_refresh::RefreshOutcome>) {
    if let Some(msg) = refreshed.and_then(super::plan_refresh::refreshed_scope_disclosure) {
        println!("\n{msg}");
    }
    if let Some(r) = refreshed {
        println!(
            "Refreshed {} machine(s): {} resource(s) measured as drifted.",
            r.machines, r.drifted
        );
    }
}
