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
///
/// forjar#501, three things the reader must agree with the writer on:
/// * the MACHINE. The lock's `input_hash` was taken on THIS host. For a
///   machine this host does not answer for it is a hash of the wrong tree
///   (or, since #501, absent), so the cache has no answer and the task runs
///   there — skipping a remote run from the controller's files was the
///   forjar#485 shape at this site.
/// * the BASE. `record_io_hashes` hashes relative to `probe_base_dir` —
///   `working_dir` — and `hash_inputs` folds the expanded path into the hash,
///   so hashing against the state directory's parent could never match it:
///   the cache was dead, and its deadness hid the machine half.
/// * the QUESTION. Inputs alone were the FJ-2701 test; the first live run of
///   it skipped a task whose output had been deleted (the plan said `output
///   artifact missing`, apply said `unchanged`). The reader now asks exactly
///   what the planner asks — [`probe_resource`] against what the lock
///   recorded, through [`staleness_reason`] — so a hit means inputs unchanged
///   AND outputs present and unmodified.
///
/// [`probe_resource`]: crate::core::task::probe::probe_resource
/// [`staleness_reason`]: crate::core::task::staleness_reason
pub(crate) fn check_task_input_cache(
    resource_id: &str,
    resource: &Resource,
    machine: &Machine,
    ctx: &RecordCtx,
) -> Option<String> {
    if !crate::core::task::probe::probe_answers_for(machine) {
        return None;
    }
    let probe = crate::core::task::probe::probe_resource(resource)?;
    let current_hash = probe.input_hash.clone()?;
    let rl = ctx.lock.resources.get(resource_id)?;
    let stored_in = rl.details.get("input_hash").and_then(|v| v.as_str());
    let stored_out = rl.details.get("output_hash").and_then(|v| v.as_str());

    if !crate::core::task::should_skip_cached(true, Some(&current_hash), stored_in) {
        return None;
    }
    if crate::core::task::staleness_reason(&probe, stored_in, stored_out).is_some() {
        return None;
    }
    Some(format!("inputs unchanged (hash: {:.16}...)", current_hash))
}

/// forjar#501: a cache hit satisfies the CURRENT spec. The row keeps its
/// observed I/O hashes — "inputs unchanged, outputs present" is what the hit
/// means — and takes the spec hash of the resource it was asked to converge,
/// so the next plan settles on `NoOp`. Measured before this: the plan said
/// `1 to change` and apply said `1 unchanged`, on every run, forever.
pub(crate) fn settle_cached_row(ctx: &mut RecordCtx, resource_id: &str, resolved: &Resource) {
    if let Some(rl) = ctx.lock.resources.get_mut(resource_id) {
        rl.hash = planner::hash_desired_state(resolved);
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
