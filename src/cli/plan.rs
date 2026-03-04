//! Plan command.

use super::apply_helpers::*;
use super::helpers::*;
use super::helpers_state::*;
use super::print_helpers::*;
use super::workspace::*;
use crate::core::{planner, resolver, types};
use crate::tripwire::hasher;
use std::path::Path;

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
    plan_out: Option<&Path>,
    why: bool,
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

    // FJ-1250: Write plan to file for later execution
    if let Some(out_path) = plan_out {
        save_plan_file(&plan, &config, file, out_path)?;
        println!("Plan saved to {}", out_path.display());
        return Ok(());
    }

    if why {
        print_why_explanation(&config, &locks, &execution_order, tag_filter);
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
        serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {e}"))?
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

/// FJ-1250: Save an execution plan to a JSON file with config integrity hash.
pub(crate) fn save_plan_file(
    plan: &types::ExecutionPlan,
    config: &types::ForjarConfig,
    config_path: &Path,
    out_path: &Path,
) -> Result<(), String> {
    let config_yaml =
        serde_yaml_ng::to_string(config).map_err(|e| format!("serialize config: {e}"))?;
    let config_hash = hasher::hash_string(&config_yaml);

    let changes: Vec<serde_json::Value> = plan
        .changes
        .iter()
        .map(|c| {
            serde_json::json!({
                "resource_id": c.resource_id,
                "machine": c.machine,
                "resource_type": c.resource_type,
                "action": c.action,
                "description": c.description,
            })
        })
        .collect();

    let output = serde_json::json!({
        "format": "forjar-plan-v1",
        "config_file": config_path.display().to_string(),
        "config_hash": config_hash,
        "name": plan.name,
        "to_create": plan.to_create,
        "to_update": plan.to_update,
        "to_destroy": plan.to_destroy,
        "unchanged": plan.unchanged,
        "execution_order": plan.execution_order,
        "changes": changes,
    });

    let json = serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {e}"))?;
    std::fs::write(out_path, json).map_err(|e| format!("write plan file: {e}"))?;
    Ok(())
}

/// FJ-1250: Load a saved plan file, validate config hash, and return the plan.
pub(crate) fn load_plan_file(
    plan_path: &Path,
    config: &types::ForjarConfig,
) -> Result<types::ExecutionPlan, String> {
    let content = std::fs::read_to_string(plan_path).map_err(|e| format!("read plan file: {e}"))?;
    let doc: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("parse plan file: {e}"))?;

    let format = doc.get("format").and_then(|v| v.as_str()).unwrap_or("");
    if format != "forjar-plan-v1" {
        return Err(format!("unsupported plan format: '{format}'"));
    }

    let stored_hash = doc
        .get("config_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let config_yaml =
        serde_yaml_ng::to_string(config).map_err(|e| format!("serialize config: {e}"))?;
    let current_hash = hasher::hash_string(&config_yaml);
    if stored_hash != current_hash {
        return Err(
            "config has changed since plan was created — re-run `forjar plan` to regenerate"
                .to_string(),
        );
    }

    let changes_arr = doc
        .get("changes")
        .and_then(|v| v.as_array())
        .ok_or("plan file missing 'changes' array")?;
    let mut changes = Vec::new();
    for entry in changes_arr {
        let action_str = entry
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("no_op");
        let action = match action_str {
            "create" => types::PlanAction::Create,
            "update" => types::PlanAction::Update,
            "destroy" => types::PlanAction::Destroy,
            _ => types::PlanAction::NoOp,
        };
        let rt_str = entry
            .get("resource_type")
            .and_then(|v| v.as_str())
            .unwrap_or("file");
        let resource_type = match rt_str {
            "package" => types::ResourceType::Package,
            "service" => types::ResourceType::Service,
            "mount" => types::ResourceType::Mount,
            "user" => types::ResourceType::User,
            "docker" => types::ResourceType::Docker,
            "pepita" => types::ResourceType::Pepita,
            "network" => types::ResourceType::Network,
            "cron" => types::ResourceType::Cron,
            "recipe" => types::ResourceType::Recipe,
            "model" => types::ResourceType::Model,
            "gpu" => types::ResourceType::Gpu,
            _ => types::ResourceType::File,
        };
        changes.push(types::PlannedChange {
            resource_id: entry
                .get("resource_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            machine: entry
                .get("machine")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            resource_type,
            action,
            description: entry
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }

    let execution_order: Vec<String> = doc
        .get("execution_order")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    Ok(types::ExecutionPlan {
        name: doc
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        changes,
        execution_order,
        to_create: doc.get("to_create").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        to_update: doc.get("to_update").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        to_destroy: doc.get("to_destroy").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        unchanged: doc.get("unchanged").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
    })
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
        for machine_name in resource.machine.to_vec() {
            let reason = why::explain_why(resource_id, resource, &machine_name, locks);
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
