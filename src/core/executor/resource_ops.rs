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
    /// Resource failed; includes whether to stop (jidoka).
    Failed { should_stop: bool },
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

    // Live state hash for drift detection
    let live_hash = match codegen::state_query_script(resolved) {
        Ok(query) => match transport::exec_script_timeout(machine, &query, ctx.timeout_secs) {
            Ok(qout) if qout.success() => Some(hasher::hash_string(&qout.stdout)),
            _ => None,
        },
        Err(_) => None,
    };

    let mut details = build_resource_details(resolved, machine);
    if let Some(ref lh) = live_hash {
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String(lh.clone()),
        );
    }

    ctx.lock.resources.insert(
        resource_id.to_string(),
        ResourceLock {
            resource_type: resource.resource_type.clone(),
            status: ResourceStatus::Converged,
            applied_at: Some(eventlog::now_iso8601()),
            duration_seconds: Some(duration),
            hash: desired_hash.clone(),
            details,
        },
    );

    if ctx.tripwire {
        let _ = eventlog::append_event(
            ctx.state_dir,
            ctx.machine_name,
            ProvenanceEvent::ResourceConverged {
                machine: ctx.machine_name.to_string(),
                resource: resource_id.to_string(),
                duration_seconds: duration,
                hash: desired_hash,
            },
        );
    }
}

/// Record a resource failure into the lock and event log. Returns true if jidoka should stop.
pub(crate) fn record_failure(
    ctx: &mut RecordCtx,
    resource_id: &str,
    resource_type: &ResourceType,
    duration: f64,
    error: &str,
) -> bool {
    ctx.lock.resources.insert(
        resource_id.to_string(),
        ResourceLock {
            resource_type: resource_type.clone(),
            status: ResourceStatus::Failed,
            applied_at: Some(eventlog::now_iso8601()),
            duration_seconds: Some(duration),
            hash: String::new(),
            details: HashMap::new(),
        },
    );

    if ctx.tripwire {
        let _ = eventlog::append_event(
            ctx.state_dir,
            ctx.machine_name,
            ProvenanceEvent::ResourceFailed {
                machine: ctx.machine_name.to_string(),
                resource: resource_id.to_string(),
                error: error.to_string(),
            },
        );
    }

    if *ctx.failure_policy == FailurePolicy::StopOnFirst {
        eprintln!(
            "JIDOKA: stopping after failure on {}/{}: {}",
            ctx.machine_name, resource_id, error
        );
        return true;
    }

    false
}

/// Check if a resource should be skipped based on filters and conditions.
fn should_skip_single(
    cfg: &ApplyConfig,
    change: &PlannedChange,
    machine: &Machine,
    converged_resources: &HashSet<String>,
) -> Option<ResourceOutcome> {
    if cfg.resource_filter.is_some_and(|f| change.resource_id != f) {
        return Some(ResourceOutcome::Skipped);
    }

    let triggered = cfg.config.resources.get(&change.resource_id).is_some_and(|r| {
        !r.triggers.is_empty() && r.triggers.iter().any(|t| converged_resources.contains(t))
    });

    if change.action == PlanAction::NoOp && !cfg.force && !triggered {
        return Some(ResourceOutcome::Unchanged);
    }

    let resource = cfg.config.resources.get(&change.resource_id)?;

    if resource_filtered_out(cfg, resource, machine) {
        return Some(ResourceOutcome::Skipped);
    }
    None
}

/// Check if a resource is filtered out by arch/tag/group/when.
fn resource_filtered_out(cfg: &ApplyConfig, resource: &Resource, machine: &Machine) -> bool {
    if !resource.arch.is_empty() && !resource.arch.contains(&machine.arch) {
        return true;
    }
    if cfg.tag_filter.is_some_and(|tag| !resource.tags.iter().any(|t| t == tag)) {
        return true;
    }
    if cfg.group_filter.is_some_and(|group| resource.resource_group.as_deref() != Some(group)) {
        return true;
    }
    if let Some(ref when_expr) = resource.when {
        return !conditions::evaluate_when(when_expr, &cfg.config.params, machine).unwrap_or(false);
    }
    false
}

/// Execute the resolved resource script and handle hooks.
fn execute_resource(
    cfg: &ApplyConfig,
    change: &PlannedChange,
    resource: &Resource,
    resolved: &Resource,
    machine: &Machine,
    ctx: &mut RecordCtx,
) -> Result<ResourceOutcome, String> {
    let resource_start = Instant::now();

    // FJ-265: pre_apply hook
    if let Some(ref pre_hook) = resolved.pre_apply {
        if let Some(error) = run_pre_apply_hook(machine, pre_hook, ctx.timeout_secs) {
            let duration = resource_start.elapsed().as_secs_f64();
            record_failure(ctx, &change.resource_id, &resource.resource_type, duration, &error);
            return Ok(ResourceOutcome::Skipped);
        }
    }

    let ssh_retries = cfg.config.policy.ssh_retries;
    let output = if resolved.resource_type == ResourceType::File
        && resolved.source.is_some()
        && copia::is_eligible(resolved.source.as_ref().unwrap())
    {
        copia_apply_file(machine, resolved, ctx.timeout_secs)
    } else {
        let script = codegen::apply_script(resolved)?;
        transport::exec_script_retry(machine, &script, ctx.timeout_secs, ssh_retries)
    };
    let duration = resource_start.elapsed().as_secs_f64();

    handle_resource_output(output, cfg, change, resource, resolved, machine, ctx, duration)
}

