//! FJ-1250: the saved plan file — `forjar plan --out` writes it,
//! `forjar apply --plan-file` reads it back.
//!
//! Extracted from `plan.rs`, which was at 514 lines against a 500-line gate.
//!
//! # Refs #356 / #358 — v1 verified the config and nothing else
//!
//! A `forjar-plan-v1` document carried a `config_hash` and, underneath it,
//! `changes` / `execution_order` / three counters as plain unauthenticated
//! JSON. `apply --plan-file` checked the hash and then read the body as if it
//! were trustworthy, so editing three integers — leaving `config_hash`
//! byte-identical — made a requested apply print "Plan has no changes to
//! apply." and exit 0 having converged nothing.
//!
//! `forjar-plan-v2` adds a `seal` object binding the config, the state locks
//! the planner READ, and the body itself. See `core::plan_seal` for what that
//! does and does not prove.
//!
//! It also carries a `selectors` object — the `-m`/`-r`/`-t`/`-g` the plan was
//! produced under — sealed with the body. `apply --plan-file` re-plans under it
//! to check what the document claims, and without it a legitimate `plan -r X
//! --out` over a converged X is byte-identical to an honest whole-stack plan
//! with the pending lines deleted out of it.
//!
//! v1 documents still load, with a warning: their config check is real, and
//! refusing them outright would strand plans written by an installed binary.
//! There is no silent downgrade in the other direction — a v2 document whose
//! seal does not verify is an error, never a fallback to v1 checking.

use crate::core::plan_seal::{self, PlanSeal};
use crate::core::plan_selectors::PlanSelectors;
use crate::core::types;
use std::path::Path;

/// The original, config-hash-only plan document.
pub(crate) const FORMAT_V1: &str = "forjar-plan-v1";
/// The sealed plan document.
pub(crate) const FORMAT_V2: &str = "forjar-plan-v2";

/// A plan file that passed its integrity checks.
#[derive(Debug)]
pub(crate) struct LoadedPlan {
    /// The plan body, as it will be executed.
    pub plan: types::ExecutionPlan,
    /// True when the body itself was sealed (v2), false for a v1 document
    /// where only the config was verified.
    ///
    /// The caller needs this: "this plan has no changes" is a legitimate
    /// instruction from a sealed plan and an unauthenticated one from a v1
    /// document, and acting on the second is how a requested apply exits 0
    /// having done nothing.
    pub sealed: bool,
    /// Refs #358: the filters this plan was produced under.
    ///
    /// The caller re-plans under exactly these to decide whether the body is
    /// still true. A v1 document has no record and reads back unfiltered, which
    /// is the strictest reading available for it.
    pub selectors: PlanSelectors,
}

