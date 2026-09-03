//! Per-resource bookkeeping shared by every wave width: the FJ-2701 input
//! cache and the FJ-2301 run metadata.
//!
//! Refs #412: both used to live in `resource_ops.rs` and be reachable only from
//! the width-1 path, so a `cache: true` task re-ran under `--parallel` and a
//! wide wave left `runs/<id>/meta.yaml` with an empty `resources:` map. They are
//! called from both widths now, and live here so `resource_ops.rs` stays under
//! the 500-line ceiling.

use super::*;

/// FJ-2701: Check if task inputs are unchanged since last successful run.
///
/// Returns Some(message) if the task should be skipped (cache hit).
/// Refs #412: shared with the wide-wave prepare phase, which used to skip the
/// cache entirely — the same `cache: true` task re-ran under `--parallel`.
pub(crate) fn check_task_input_cache(
    resource_id: &str,
    resource: &Resource,
    ctx: &RecordCtx,
) -> Option<String> {
    let base_dir = ctx.state_dir.parent().unwrap_or(ctx.state_dir);
    let current_hash = crate::core::task::hash_declared_inputs(resource, base_dir)?;
    let stored_hash = ctx
        .lock
        .resources
        .get(resource_id)
        .and_then(|rl| rl.details.get("input_hash"))
        .and_then(|v| v.as_str());

    if crate::core::task::should_skip_cached(true, Some(&current_hash), stored_hash) {
        Some(format!("inputs unchanged (hash: {:.16}...)", current_hash))
    } else {
        None
    }
}

/// Update meta.yaml with resource status after execution.
pub(crate) fn update_run_meta(
    ctx: &RecordCtx,
    run_id: Option<&str>,
    resource_id: &str,
    status: ResourceRunStatus,
) {
    if let Some(rid) = run_id {
        let dir = run_capture::run_dir(ctx.state_dir, ctx.machine_name, rid);
        run_capture::update_meta_resource(&dir, resource_id, status);
    }
}
