//! `forjar plan --json`.
//!
//! Split out of `plan.rs` (forjar#342): that file was 514 lines against the
//! repo's 500-line ceiling, which the pre-commit ratchet enforces by refusing
//! growth, so the disclosure this issue needed could not be added in place.

use crate::core::types;

/// FJ-301: Serialize plan as enriched JSON with resource metadata.
///
/// forjar#342 landed the scope disclosure on the TTY rendering only, because
/// `print_scope_disclosure` formatted the sentence and immediately printed it —
/// there was no value for this function to serialise. That inverted the issue's
/// own threat model: the consumers that cannot notice a missing disclosure (a
/// CI parser, an MCP agent reading `to_update: 0`) were the ones still receiving
/// an undisclosed lock diff, while the human at a terminal, who at least has
/// `forjar drift` in muscle memory, was the only one told.
pub(crate) fn print_plan_json(
    plan: &types::ExecutionPlan,
    config: &types::ForjarConfig,
    unconsulted: usize,
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
    let change_ids: std::collections::HashSet<&str> = plan
        .changes
        .iter()
        .map(|c| c.resource_id.as_str())
        .collect();
    let filtered_order: Vec<&str> = plan
        .execution_order
        .iter()
        .filter(|id| change_ids.contains(id.as_str()))
        .map(|s| s.as_str())
        .collect();
    let mut output = serde_json::json!({
        "name": plan.name,
        "to_create": plan.to_create,
        "to_update": plan.to_update,
        "to_destroy": plan.to_destroy,
        "unchanged": plan.unchanged,
        "execution_order": filtered_order,
        "changes": changes,
        // forjar#342: STATE THE QUANTIFIER THIS REPORT RANGES OVER. Both are
        // unconditional facts, because a machine consumer needs a TOTAL
        // function: `unconsulted_observations: 0` says "nothing observed",
        // while an absent key says "old binary", and a parser must be able to
        // tell those apart.
        "lock_relative": true,
        "unconsulted_observations": unconsulted,
    });
    // The prose disclosure is present iff there is a blind spot to declare —
    // the contract's biconditional, and the reason it is not an unconditional
    // banner: noise is how a warning stops being read.
    if let Some(msg) = super::print_helpers::scope_disclosure(unconsulted) {
        output["disclosure"] = serde_json::json!(msg);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {e}"))?
    );
    Ok(())
}
