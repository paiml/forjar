//! FJ-1250: the saved plan file — `forjar plan --out` writes it,
//! `forjar apply --plan-file` reads it back.
//!
//! Extracted from `plan.rs`, which was at 514 lines against a 500-line gate.

use crate::core::types;
use std::path::Path;

/// FJ-1250: Save an execution plan to a JSON file with config integrity hash.
pub(crate) fn save_plan_file(
    plan: &types::ExecutionPlan,
    config: &types::ForjarConfig,
    config_path: &Path,
    out_path: &Path,
) -> Result<(), String> {
    // GH-212: canonical (sorted-map) hash — the plain serialisation varied per
    // process, so `apply --plan-file` rejected plans nobody had touched.
    let config_hash = crate::core::config_hash::config_hash(config)?;

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

/// A plan-file string field, or `default` when absent or not a string.
fn plan_str<'a>(entry: &'a serde_json::Value, key: &str, default: &'a str) -> &'a str {
    entry.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}

/// A plan-file unsigned field, or 0 when absent or not a number.
fn plan_u32(doc: &serde_json::Value, key: &str) -> u32 {
    doc.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as u32
}

/// A plan-file array-of-strings field, or empty when absent.
fn plan_str_array(doc: &serde_json::Value, key: &str) -> Vec<String> {
    doc.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn plan_action_from_str(action_str: &str) -> types::PlanAction {
    match action_str {
        "create" => types::PlanAction::Create,
        "update" => types::PlanAction::Update,
        "destroy" => types::PlanAction::Destroy,
        _ => types::PlanAction::NoOp,
    }
}

fn plan_resource_type_from_str(rt_str: &str) -> types::ResourceType {
    match rt_str {
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
    }
}

fn planned_change_from_entry(entry: &serde_json::Value) -> types::PlannedChange {
    let action = plan_action_from_str(plan_str(entry, "action", "no_op"));
    let resource_type = plan_resource_type_from_str(plan_str(entry, "resource_type", "file"));
    types::PlannedChange {
        resource_id: plan_str(entry, "resource_id", "").to_string(),
        machine: plan_str(entry, "machine", "").to_string(),
        resource_type,
        action,
        description: plan_str(entry, "description", "").to_string(),
    }
}

/// Reject a plan whose format tag is unknown or whose config hash no longer
/// matches the config being applied.
fn check_plan_provenance(
    doc: &serde_json::Value,
    config: &types::ForjarConfig,
) -> Result<(), String> {
    let format = plan_str(doc, "format", "");
    if format != "forjar-plan-v1" {
        return Err(format!("unsupported plan format: '{format}'"));
    }

    let stored_hash = plan_str(doc, "config_hash", "");
    let current_hash = crate::core::config_hash::config_hash(config)?;
    if stored_hash != current_hash {
        return Err(
            "config has changed since plan was created — re-run `forjar plan` to regenerate"
                .to_string(),
        );
    }
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

    check_plan_provenance(&doc, config)?;

    let changes_arr = doc
        .get("changes")
        .and_then(|v| v.as_array())
        .ok_or("plan file missing 'changes' array")?;
    let changes: Vec<types::PlannedChange> =
        changes_arr.iter().map(planned_change_from_entry).collect();

    Ok(types::ExecutionPlan {
        name: plan_str(&doc, "name", "").to_string(),
        changes,
        execution_order: plan_str_array(&doc, "execution_order"),
        to_create: plan_u32(&doc, "to_create"),
        to_update: plan_u32(&doc, "to_update"),
        to_destroy: plan_u32(&doc, "to_destroy"),
        unchanged: plan_u32(&doc, "unchanged"),
    })
}
