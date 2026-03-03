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

    execute_machine_changes(
        cfg,
        &machine_changes,
        machine,
        &mut ctx,
        &mut trace_session,
        machine_name,
        &mut counters,
    )?;

    finalize_machine(
        cfg,
        ctx.lock,
        &mut trace_session,
        machine_name,
        &run_id,
        &machine_start,
        &counters,
        machine,
    )
}

/// Execute all resource changes for a machine (parallel waves or sequential).
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_machine_changes(
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
        execute_parallel_waves(
            cfg,
            machine_changes,
            machine,
            ctx,
            trace_session,
            machine_name,
            counters,
        )
    } else {
        execute_sequential(
            cfg,
            machine_changes,
            machine,
            ctx,
            trace_session,
            machine_name,
            counters,
        )
    }
}

/// Execute changes sequentially.
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_sequential(
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
            cfg,
            change,
            machine,
            ctx,
            trace_session,
            machine_name,
            &counters.converged_resources,
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
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_parallel_waves(
    cfg: &ApplyConfig,
    machine_changes: &[&PlannedChange],
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> Result<(), String> {
    let change_ids: Vec<&str> = machine_changes
        .iter()
        .map(|c| c.resource_id.as_str())
        .collect();
    let raw_waves = compute_resource_waves(cfg.config, &change_ids);
    let waves = split_waves_by_max_parallel(raw_waves, cfg.max_parallel);

    for wave in &waves {
        let should_stop = execute_single_wave(
            cfg,
            wave,
            machine_changes,
            machine,
            ctx,
            trace_session,
            machine_name,
            counters,
        )?;
        if should_stop {
            break;
        }
    }
    Ok(())
}

/// Execute a single wave — either sequentially (1 resource) or in parallel.
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_single_wave(
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
                cfg,
                change,
                machine,
                ctx,
                trace_session,
                machine_name,
                &counters.converged_resources,
            )?;
            return Ok(counters.record(&outcome, &change.resource_id));
        }
        Ok(false)
    } else {
        execute_wave_parallel(
            cfg,
            wave,
            machine_changes,
            machine,
            ctx,
            trace_session,
            machine_name,
            counters,
        )
    }
}

pub(super) use super::machine_b::*;
