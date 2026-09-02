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
    /// Resource failed; includes whether to stop (jidoka) and whether the
    /// failure is safe to retry. `retryable` is `false` for pre_apply gate
    /// failures (#165) — re-running that hook under `--retry` would re-execute
    /// its non-idempotent side effects; genuine apply failures stay retryable.
    Failed { should_stop: bool, retryable: bool },
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
    // Bug-hunt #3 (Refs #154): a failing pre_apply hook must surface as a real
    // Failed outcome (propagating jidoka's should_stop), NOT Skipped. Returning
    // Skipped here is a no-op in MachineCounters::record, so the sequential /
    // single-resource-wave path left counters.failed == 0 → apply exited 0
    // reporting success, rollback was skipped, and dependents ran as if the
    // prerequisite had succeeded. Mirror the post_apply branch below.
    if let Some(ref pre_hook) = resolved.pre_apply {
        if let Some(error) =
            super::output_verify::run_pre_apply_hook(machine, pre_hook, ctx.timeout_secs)
        {
            let duration = resource_start.elapsed().as_secs_f64();
            let should_stop = record_failure(
                ctx,
                &change.resource_id,
                &resource.resource_type,
                duration,
                &error,
            );
            // #165: pre_apply gate failures are non-retryable so --retry does
            // not re-run the hook's side effects. The resource still fails,
            // cascades to dependents, and triggers rollback — it just doesn't loop.
            return Ok(ResourceOutcome::Failed {
                should_stop,
                retryable: false,
            });
        }
    }

    // FJ-2701: Task input caching — skip execution if inputs unchanged
    if resolved.cache && crate::core::task::declares_inputs(resolved) {
        if let Some(cached) = check_task_input_cache(&change.resource_id, resolved, ctx) {
            if cfg.trace {
                eprintln!("[TRACE] {} cached: {}", change.resource_id, cached);
            }
            return Ok(ResourceOutcome::Unchanged);
        }
    }

    let ssh_retries = cfg.config.policy.ssh_retries;
    // Dogfood #208 (logs-script-flag-noop): capture the generated script so the
    // `.script` sidecar and `script_hash` are not empty and `--script` has
    // something to show.
    let mut executed_script = String::new();
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
        let result = transport::exec_script_retry(machine, &script, ctx.timeout_secs, ssh_retries);
        executed_script = script;
        result
    };
    let duration = resource_start.elapsed().as_secs_f64();

    let s = &executed_script;
    handle_resource_output(
        output, cfg, change, resource, resolved, machine, ctx, duration, s,
    )
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
    executed_script: &str,
) -> Result<ResourceOutcome, String> {
    // FJ-2301 / Refs #390: persist the transcript FIRST and keep the path, so
    // a failure message can only ever name a run log that exists.
    let action = format!("{:?}", change.action).to_lowercase();
    let slot = run_capture::RunSlot::new(ctx.state_dir, ctx.machine_name, cfg.run_id.as_deref());
    let executed = run_capture::Executed {
        resource_id: &change.resource_id,
        resource_type: &resource.resource_type,
        action: &action,
        script: executed_script,
        // Refs #406: `resource`, not `resolved` — see `Transcript::for_resource`.
        transcript: run_capture::Transcript::for_resource(resource, &cfg.config.secrets),
    };
    let log = output
        .as_ref()
        .ok()
        .and_then(|out| run_capture::capture_exec_output(&slot, &executed, out, duration));
    let site = super::failure_text::Site {
        resource_id: &change.resource_id,
        state_dir: ctx.state_dir,
        run_id: cfg.run_id.as_deref(),
        log: log.as_deref(),
        resolved,
    };
    match output {
        Ok(out) if out.success() => {
            // Three post-apply questions, asked in one place: did the hook
            // pass, were the declared outputs produced, and does the HOST
            // report the declared state? Each used to be its own near-identical
            // record_failure block here; consolidated into output_verify so
            // adding a fourth does not grow this file again.
            if let Some(verdict) =
                super::output_verify::post_apply_failure(resolved, machine, ctx.timeout_secs)
            {
                let error = super::failure_text::verify_failure(&site, &out, &verdict);
                let should_stop = record_failure(
                    ctx,
                    &change.resource_id,
                    &resource.resource_type,
                    duration,
                    &error,
                );
                return Ok(ResourceOutcome::Failed {
                    should_stop,
                    retryable: true,
                });
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
            let error = super::failure_text::exec_failure(&site, &out);
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
            Ok(ResourceOutcome::Failed {
                should_stop,
                retryable: true,
            })
        }
        Err(e) => {
            let error = super::failure_text::transport_failure(&e);
            let should_stop = record_failure(
                ctx,
                &change.resource_id,
                &resource.resource_type,
                duration,
                &error,
            );
            Ok(ResourceOutcome::Failed {
                should_stop,
                retryable: true,
            })
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

    crate::core::executor::log_tripwire(
        ctx.state_dir,
        ctx.machine_name,
        ctx.tripwire,
        ProvenanceEvent::ResourceStarted {
            machine: ctx.machine_name.to_string(),
            resource: change.resource_id.clone(),
            action: change.action.to_string(),
        },
    );

    let resolved = resolver::resolve_resource_templates_with_secrets(
        resource,
        &cfg.config.params,
        &cfg.config.machines,
        &cfg.config.secrets,
    )?;

    execute_resource(cfg, change, resource, &resolved, machine, ctx)
}
