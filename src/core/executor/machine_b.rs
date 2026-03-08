use super::machine::*;
use super::machine_wave::{execute_wave_io, record_wave_outcomes};
use super::*;

/// FJ-313: Split large waves to respect max_parallel constraint.
pub(super) fn split_waves_by_max_parallel(
    waves: Vec<Vec<String>>,
    max_parallel: Option<usize>,
) -> Vec<Vec<String>> {
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
#[allow(clippy::too_many_arguments)]
pub(super) fn finalize_machine(
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
            error: rl
                .details
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
        .collect()
}

/// Cleanup ephemeral container after apply if applicable.
fn cleanup_container_if_needed(cfg: &ApplyConfig, machine: &Machine, machine_name: &str) {
    if machine.is_container_transport() && !cfg.dry_run {
        if let Some(ref container) = machine.container {
            if container.ephemeral {
                if let Err(e) = transport::container::cleanup_container(machine) {
                    eprintln!("warning: container cleanup failed for {machine_name}: {e}");
                }
            }
        }
    }
}

/// Prepared resource for parallel wave execution.
pub(crate) struct PreparedResource {
    pub(crate) change_idx: usize,
    pub(crate) resolved: Resource,
    pub(crate) use_copia: bool,
}

/// FJ-257: Execute a multi-resource wave in parallel.
/// Returns true if jidoka should stop processing further waves.
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_wave_parallel(
    cfg: &ApplyConfig,
    wave: &[String],
    machine_changes: &[&PlannedChange],
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> Result<bool, String> {
    // FJ-63: Filter out resources whose dependencies have failed
    let mut active_ids: Vec<&String> = Vec::new();
    for id in wave {
        if let Some(resource) = cfg.config.resources.get(id) {
            if let Some(failed_dep) = counters.failed_dependency(&resource.depends_on) {
                eprintln!(
                    "JIDOKA: skipping {} — depends on failed '{}'",
                    id, failed_dep
                );
                counters.failed += 1;
                counters.failed_resources.insert(id.clone());
                continue;
            }
        }
        active_ids.push(id);
    }

    if active_ids.is_empty() {
        return Ok(false);
    }

    let wave_changes: Vec<&PlannedChange> = active_ids
        .iter()
        .filter_map(|id| {
            machine_changes
                .iter()
                .find(|c| c.resource_id == **id)
                .copied()
        })
        .collect();

    // Phase 1: Pre-check and prepare
    let (prepared, skipped_or_unchanged) = prepare_wave_resources(
        cfg,
        &wave_changes,
        machine,
        ctx,
        &counters.converged_resources,
    )?;

    // Phase 2: Execute transport I/O in parallel
    let exec_results = execute_wave_io(cfg, &prepared, machine);

    // Phase 3: Record outcomes
    record_wave_outcomes(
        cfg,
        &wave_changes,
        &skipped_or_unchanged,
        exec_results,
        &prepared,
        machine,
        ctx,
        trace_session,
        machine_name,
        counters,
    )
}

/// Phase 1: Filter and prepare resources for parallel execution.
#[allow(clippy::type_complexity)]
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
        if let Some(outcome) = classify_resource(cfg, change, machine, converged_resources) {
            skipped.push((idx, outcome));
            continue;
        }

        let Some(resource) = cfg.config.resources.get(&change.resource_id) else {
            skipped.push((idx, ResourceOutcome::Skipped));
            continue;
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
        let use_copia = resolved.resource_type == ResourceType::File
            && resolved
                .source
                .as_ref()
                .map(|s| copia::is_eligible(s))
                .unwrap_or(false);
        prepared.push(PreparedResource {
            change_idx: idx,
            resolved,
            use_copia,
        });
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
        && resource
            .triggers
            .iter()
            .any(|t| converged_resources.contains(t));
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
