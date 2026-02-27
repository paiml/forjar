//! Per-machine apply logic: setup, wave execution, finalization.

use super::*;

/// Counters for tracking apply outcomes within a machine.
pub(crate) struct MachineCounters {
    pub converged: u32,
    pub unchanged: u32,
    pub failed: u32,
    pub converged_resources: HashSet<String>,
}

impl MachineCounters {
    fn new() -> Self {
        Self {
            converged: 0,
            unchanged: 0,
            failed: 0,
            converged_resources: HashSet::new(),
        }
    }

    fn record(&mut self, outcome: &ResourceOutcome, resource_id: &str) -> bool {
        match outcome {
            ResourceOutcome::Converged => {
                self.converged += 1;
                self.converged_resources.insert(resource_id.to_string());
                false
            }
            ResourceOutcome::Unchanged => {
                self.unchanged += 1;
                false
            }
            ResourceOutcome::Skipped => false,
            ResourceOutcome::Failed { should_stop } => {
                self.failed += 1;
                *should_stop
            }
        }
    }
}

pub(crate) fn apply_machine(
    cfg: &ApplyConfig,
    machine_name: &str,
    machine: &Machine,
    plan: &ExecutionPlan,
    locks: &mut HashMap<String, StateLock>,
) -> Result<ApplyResult, String> {
    let machine_start = Instant::now();
    let run_id = eventlog::generate_run_id();

    // Container lifecycle: ensure container is running before apply
    if machine.is_container_transport() && !cfg.dry_run {
        transport::container::ensure_container(machine)?;
    }

    let mut lock = locks
        .remove(machine_name)
        .unwrap_or_else(|| state::new_lock(machine_name, &machine.hostname));

    let mut trace_session = tracer::TraceSession::start(&run_id);

    log_tripwire(
        cfg.state_dir,
        machine_name,
        cfg.config.policy.tripwire,
        ProvenanceEvent::ApplyStarted {
            machine: machine_name.to_string(),
            run_id: run_id.clone(),
            forjar_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    );

    let mut counters = MachineCounters::new();

    let machine_changes: Vec<_> = plan
        .changes
        .iter()
        .filter(|c| c.machine == machine_name)
        .collect();

    let mut ctx = RecordCtx {
        lock: &mut lock,
        state_dir: cfg.state_dir,
        machine_name,
        tripwire: cfg.config.policy.tripwire,
        failure_policy: &cfg.config.policy.failure,
        timeout_secs: cfg.resource_timeout.or(cfg.timeout_secs),
    };

    execute_machine_changes(cfg, &machine_changes, machine, &mut ctx, &mut trace_session, machine_name, &mut counters)?;

    finalize_machine(cfg, ctx.lock, &mut trace_session, machine_name, &run_id, &machine_start, &counters, machine)
}

/// Execute all resource changes for a machine (parallel waves or sequential).
fn execute_machine_changes(
    cfg: &ApplyConfig,
    machine_changes: &[&PlannedChange],
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> Result<(), String> {
    let use_parallel = cfg.parallel.unwrap_or(cfg.config.policy.parallel_resources);
    if use_parallel && machine_changes.len() > 1 {
        execute_parallel_waves(cfg, machine_changes, machine, ctx, trace_session, machine_name, counters)
    } else {
        execute_sequential(cfg, machine_changes, machine, ctx, trace_session, machine_name, counters)
    }
}

/// Execute changes sequentially.
fn execute_sequential(
    cfg: &ApplyConfig,
    machine_changes: &[&PlannedChange],
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> Result<(), String> {
    let total = machine_changes.len();
    for (idx, change) in machine_changes.iter().enumerate() {
        if cfg.progress {
            eprint!("[{}/{}] {} ", idx + 1, total, change.resource_id);
        }
        let outcome = apply_and_record_outcome(
            cfg, change, machine, ctx, trace_session, machine_name, &counters.converged_resources,
        )?;
        if cfg.progress {
            match &outcome {
                ResourceOutcome::Converged => eprintln!("converged"),
                ResourceOutcome::Unchanged => eprintln!("unchanged"),
                ResourceOutcome::Skipped => eprintln!("skipped"),
                ResourceOutcome::Failed { .. } => eprintln!("FAILED"),
            }
        }
        if counters.record(&outcome, &change.resource_id) {
            break;
        }
    }
    Ok(())
}

/// Execute changes in parallel waves with dependency ordering.
fn execute_parallel_waves(
    cfg: &ApplyConfig,
    machine_changes: &[&PlannedChange],
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> Result<(), String> {
    let change_ids: Vec<&str> = machine_changes.iter().map(|c| c.resource_id.as_str()).collect();
    let raw_waves = compute_resource_waves(cfg.config, &change_ids);
    let waves = split_waves_by_max_parallel(raw_waves, cfg.max_parallel);

    for wave in &waves {
        let should_stop = execute_single_wave(cfg, wave, machine_changes, machine, ctx, trace_session, machine_name, counters)?;
        if should_stop {
            break;
        }
    }
    Ok(())
}

/// Execute a single wave — either sequentially (1 resource) or in parallel.
fn execute_single_wave(
    cfg: &ApplyConfig,
    wave: &[String],
    machine_changes: &[&PlannedChange],
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> Result<bool, String> {
    if wave.len() == 1 {
        if let Some(change) = machine_changes.iter().find(|c| c.resource_id == wave[0]) {
            let outcome = apply_and_record_outcome(
                cfg, change, machine, ctx, trace_session, machine_name, &counters.converged_resources,
            )?;
            return Ok(counters.record(&outcome, &change.resource_id));
        }
        Ok(false)
    } else {
        execute_wave_parallel(cfg, wave, machine_changes, machine, ctx, trace_session, machine_name, counters)
    }
}

/// FJ-313: Split large waves to respect max_parallel constraint.
fn split_waves_by_max_parallel(waves: Vec<Vec<String>>, max_parallel: Option<usize>) -> Vec<Vec<String>> {
    match max_parallel {
        Some(max_p) => waves
            .into_iter()
            .flat_map(|wave| {
                if wave.len() <= max_p {
                    vec![wave]
                } else {
                    wave.chunks(max_p).map(|chunk| chunk.to_vec()).collect()
                }
            })
            .collect(),
        None => waves,
    }
}

/// Finalize a machine apply: save lock, write trace, log completion, build result.
fn finalize_machine(
    cfg: &ApplyConfig,
    lock: &mut StateLock,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    run_id: &str,
    machine_start: &Instant,
    counters: &MachineCounters,
    machine: &Machine,
) -> Result<ApplyResult, String> {
    lock.generated_at = eventlog::now_iso8601();
    if cfg.config.policy.lock_file {
        state::save_lock(cfg.state_dir, lock)?;
    }

    if cfg.config.policy.tripwire {
        let _root_span = trace_session.finalize();
        let _ = tracer::write_trace(cfg.state_dir, machine_name, trace_session);
    }

    log_tripwire(
        cfg.state_dir,
        machine_name,
        cfg.config.policy.tripwire,
        ProvenanceEvent::ApplyCompleted {
            machine: machine_name.to_string(),
            run_id: run_id.to_string(),
            resources_converged: counters.converged,
            resources_unchanged: counters.unchanged,
            resources_failed: counters.failed,
            total_seconds: machine_start.elapsed().as_secs_f64(),
        },
    );

    let resource_reports = build_resource_reports(lock);

    let result = ApplyResult {
        machine: machine_name.to_string(),
        resources_converged: counters.converged,
        resources_unchanged: counters.unchanged,
        resources_failed: counters.failed,
        total_duration: machine_start.elapsed(),
        resource_reports,
    };

    // Container lifecycle: cleanup ephemeral containers after apply
    cleanup_container_if_needed(cfg, machine, machine_name);

    Ok(result)
}

/// Build per-resource reports from lock state.
fn build_resource_reports(lock: &StateLock) -> Vec<ResourceReport> {
    lock.resources
        .iter()
        .map(|(id, rl)| ResourceReport {
            resource_id: id.clone(),
            resource_type: format!("{:?}", rl.resource_type).to_lowercase(),
            status: format!("{:?}", rl.status).to_lowercase(),
            duration_seconds: rl.duration_seconds.unwrap_or(0.0),
            exit_code: None,
            hash: if rl.hash.is_empty() {
                None
            } else {
                Some(rl.hash.clone())
            },
            error: rl.details.get("error").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
        .collect()
}

/// Cleanup ephemeral container after apply if applicable.
fn cleanup_container_if_needed(cfg: &ApplyConfig, machine: &Machine, machine_name: &str) {
    if machine.is_container_transport() && !cfg.dry_run {
        if let Some(ref container) = machine.container {
            if container.ephemeral {
                if let Err(e) = transport::container::cleanup_container(machine) {
                    eprintln!("warning: container cleanup failed for {}: {}", machine_name, e);
                }
            }
        }
    }
}

/// Prepared resource for parallel wave execution.
struct PreparedResource {
    change_idx: usize,
    resolved: Resource,
    use_copia: bool,
}

/// FJ-257: Execute a multi-resource wave in parallel.
/// Returns true if jidoka should stop processing further waves.
#[allow(clippy::too_many_arguments)]
fn execute_wave_parallel(
    cfg: &ApplyConfig,
    wave: &[String],
    machine_changes: &[&PlannedChange],
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> Result<bool, String> {
    let wave_changes: Vec<&PlannedChange> = wave
        .iter()
        .filter_map(|id| machine_changes.iter().find(|c| c.resource_id == *id).copied())
        .collect();

    // Phase 1: Pre-check and prepare
    let (prepared, skipped_or_unchanged) = prepare_wave_resources(cfg, &wave_changes, machine, ctx, &counters.converged_resources)?;

    // Phase 2: Execute transport I/O in parallel
    let exec_results = execute_wave_io(cfg, &prepared, machine);

    // Phase 3: Record outcomes
    record_wave_outcomes(cfg, &wave_changes, &skipped_or_unchanged, exec_results, &prepared, machine, ctx, trace_session, machine_name, counters)
}

/// Phase 1: Filter and prepare resources for parallel execution.
fn prepare_wave_resources(
    cfg: &ApplyConfig,
    wave_changes: &[&PlannedChange],
    machine: &Machine,
    ctx: &mut RecordCtx,
    converged_resources: &HashSet<String>,
) -> Result<(Vec<PreparedResource>, Vec<(usize, ResourceOutcome)>), String> {
    let mut prepared = Vec::new();
    let mut skipped = Vec::new();

    for (idx, change) in wave_changes.iter().enumerate() {
        match classify_resource(cfg, change, machine, converged_resources) {
            Some(outcome) => {
                skipped.push((idx, outcome));
                continue;
            }
            None => {}
        }

        let resource = cfg.config.resources.get(&change.resource_id).unwrap();

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

        let resolved = resolver::resolve_resource_templates(resource, &cfg.config.params, &cfg.config.machines)?;
        let use_copia = resolved.resource_type == ResourceType::File
            && resolved.source.is_some()
            && copia::is_eligible(resolved.source.as_ref().unwrap());
        prepared.push(PreparedResource { change_idx: idx, resolved, use_copia });
    }

    Ok((prepared, skipped))
}

/// Classify whether a resource should be skipped/unchanged before execution.
fn classify_resource(
    cfg: &ApplyConfig,
    change: &PlannedChange,
    machine: &Machine,
    converged_resources: &HashSet<String>,
) -> Option<ResourceOutcome> {
    if let Some(filter) = cfg.resource_filter {
        if change.resource_id != filter {
            return Some(ResourceOutcome::Skipped);
        }
    }
    let resource = cfg.config.resources.get(&change.resource_id)?;

    let triggered = !resource.triggers.is_empty()
        && resource.triggers.iter().any(|t| converged_resources.contains(t));
    if change.action == PlanAction::NoOp && !cfg.force && !triggered {
        return Some(ResourceOutcome::Unchanged);
    }
    if should_skip_resource(cfg, resource, machine) {
        return Some(ResourceOutcome::Skipped);
    }
    None
}

/// Check filter/arch/tag/group/when conditions that skip a resource.
fn should_skip_resource(cfg: &ApplyConfig, resource: &Resource, machine: &Machine) -> bool {
    if !resource.arch.is_empty() && !resource.arch.contains(&machine.arch) {
        return true;
    }
    if let Some(tag) = cfg.tag_filter {
        if !resource.tags.iter().any(|t| t == tag) {
            return true;
        }
    }
    if let Some(group) = cfg.group_filter {
        if resource.resource_group.as_deref() != Some(group) {
            return true;
        }
    }
    if let Some(ref when_expr) = resource.when {
        return !conditions::evaluate_when(when_expr, &cfg.config.params, machine).unwrap_or(false);
    }
    false
}

/// Phase 2: Execute transport I/O in parallel threads.
fn execute_wave_io(
    cfg: &ApplyConfig,
    prepared: &[PreparedResource],
    machine: &Machine,
) -> Vec<(usize, f64, Result<transport::ExecOutput, String>)> {
    let ssh_retries = cfg.config.policy.ssh_retries;
    std::thread::scope(|s| {
        let handles: Vec<_> = prepared
            .iter()
            .map(|prep| {
                s.spawn(move || {
                    let start = Instant::now();
                    if let Some(ref pre_hook) = prep.resolved.pre_apply {
                        if let Some(err) = run_pre_hook(machine, pre_hook, cfg.timeout_secs) {
                            return (prep.change_idx, start.elapsed().as_secs_f64(), Err(err));
                        }
                    }
                    let output = if prep.use_copia {
                        copia_apply_file(machine, &prep.resolved, cfg.timeout_secs)
                    } else {
                        codegen::apply_script(&prep.resolved).and_then(|script| {
                            transport::exec_script_retry(machine, &script, cfg.timeout_secs, ssh_retries)
                        })
                    };
                    let output = run_post_hook_if_success(output, &prep.resolved, machine, cfg.timeout_secs);
                    (prep.change_idx, start.elapsed().as_secs_f64(), output)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })
}

/// Run pre_apply hook, returning error string on failure.
fn run_pre_hook(machine: &Machine, hook: &str, timeout: Option<u64>) -> Option<String> {
    match transport::exec_script_timeout(machine, hook, timeout) {
        Ok(out) if !out.success() => Some(format!(
            "pre_apply hook failed (exit {}): {}",
            out.exit_code,
            out.stderr.trim()
        )),
        Err(e) => Some(format!("pre_apply hook error: {}", e)),
        _ => None,
    }
}

/// Run post_apply hook after successful execution.
fn run_post_hook_if_success(
    output: Result<transport::ExecOutput, String>,
    resolved: &Resource,
    machine: &Machine,
    timeout: Option<u64>,
) -> Result<transport::ExecOutput, String> {
    match output {
        Ok(ref out) if out.success() => {
            if let Some(ref post_hook) = resolved.post_apply {
                match transport::exec_script_timeout(machine, post_hook, timeout) {
                    Ok(pout) if !pout.success() => Err(format!(
                        "post_apply hook failed (exit {}): {}",
                        pout.exit_code,
                        pout.stderr.trim()
                    )),
                    Err(e) => Err(format!("post_apply hook error: {}", e)),
                    _ => output,
                }
            } else {
                output
            }
        }
        _ => output,
    }
}

/// Phase 3: Record wave outcomes sequentially.
#[allow(clippy::too_many_arguments)]
fn record_wave_outcomes(
    cfg: &ApplyConfig,
    wave_changes: &[&PlannedChange],
    skipped_or_unchanged: &[(usize, ResourceOutcome)],
    exec_results: Vec<(usize, f64, Result<transport::ExecOutput, String>)>,
    prepared: &[PreparedResource],
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> Result<bool, String> {
    // Record skipped/unchanged
    for (idx, outcome) in skipped_or_unchanged {
        let change = wave_changes[*idx];
        let resource_rt = resource_type_label(cfg, &change.resource_id);
        if let ResourceOutcome::Unchanged = outcome {
            counters.unchanged += 1;
            trace_session.record_noop(&change.resource_id, &resource_rt, machine_name);
        }
    }

    // Record executed resources
    let mut stop = false;
    for (idx, duration, output) in exec_results {
        let change = wave_changes[idx];
        let resource = cfg.config.resources.get(&change.resource_id).unwrap();
        let prep = prepared.iter().find(|p| p.change_idx == idx).unwrap();

        match output {
            Ok(out) if out.success() => {
                record_success(ctx, &change.resource_id, resource, &prep.resolved, machine, duration);
                counters.converged += 1;
                counters.converged_resources.insert(change.resource_id.clone());
                let rt = resource_type_label(cfg, &change.resource_id);
                let action = if change.action == PlanAction::Create { "create" } else { "update" };
                trace_session.record_span(&change.resource_id, &rt, machine_name, action, std::time::Duration::from_secs_f64(duration), 0, None);
            }
            Ok(out) => {
                let error = format!("exit code {}: {}", out.exit_code, out.stderr.trim());
                stop |= record_failure(ctx, &change.resource_id, &resource.resource_type, duration, &error);
                counters.failed += 1;
                let rt = resource_type_label(cfg, &change.resource_id);
                trace_session.record_span(&change.resource_id, &rt, machine_name, "create", std::time::Duration::from_secs_f64(duration), 1, None);
            }
            Err(e) => {
                let error = format!("transport error: {}", e);
                stop |= record_failure(ctx, &change.resource_id, &resource.resource_type, duration, &error);
                counters.failed += 1;
                let rt = resource_type_label(cfg, &change.resource_id);
                trace_session.record_span(&change.resource_id, &rt, machine_name, "create", std::time::Duration::from_secs_f64(duration), 1, None);
            }
        }
    }
    Ok(stop)
}

/// Get lowercase resource type label for a resource ID.
fn resource_type_label(cfg: &ApplyConfig, resource_id: &str) -> String {
    cfg.config
        .resources
        .get(resource_id)
        .map(|r| format!("{:?}", r.resource_type))
        .unwrap_or_default()
        .to_lowercase()
}