/// Run the pre_apply hook; returns error string on failure.
fn run_pre_apply_hook(machine: &Machine, hook: &str, timeout: Option<u64>) -> Option<String> {
    match transport::exec_script_timeout(machine, hook, timeout) {
        Ok(out) if !out.success() => Some(format!(
            "pre_apply hook failed (exit {}): {}", out.exit_code, out.stderr.trim()
        )),
        Err(e) => Some(format!("pre_apply hook error: {}", e)),
        _ => None,
    }
}

/// Handle the output of a resource execution, including post_apply hook.
#[allow(clippy::too_many_arguments)]
fn handle_resource_output(
    output: Result<transport::ExecOutput, String>,
    _cfg: &ApplyConfig,
    change: &PlannedChange,
    resource: &Resource,
    resolved: &Resource,
    machine: &Machine,
    ctx: &mut RecordCtx,
    duration: f64,
) -> Result<ResourceOutcome, String> {
    match output {
        Ok(out) if out.success() => {
            if let Some(ref post_hook) = resolved.post_apply {
                if let Some(error) = check_post_hook(machine, post_hook, ctx.timeout_secs) {
                    let should_stop = record_failure(ctx, &change.resource_id, &resource.resource_type, duration, &error);
                    return Ok(ResourceOutcome::Failed { should_stop });
                }
            }
            record_success(ctx, &change.resource_id, resource, resolved, machine, duration);
            Ok(ResourceOutcome::Converged)
        }
        Ok(out) => {
            let error = format!("exit code {}: {}", out.exit_code, out.stderr.trim());
            let should_stop = record_failure(ctx, &change.resource_id, &resource.resource_type, duration, &error);
            Ok(ResourceOutcome::Failed { should_stop })
        }
        Err(e) => {
            let error = format!("transport error: {}", e);
            let should_stop = record_failure(ctx, &change.resource_id, &resource.resource_type, duration, &error);
            Ok(ResourceOutcome::Failed { should_stop })
        }
    }
}

/// Run post_apply hook; returns error string on failure.
fn check_post_hook(machine: &Machine, hook: &str, timeout: Option<u64>) -> Option<String> {
    match transport::exec_script_timeout(machine, hook, timeout) {
        Ok(pout) if !pout.success() => Some(format!(
            "post_apply hook failed (exit {}): {}", pout.exit_code, pout.stderr.trim()
        )),
        Err(e) => Some(format!("post_apply hook error: {}", e)),
        _ => None,
    }
}

/// Apply a single planned change, returning its outcome.
pub(crate) fn apply_single_resource(
    cfg: &ApplyConfig,
    change: &PlannedChange,
    machine: &Machine,
    ctx: &mut RecordCtx,
    converged_resources: &HashSet<String>,
) -> Result<ResourceOutcome, String> {
    if let Some(outcome) = should_skip_single(cfg, change, machine, converged_resources) {
        return Ok(outcome);
    }

    let resource = match cfg.config.resources.get(&change.resource_id) {
        Some(r) => r,
        None => return Ok(ResourceOutcome::Skipped),
    };

    if ctx.tripwire {
        let _ = eventlog::append_event(
            ctx.state_dir,
            ctx.machine_name,
            ProvenanceEvent::ResourceStarted {
                machine: ctx.machine_name.to_string(),
                resource: change.resource_id.clone(),
                action: change.action.to_string(),
            },
        );
    }

    let resolved =
        resolver::resolve_resource_templates(resource, &cfg.config.params, &cfg.config.machines)?;

    execute_resource(cfg, change, resource, &resolved, machine, ctx)
}

/// FJ-242: Two-phase copia delta sync for large file sources.
/// Phase 1: Execute signature script on remote to get per-block BLAKE3 hashes.
/// Phase 2: Compute delta locally, transfer only changed blocks.
/// Falls back to full base64 transfer for new files (no remote state to diff).
pub(crate) fn copia_apply_file(
    machine: &Machine,
    resource: &Resource,
    timeout_secs: Option<u64>,
) -> Result<transport::ExecOutput, String> {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let source = resource.source.as_deref().unwrap_or("");

    // Phase 1: Get remote file block signatures
    let sig_script = copia::signature_script(path);
    let sig_output = transport::exec_script_timeout(machine, &sig_script, timeout_secs)?;

    if !sig_output.success() {
        return Err(format!(
            "copia signature failed: {}",
            sig_output.stderr.trim()
        ));
    }

    let remote_sigs = copia::parse_signatures(&sig_output.stdout)?;

    let owner = resource.owner.as_deref();
    let group = resource.group.as_deref();
    let mode = resource.mode.as_deref();

    match remote_sigs {
        None => {
            // New file — full transfer via base64
            let script = copia::full_transfer_script(path, source, owner, group, mode)?;
            transport::exec_script_timeout(machine, &script, timeout_secs)
        }
        Some(sigs) => {
            // Read local source file
            let new_data =
                std::fs::read(source).map_err(|e| format!("copia read source: {}", e))?;

            // Compute delta
            let delta = copia::compute_delta(&new_data, &sigs);

            // Generate and execute patch script
            let script = copia::patch_script(path, &delta, owner, group, mode);
            transport::exec_script_timeout(machine, &script, timeout_secs)
        }
    }
}

/// Log a tripwire event if tripwire is enabled.
pub(crate) fn log_tripwire(
    state_dir: &std::path::Path,
    machine: &str,
    tripwire: bool,
    event: ProvenanceEvent,
) {
    if tripwire {
        let _ = eventlog::append_event(state_dir, machine, event);
    }
}