/// FJ-1250: Save an execution plan to a JSON file, sealed against the config,
/// the state it was planned from, and its own body.
///
/// The seal carries no wall-clock expiry (`ttl_secs: 0`). A plan file routinely
/// crosses CI stages, and forjar has no trusted clock — the state leg already
/// invalidates a plan the moment the world it reasoned about moves, which is a
/// far better staleness signal than an age in seconds.
pub(crate) fn save_plan_file(
    plan: &types::ExecutionPlan,
    selectors: &PlanSelectors,
    config: &types::ForjarConfig,
    config_path: &Path,
    state_dir: &Path,
    out_path: &Path,
) -> Result<(), String> {
    let sealed = plan_seal::seal(plan, selectors, config, state_dir, None)?;

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
        "format": FORMAT_V2,
        "config_file": config_path.display().to_string(),
        // GH-212: canonical (sorted-map) hash — the plain serialisation varied
        // per process, so `apply --plan-file` rejected plans nobody had
        // touched. Kept at its v1 key and value; the seal carries the same
        // string and the two are checked against each other on load.
        "config_hash": sealed.config_hash,
        "name": plan.name,
        "to_create": plan.to_create,
        "to_update": plan.to_update,
        "to_destroy": plan.to_destroy,
        "unchanged": plan.unchanged,
        "execution_order": plan.execution_order,
        "changes": changes,
        // Refs #358: what this plan was filtered by, so `apply --plan-file` can
        // recompute the plan it claims to be rather than guessing.
        "selectors": selectors,
        "seal": sealed,
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

/// Reconstruct the plan body from the document.
///
/// The RECONSTRUCTED body is what gets sealed and what gets executed, so the
/// diff leg is computed over exactly the value the executor will act on — a
/// field the reader normalises cannot smuggle a difference past the hash.
fn plan_body_from_doc(doc: &serde_json::Value) -> Result<types::ExecutionPlan, String> {
    let changes_arr = doc
        .get("changes")
        .and_then(|v| v.as_array())
        .ok_or("plan file missing 'changes' array")?;
    Ok(types::ExecutionPlan {
        name: plan_str(doc, "name", "").to_string(),
        changes: changes_arr.iter().map(planned_change_from_entry).collect(),
        execution_order: plan_str_array(doc, "execution_order"),
        to_create: plan_u32(doc, "to_create"),
        to_update: plan_u32(doc, "to_update"),
        to_destroy: plan_u32(doc, "to_destroy"),
        unchanged: plan_u32(doc, "unchanged"),
    })
}

/// Read the selector record back.
///
/// An ABSENT `selectors` key reads as the unfiltered record, which is both the
/// v1 case and the strictest reading: a document that does not claim to be
/// narrow is held to the whole config. It is not a silent default for v2 — the
/// record is inside the diff leg, so a v2 document that omits it verifies only
/// if it was sealed unfiltered.
///
/// A `selectors` value that is not a valid record is an error rather than a
/// fallback to unfiltered: falling back would run the comparison under filters
/// the document did not ask for.
fn selectors_from_doc(doc: &serde_json::Value) -> Result<PlanSelectors, String> {
    let Some(raw) = doc.get("selectors") else {
        return Ok(PlanSelectors::default());
    };
    serde_json::from_value(raw.clone())
        .map_err(|e| format!("PLAN_MALFORMED: unreadable plan selectors: {e}"))
}

/// Reject a plan whose config hash no longer matches the config being applied.
fn check_config_hash(doc: &serde_json::Value, config: &types::ForjarConfig) -> Result<(), String> {
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

/// v1: the config is verified; the body is not. Load it, say so, and still
/// refuse a body whose counters contradict its own change list.
fn check_v1(
    doc: &serde_json::Value,
    plan: &types::ExecutionPlan,
    config: &types::ForjarConfig,
) -> Result<(), String> {
    eprintln!(
        "warning: '{FORMAT_V1}' plan file — only the config is verified. The state it was \
         planned against and the plan body itself are unsealed. Re-run `forjar plan --out` \
         to write a sealed '{FORMAT_V2}' plan."
    );
    check_config_hash(doc, config)?;
    plan_seal::check_body_partition(plan).map_err(|e| e.to_string())
}

/// v2: recompute all three legs from live inputs and compare with the seal.
fn check_v2(
    doc: &serde_json::Value,
    plan: &types::ExecutionPlan,
    selectors: &PlanSelectors,
    config: &types::ForjarConfig,
    state_dir: &Path,
) -> Result<(), String> {
    let raw = doc
        .get("seal")
        .ok_or_else(|| format!("PLAN_MALFORMED: '{FORMAT_V2}' plan file has no 'seal'"))?;
    let sealed: PlanSeal = serde_json::from_value(raw.clone())
        .map_err(|e| format!("PLAN_MALFORMED: unreadable plan seal: {e}"))?;
    if plan_str(doc, "config_hash", "") != sealed.config_hash {
        return Err(
            "PLAN_MALFORMED: the plan's config_hash disagrees with its own seal".to_string(),
        );
    }
    plan_seal::verify(&sealed, plan, selectors, config, state_dir).map_err(|e| e.to_string())
}

/// FJ-1250: Load a saved plan file and verify it against the live world.
///
/// The format tag is checked BEFORE the body is read, so an unrecognised
/// document is reported as an unsupported format rather than as a missing
/// field it was never going to have.
pub(crate) fn load_plan_file(
    plan_path: &Path,
    config: &types::ForjarConfig,
    state_dir: &Path,
) -> Result<LoadedPlan, String> {
    let content = std::fs::read_to_string(plan_path).map_err(|e| format!("read plan file: {e}"))?;
    let doc: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("parse plan file: {e}"))?;

    let format = plan_str(&doc, "format", "").to_string();
    let sealed = match format.as_str() {
        FORMAT_V2 => true,
        FORMAT_V1 => false,
        other => return Err(format!("unsupported plan format: '{other}'")),
    };

    let plan = plan_body_from_doc(&doc)?;
    let selectors = selectors_from_doc(&doc)?;
    if sealed {
        check_v2(&doc, &plan, &selectors, config, state_dir)?;
    } else {
        check_v1(&doc, &plan, config)?;
    }
    Ok(LoadedPlan {
        plan,
        sealed,
        selectors,
    })
}
