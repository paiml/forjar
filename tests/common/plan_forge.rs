//! Refs #358 — THE ADVERSARY, shared by every test that has to forge a plan.
//!
//! Rewrite an honest plan document's body to a lie and re-seal it with the
//! PUBLIC `plan_seal::digest` API. `config_hash` and `state_hash` are copied
//! verbatim out of the honest plan, so neither of those legs moves; only the
//! diff leg and the composition are recomputed, exactly as forjar itself would
//! compute them.
//!
//! The result is a document that passes every check the seal can perform. That
//! is not a defect in the seal — it is what "unkeyed" means, and
//! `core::plan_seal` says so — which is why the refusals these tests demand come
//! from re-planning rather than from more hashing.
//!
//! One copy, because two spellings of the adversary is how one of them quietly
//! stops being the attack it was written to be.

#![allow(dead_code)]

use forjar::core::plan_seal::digest;
use forjar::core::plan_selectors::PlanSelectors;
use forjar::core::types::{ExecutionPlan, PlanAction, PlannedChange, ResourceType};
use std::path::Path;

/// Read a plan document.
pub fn read_plan(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("read plan")).expect("parse plan")
}

fn str_at(doc: &serde_json::Value, path: [&str; 2]) -> String {
    doc[path[0]][path[1]]
        .as_str()
        .unwrap_or_else(|| panic!("plan file has no {}.{}", path[0], path[1]))
        .to_string()
}

/// One entry in a forged change list.
pub fn change(id: &str, machine: &str, action: PlanAction, description: &str) -> PlannedChange {
    PlannedChange {
        resource_id: id.to_string(),
        machine: machine.to_string(),
        resource_type: ResourceType::File,
        action,
        description: description.to_string(),
    }
}

/// An `ExecutionPlan` whose counters partition its own change list, so
/// `plan_seal::check_body_partition` is silent and the refusal under test has to
/// come from somewhere else.
pub fn body(name: &str, changes: Vec<PlannedChange>, execution_order: &[&str]) -> ExecutionPlan {
    let mut plan = ExecutionPlan {
        name: name.to_string(),
        changes,
        execution_order: execution_order.iter().map(|s| s.to_string()).collect(),
        to_create: 0,
        to_update: 0,
        to_destroy: 0,
        unchanged: 0,
    };
    for c in &plan.changes {
        match c.action {
            PlanAction::Create => plan.to_create += 1,
            PlanAction::Update => plan.to_update += 1,
            PlanAction::Destroy => plan.to_destroy += 1,
            PlanAction::NoOp => plan.unchanged += 1,
        }
    }
    plan
}

/// Overwrite `plan_path` with `body` under `selectors`, re-sealed.
///
/// `selectors` is what the forgery CLAIMS the plan was filtered by. Forging it
/// is allowed and expected — the point of sealing the record is not that an
/// adversary cannot write one, it is that they can no longer leave the question
/// unanswered, which is what made a deleted line indistinguishable from a
/// legitimately narrow plan.
pub fn reseal_as(plan_path: &Path, body: &ExecutionPlan, selectors: &PlanSelectors) {
    let honest = read_plan(plan_path);
    let config_hash = str_at(&honest, ["seal", "config_hash"]);
    let state_hash = str_at(&honest, ["seal", "state_hash"]);
    let sealed_at = honest["seal"]["sealed_at_unix"]
        .as_u64()
        .expect("sealed_at");
    let ttl = honest["seal"]["ttl_secs"].as_u64().expect("ttl");

    let diff_hash = digest::diff_leg(body, selectors).expect("diff leg");
    let seal = digest::compose(&config_hash, &state_hash, &diff_hash, sealed_at, ttl);

    let changes: Vec<serde_json::Value> = body
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
    let forged = serde_json::json!({
        "format": "forjar-plan-v2",
        "config_file": honest["config_file"],
        "config_hash": config_hash,
        "name": body.name,
        "to_create": body.to_create,
        "to_update": body.to_update,
        "to_destroy": body.to_destroy,
        "unchanged": body.unchanged,
        "execution_order": body.execution_order,
        "changes": changes,
        "selectors": selectors,
        "seal": {
            "version": honest["seal"]["version"],
            "plan_id": digest::plan_id(&seal),
            "config_hash": config_hash,
            "state_hash": state_hash,
            "diff_hash": diff_hash,
            "sealed_at_unix": sealed_at,
            "ttl_secs": ttl,
            "seal": seal,
        },
    });
    std::fs::write(
        plan_path,
        serde_json::to_string_pretty(&forged).expect("render"),
    )
    .expect("write forged plan");
}

/// The common case: forge a body and claim the plan was unfiltered.
pub fn reseal(plan_path: &Path, body: &ExecutionPlan) {
    reseal_as(plan_path, body, &PlanSelectors::default());
}

/// stdout and stderr together — the refusals land on both.
pub fn combined(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
