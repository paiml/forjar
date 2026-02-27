//! Plan command.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::apply_helpers::*;
use super::print_helpers::*;
use super::workspace::*;


#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_plan(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    _resource_filter: Option<&str>,
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
) -> Result<(), String> {
    let mut config = parse_and_validate(file)?;

    // FJ-333: Apply hypothetical param overrides
    for kv in what_if {
        if let Some((key, value)) = kv.split_once('=') {
            config.params.insert(
                key.to_string(),
                serde_yaml_ng::Value::String(value.to_string()),
            );
        } else {
            return Err(format!(
                "invalid --what-if format '{}': expected KEY=VALUE",
                kv
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
    let execution_order = resolver::build_execution_order(&config)?;

    // Load existing locks so plan shows accurate Create vs Update vs NoOp
    let locks = load_machine_locks(&config, state_dir, machine_filter)?;
    let plan = planner::plan(&config, &execution_order, &locks, tag_filter);

    if let Some(dir) = output_dir {
        export_scripts(&config, dir)?;
    }

    if json {
        print_plan_json(&plan, &config)?;
    } else {
        let show_diff = !no_diff;
        print_plan(
            &plan,
            machine_filter,
            if show_diff { Some(&config) } else { None },
        );
    }

    if cost && !plan.changes.is_empty() {
        print_plan_cost(&plan);
    }

    Ok(())
}


/// FJ-301: Serialize plan as enriched JSON with resource metadata.
fn print_plan_json(
    plan: &types::ExecutionPlan,
    config: &types::ForjarConfig,
) -> Result<(), String> {
    let changes: Vec<serde_json::Value> = plan
        .changes
        .iter()
        .map(|c| {
            let mut entry = serde_json::json!({
                "resource_id": c.resource_id,
                "machine": c.machine,
                "resource_type": c.resource_type,
                "action": c.action,
                "description": c.description,
            });
            if let Some(res) = config.resources.get(&c.resource_id) {
                if let Some(ref rg) = res.resource_group {
                    entry["resource_group"] = serde_json::json!(rg);
                }
                if !res.tags.is_empty() {
                    entry["tags"] = serde_json::json!(res.tags);
                }
                if !res.depends_on.is_empty() {
                    entry["depends_on"] = serde_json::json!(res.depends_on);
                }
            }
            entry
        })
        .collect();
    let output = serde_json::json!({
        "name": plan.name,
        "to_create": plan.to_create,
        "to_update": plan.to_update,
        "to_destroy": plan.to_destroy,
        "unchanged": plan.unchanged,
        "execution_order": plan.execution_order,
        "changes": changes,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {}", e))?
    );
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

fn print_plan_cost(plan: &types::ExecutionPlan) {
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

