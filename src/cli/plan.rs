//! Plan command.

use super::apply_helpers::*;
use super::helpers::*;
use super::helpers_state::*;
use super::print_helpers::*;
use super::workspace::*;
use crate::core::{planner, resolver, types};
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_plan(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    json: bool,
    verbose: bool,
    output_dir: Option<&Path>,
    env_file: Option<&Path>,
    workspace: Option<&str>,
    no_diff: bool,
    target: Option<&str>,
    cost: bool,
    what_if: &[String],
    plan_out: Option<&Path>,
    why: bool,
    // GH-214: `-g` printed "not yet implemented … Flag ignored" and then the
    // whole plan. It is a real filter now, so it has to reach the planner.
    group_filter: Option<&str>,
) -> Result<(), String> {
    let mut config = parse_and_validate(file)?;

    // FJ-333: Apply hypothetical param overrides
    apply_what_if_overrides(&mut config, what_if)?;
    if let Some(path) = env_file {
        load_env_params(&mut config, path)?;
    }
    inject_workspace_param(&mut config, workspace);
    resolver::resolve_data_sources(&mut config)?;

    // FJ-285: --target filters config to one resource + transitive deps
    if let Some(target_id) = target {
        let keep = collect_transitive_deps(&config, target_id)?;
        config.resources.retain(|k, _| keep.contains(k));
    }

    if verbose {
        eprintln!(
            "Planning {} ({} machines, {} resources)",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
    }
    // Load existing locks so plan shows accurate Create vs Update vs NoOp
    let locks = load_machine_locks(&config, state_dir, machine_filter)?;

    // GH-273: say WHERE state came from, and when there was none.
    super::state_visibility::report(state_dir, &config, &locks);
    // FJ-2725: phony resources are goal-only; a bulk plan must not report them
    // as perpetual changes, or `plan` never reaches "0 to change" again.
    super::apply_selection::strip_unrequested_phony(&mut config, &[]);
    let execution_order = resolver::build_execution_order(&config)?;
    let mut plan = planner::plan(&config, &execution_order, &locks, tag_filter);

    super::plan_selector::apply_machine_filter(&mut plan, machine_filter);
    // GH-214: -r and -g used to print "not yet implemented … Flag ignored"
    // followed by the whole plan, while `apply -r/-g` filtered correctly.
    super::plan_selector::apply_resource_filter(&mut plan, &config, resource_filter)?;
    super::plan_selector::apply_group_filter(&mut plan, &config, group_filter)?;
    let plan = plan;

    if let Some(dir) = output_dir {
        export_scripts(&config, dir)?;
    }

    // FJ-1250: Write plan to file for later execution
    //
    // forjar#370 asked whether producing the artifact should require operator
    // authorization too. Decided explicitly: NO, and this comment is the
    // decision, not an omission.
    //
    // A plan file is unauthenticated data. Any user can write one in a text
    // editor, so an attacker never needs `forjar plan --out` to obtain one —
    // gating production buys nothing an attacker cannot route around, while
    // costing something real: `plan` is one of the nine ReadOnly verbs
    // (`src/verb/registry.rs`), and `allowed_operators` is an apply-time gate
    // (FJ-2300). Making a read refuse for an unauthorized reader would break
    // that contract for the sake of a check with no defensive value. The gate
    // that IS load-bearing is at execution, and it now runs there —
    // `cmd_apply_from_plan` checks before it reads the plan file at all.
    if let Some(out_path) = plan_out {
        super::plan_file::save_plan_file(&plan, &config, file, out_path)?;
        println!("Plan saved to {}", out_path.display());
        return Ok(());
    }

    if why {
        // GH-214: explain only what the (possibly filtered) plan contains, so
        // `--why` cannot contradict the plan printed beside it.
        print_why_explanation(&config, &locks, &plan.execution_order, tag_filter);
    }

    // forjar#342: ONE binding, so both arms range over the same count and the
    // TTY rendering and `--json` cannot disagree about the blind spot.
    let unconsulted = super::print_helpers::unconsulted_observations(&locks);
    if json {
        super::plan_json::print_plan_json(&plan, &config, unconsulted)?;
    } else {
        print_plan(
            &plan,
            machine_filter,
            if no_diff { None } else { Some(&config) },
            unconsulted,
        );
    }

    if cost && !plan.changes.is_empty() {
        print_plan_cost(&plan);
    }

    Ok(())
}

/// FJ-333: Decides what the hypothetical params are for this run — parses each
/// `--what-if KEY=VALUE` onto the config and announces the set that was applied.
/// Rejects a pair without `=`. Lifted out of `cmd_plan` because it is the one
/// place the command validates its own argument syntax, and it is self-contained.
fn apply_what_if_overrides(
    config: &mut types::ForjarConfig,
    what_if: &[String],
) -> Result<(), String> {
    for kv in what_if {
        if let Some((key, value)) = kv.split_once('=') {
            config.params.insert(
                key.to_string(),
                serde_yaml_ng::Value::String(value.to_string()),
            );
        } else {
            return Err(format!(
                "invalid --what-if format '{kv}': expected KEY=VALUE"
            ));
        }
    }
    if !what_if.is_empty() {
        println!(
            "{}",
            dim(&format!(
                "[what-if] Hypothetical params: {}",
                what_if.join(", ")
            ))
        );
    }
    Ok(())
}

/// FJ-312: Compute and print change cost summary.
fn type_weight(t: &types::ResourceType) -> u32 {
    match t {
        types::ResourceType::Package => 3,
        types::ResourceType::Service => 3,
        types::ResourceType::Mount => 4,
        types::ResourceType::Docker | types::ResourceType::Pepita => 5,
        types::ResourceType::User => 3,
        types::ResourceType::Network => 2,
        types::ResourceType::Gpu => 4,
        types::ResourceType::Model => 5,
        types::ResourceType::Cron => 2,
        _ => 1, // file, recipe
    }
}

pub(crate) fn print_plan_cost(plan: &types::ExecutionPlan) {
    let total_cost: u32 = plan
        .changes
        .iter()
        .map(|c| type_weight(&c.resource_type))
        .sum();
    let destroy_cost: u32 = plan
        .changes
        .iter()
        .filter(|c| c.action == types::PlanAction::Destroy)
        .map(|c| type_weight(&c.resource_type) * 2) // destructive = 2x
        .sum();
    println!(
        "\nCost: {} total (create/update: {}, destroy: {})",
        total_cost + destroy_cost,
        total_cost,
        destroy_cost
    );
    if destroy_cost > 10 {
        println!(
            "  {} High destructive cost — consider --dry-run first",
            red("!")
        );
    }
}

/// FJ-344: Compact one-line-per-resource plan output.
pub(crate) fn cmd_plan_compact(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let execution_order = resolver::build_execution_order(&config)?;
    let locks = load_machine_locks(&config, state_dir, machine_filter)?;
    let plan = planner::plan(&config, &execution_order, &locks, None);

    if json {
        let compact: Vec<serde_json::Value> = plan
            .changes
            .iter()
            .map(|c| {
                serde_json::json!({
                    "resource": c.resource_id,
                    "action": format!("{:?}", c.action),
                    "machine": c.machine,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&compact).unwrap_or_default()
        );
    } else {
        for change in &plan.changes {
            let icon = match change.action {
                types::PlanAction::Create => green("+"),
                types::PlanAction::Update => yellow("~"),
                types::PlanAction::Destroy => red("-"),
                types::PlanAction::NoOp => dim("="),
            };
            println!("  {} {} ({})", icon, change.resource_id, change.machine,);
        }
        println!(
            "\n{} change(s)",
            plan.changes
                .iter()
                .filter(|c| c.action != types::PlanAction::NoOp)
                .count()
        );
    }

    Ok(())
}

/// FJ-1379: Print per-resource --why explanation.
fn print_why_explanation(
    config: &types::ForjarConfig,
    locks: &std::collections::HashMap<String, types::StateLock>,
    execution_order: &[String],
    tag_filter: Option<&str>,
) {
    println!("\n{}", bold("Change Explanations (--why):"));
    let reasons = collect_why_reasons(config, locks, execution_order, tag_filter);
    for reason in &reasons {
        let icon = action_icon(&reason.action);
        println!("  {} {} on {}", icon, reason.resource_id, reason.machine);
        for r in &reason.reasons {
            println!("    {}", dim(&format!("- {r}")));
        }
    }
    println!();
}

/// Collect non-noop change reasons for all matching resources.
fn collect_why_reasons(
    config: &types::ForjarConfig,
    locks: &std::collections::HashMap<String, types::StateLock>,
    execution_order: &[String],
    tag_filter: Option<&str>,
) -> Vec<crate::core::planner::why::ChangeReason> {
    use crate::core::planner::why;
    let mut results = Vec::new();
    for resource_id in execution_order {
        let Some(resource) = config.resources.get(resource_id) else {
            continue;
        };
        if let Some(tag) = tag_filter {
            if !resource.tags.iter().any(|t| t == tag) {
                continue;
            }
        }
        // GH-212: explain the RESOLVED resource. Comparing the raw config
        // against a lock that stores resolved values produced nonsense like
        // "path changed: /tmp/x/a.txt -> {{params.sandbox}}/a.txt".
        let resolved = crate::core::resolver::resolve_or_fallback(
            resource_id,
            resource,
            &config.params,
            &config.machines,
            &config.secrets,
        );
        for machine_name in resource.machine.iter() {
            let reason = why::explain_why(resource_id, &resolved, machine_name, locks);
            if reason.action != types::PlanAction::NoOp {
                results.push(reason);
            }
        }
    }
    results
}

/// Action icon for display.
fn action_icon(action: &types::PlanAction) -> String {
    match action {
        types::PlanAction::Create => green("+"),
        types::PlanAction::Update => yellow("~"),
        types::PlanAction::Destroy => red("-"),
        types::PlanAction::NoOp => dim("="),
    }
}

#[cfg(test)]
#[path = "plan_tests_selector_scope.rs"]
mod tests_selector_scope;
