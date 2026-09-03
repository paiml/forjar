//! Single-resource operations: apply, record success/failure, copia sync, tripwire logging.

use super::*;

/// Outcome of applying a single resource.
pub(crate) enum ResourceOutcome {
    /// Resource converged successfully.
    Converged,
    /// Resource was unchanged (NoOp, not forced).
    Unchanged,
    /// Resource was skipped (filtered out or not found).
    Skipped,
    /// The resource failed; `record_failure` already decided whether the
    /// machine stops and the wave already decided whether to retry.
    Failed,
}

/// Shared context for recording resource outcomes.
pub(crate) struct RecordCtx<'a> {
    pub lock: &'a mut StateLock,
    pub state_dir: &'a std::path::Path,
    pub machine_name: &'a str,
    pub tripwire: bool,
    pub failure_policy: &'a FailurePolicy,
    pub timeout_secs: Option<u64>,
}

/// Record a successful resource application into the lock and event log.
pub(crate) fn record_success(
    ctx: &mut RecordCtx,
    resource_id: &str,
    resource: &Resource,
    resolved: &Resource,
    machine: &Machine,
    duration: f64,
) {
    let desired_hash = planner::hash_desired_state(resolved);

    // Live state hash for drift detection. Query stdout may legitimately
    // be empty when the queried file/service doesn't exist yet — use the
    // sentinel wrapper to uphold the STRONG `!input.is_empty()` precondition
    // without losing the drift signal.
    let live_hash = match codegen::state_query_script(resolved) {
        Ok(query) => match transport::exec_script_timeout(machine, &query, ctx.timeout_secs) {
            Ok(qout) if qout.success() => Some(hasher::hash_string_or_sentinel(&qout.stdout)),
            _ => None,
        },
        Err(_) => None,
    };

    let mut details = build_resource_details(resolved, machine);
    // NOTE: `live_hash` is inserted into `details` here rather than through
    // `ResourceLock::set_observed_state`, because the lock struct is not built
    // until further down this function. The typed field is populated from this
    // same value at construction; see the `observed:` field below. Both are
    // written from ONE source — the state query that actually reached the
    // target — which is what distinguishes this from forjar#305.
    if let Some(ref lh) = live_hash {
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String(lh.clone()),
        );
    }

    // FJ-2710 (PMAT-197): record observed I/O so the NEXT plan can detect
    // staleness. See core::task::probe::record_io_hashes.
    crate::core::task::probe::record_io_hashes(resolved, &mut details);

    // FJ-266: `insert` returns the entry it displaced, which is the
    // before-state this converge overwrote. Free — no extra read.
    let previous = ctx.lock.resources.insert(
        resource_id.to_string(),
        ResourceLock {
            resource_type: resource.resource_type.clone(),
            status: ResourceStatus::Converged,
            applied_at: Some(eventlog::now_iso8601()),
            duration_seconds: Some(duration),
            // SPEC and STATUS, side by side, so the difference is legible at
            // the one site that writes both.
            //   hash     = hash_desired_state(resource) — the CONFIG, never a host
            //   observed = digest of the state query's stdout, from the TARGET
            // Reading one where you meant the other is forjar#305.
            hash: desired_hash.clone(),
            observed: live_hash.clone(),
            details,
        },
    );
    let previous_hash = crate::tripwire::eventlog::displaced_hash(previous);

    crate::core::executor::log_tripwire(
        ctx.state_dir,
        ctx.machine_name,
        ctx.tripwire,
        ProvenanceEvent::ResourceConverged {
            machine: ctx.machine_name.to_string(),
            resource: resource_id.to_string(),
            duration_seconds: duration,
            hash: desired_hash,
            previous_hash,
        },
    );
}

/// Record a resource failure into the lock and event log. Returns true if jidoka should stop.
pub(crate) fn record_failure(
    ctx: &mut RecordCtx,
    resource_id: &str,
    resource_type: &ResourceType,
    duration: f64,
    error: &str,
) -> bool {
    // Contract: execution-safety-v1.yaml precondition (pv codegen)
    contract_pre_jidoka_stop!(resource_id);
    ctx.lock.resources.insert(
        resource_id.to_string(),
        ResourceLock {
            resource_type: resource_type.clone(),
            status: ResourceStatus::Failed,
            applied_at: Some(eventlog::now_iso8601()),
            duration_seconds: Some(duration),
            hash: String::new(),
            // A resource that FAILED observed nothing. `None` is correct and
            // load-bearing: drift skips not-observed rather than treating an
            // absent digest as agreement.
            observed: None,
            // Refs #390-C: THE FAILURE TEXT, so machine-readable output carries
            // it. `build_resource_reports` reads `details["error"]`, a key this
            // function never wrote -- so `--json`, `--output events` and
            // `--report` emitted `"error": null` for every failed resource and
            // were strictly WORSE than the console, which at least printed
            // stderr. For a CI pipeline that is the surface that matters.
            //
            // Deferred from 1.24.0 for a real reason: this string lands in
            // `state.lock.yaml`, which is re-serialised and blake3-sidecarred
            // every run and commonly committed. It is safe NOW because #390
            // bounded every one of the six `record_failure` call sites -- before
            // that, an unbounded stderr could have gone into a hashed, committed
            // file.
            details: HashMap::from([(
                "error".to_string(),
                serde_yaml_ng::Value::String(error.to_string()),
            )]),
        },
    );

    crate::core::executor::log_tripwire(
        ctx.state_dir,
        ctx.machine_name,
        ctx.tripwire,
        ProvenanceEvent::ResourceFailed {
            machine: ctx.machine_name.to_string(),
            resource: resource_id.to_string(),
            error: error.to_string(),
        },
    );

    if *ctx.failure_policy == FailurePolicy::StopOnFirst {
        eprintln!(
            "JIDOKA: {}/{} failed — dependents will be skipped: {}",
            ctx.machine_name, resource_id, error
        );
        return true;
    }

    false
}
