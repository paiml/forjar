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

    // FJ-2701: Store task input hash for cache-based skip on next run
    if resolved.cache && !resolved.task_inputs.is_empty() {
        let base_dir = ctx.state_dir.parent().unwrap_or(ctx.state_dir);
        if let Ok(Some(hash)) = crate::core::task::hash_inputs(&resolved.task_inputs, base_dir) {
            details.insert("input_hash".to_string(), serde_yaml_ng::Value::String(hash));
        }
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
            "JIDOKA: {}/{} failed — dependents will be skipped: {}",
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

    let triggered = cfg
        .config
        .resources
        .get(&change.resource_id)
        .is_some_and(|r| {
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
    if cfg
        .tag_filter
        .is_some_and(|tag| !resource.tags.iter().any(|t| t == tag))
    {
        return true;
    }
    if cfg
        .group_filter
        .is_some_and(|group| resource.resource_group.as_deref() != Some(group))
    {
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
            record_failure(
                ctx,
                &change.resource_id,
                &resource.resource_type,
                duration,
                &error,
            );
            return Ok(ResourceOutcome::Skipped);
        }
    }

    // FJ-2701: Task input caching — skip execution if inputs unchanged
    if resolved.cache && !resolved.task_inputs.is_empty() {
        if let Some(cached) = check_task_input_cache(&change.resource_id, resolved, ctx) {
            if cfg.trace {
                eprintln!("[TRACE] {} cached: {}", change.resource_id, cached);
            }
            return Ok(ResourceOutcome::Unchanged);
        }
    }

    let ssh_retries = cfg.config.policy.ssh_retries;
    let output = if resolved.resource_type == ResourceType::File
        && resolved
            .source
            .as_ref()
            .map(|s| copia::is_eligible(s))
            .unwrap_or(false)
    {
        copia_apply_file(machine, resolved, ctx.timeout_secs)
    } else {
        let script = codegen::apply_script(resolved)?;
        // FJ-1397: Debug trace mode — print generated script
        if cfg.trace {
            eprintln!("[TRACE] {} script:\n{}", change.resource_id, script);
        }
        transport::exec_script_retry(machine, &script, ctx.timeout_secs, ssh_retries)
    };
    let duration = resource_start.elapsed().as_secs_f64();

    handle_resource_output(
        output, cfg, change, resource, resolved, machine, ctx, duration,
    )
}

/// Run the pre_apply hook; returns error string on failure.
fn run_pre_apply_hook(machine: &Machine, hook: &str, timeout: Option<u64>) -> Option<String> {
    match transport::exec_script_timeout(machine, hook, timeout) {
        Ok(out) if !out.success() => Some(format!(
            "pre_apply hook failed (exit {}): {}",
            out.exit_code,
            out.stderr.trim()
        )),
        Err(e) => Some(format!("pre_apply hook error: {e}")),
        _ => None,
    }
}

/// FJ-2701: Check if task inputs are unchanged since last successful run.
///
/// Returns Some(message) if the task should be skipped (cache hit).
fn check_task_input_cache(
    resource_id: &str,
    resource: &Resource,
    ctx: &RecordCtx,
) -> Option<String> {
    let base_dir = ctx.state_dir.parent().unwrap_or(ctx.state_dir);
    let current_hash = crate::core::task::hash_inputs(&resource.task_inputs, base_dir).ok()??;
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

/// FJ-2301: Persist ExecOutput to .log files for post-mortem debugging.
fn capture_exec_output(
    ctx: &RecordCtx,
    run_id: Option<&str>,
    resource_id: &str,
    action: &str,
    output: &transport::ExecOutput,
    duration: f64,
) {
    let rid = run_id.unwrap_or("run-adhoc");
    let run_dir = run_capture::run_dir(ctx.state_dir, ctx.machine_name, rid);
    run_capture::ensure_run_dir(&run_dir, rid, ctx.machine_name, "apply");
    let rt = "unknown"; // resource type not available here; log content is primary
    run_capture::capture_output(
        &run_dir,
        resource_id,
        rt,
        action,
        ctx.machine_name,
        "transport",
        "",
        output,
        duration,
    );
}

/// Handle the output of a resource execution, including post_apply hook.
#[allow(clippy::too_many_arguments)]
fn handle_resource_output(
    output: Result<transport::ExecOutput, String>,
    cfg: &ApplyConfig,
    change: &PlannedChange,
    resource: &Resource,
    resolved: &Resource,
    machine: &Machine,
    ctx: &mut RecordCtx,
    duration: f64,
) -> Result<ResourceOutcome, String> {
    // FJ-2301: Capture output to run log directory
    if let Ok(ref out) = output {
        let action_str = format!("{:?}", change.action).to_lowercase();
        capture_exec_output(
            ctx,
            cfg.run_id.as_deref(),
            &change.resource_id,
            &action_str,
            out,
            duration,
        );
    }
    match output {
        Ok(out) if out.success() => {
            if let Some(ref post_hook) = resolved.post_apply {
                if let Some(error) = check_post_hook(machine, post_hook, ctx.timeout_secs) {
                    let should_stop = record_failure(
                        ctx,
                        &change.resource_id,
                        &resource.resource_type,
                        duration,
                        &error,
                    );
                    return Ok(ResourceOutcome::Failed { should_stop });
                }
            }
            record_success(
                ctx,
                &change.resource_id,
                resource,
                resolved,
                machine,
                duration,
            );
            update_run_meta(
                ctx,
                cfg.run_id.as_deref(),
                &change.resource_id,
                ResourceRunStatus::Converged {
                    exit_code: Some(0),
                    duration_secs: Some(duration),
                    failed: false,
                },
            );
            Ok(ResourceOutcome::Converged)
        }
        Ok(out) => {
            let error = format!("exit code {}: {}", out.exit_code, out.stderr.trim());
            let should_stop = record_failure(
                ctx,
                &change.resource_id,
                &resource.resource_type,
                duration,
                &error,
            );
            update_run_meta(
                ctx,
                cfg.run_id.as_deref(),
                &change.resource_id,
                ResourceRunStatus::Converged {
                    exit_code: Some(out.exit_code),
                    duration_secs: Some(duration),
                    failed: true,
                },
            );
            Ok(ResourceOutcome::Failed { should_stop })
        }
        Err(e) => {
            let error = format!("transport error: {e}");
            let should_stop = record_failure(
                ctx,
                &change.resource_id,
                &resource.resource_type,
                duration,
                &error,
            );
            Ok(ResourceOutcome::Failed { should_stop })
        }
    }
}

/// Update meta.yaml with resource status after execution.
fn update_run_meta(
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

/// Run post_apply hook; returns error string on failure.
fn check_post_hook(machine: &Machine, hook: &str, timeout: Option<u64>) -> Option<String> {
    match transport::exec_script_timeout(machine, hook, timeout) {
        Ok(pout) if !pout.success() => Some(format!(
            "post_apply hook failed (exit {}): {}",
            pout.exit_code,
            pout.stderr.trim()
        )),
        Err(e) => Some(format!("post_apply hook error: {e}")),
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

    let resolved = resolver::resolve_resource_templates_with_secrets(
        resource,
        &cfg.config.params,
        &cfg.config.machines,
        &cfg.config.secrets,
    )?;

    execute_resource(cfg, change, resource, &resolved, machine, ctx)
}
