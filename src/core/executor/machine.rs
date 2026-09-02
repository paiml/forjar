//! Per-machine apply logic: setup, wave execution, finalization.

use super::*;

/// Counters for tracking apply outcomes within a machine.
pub(crate) struct MachineCounters {
    pub converged: u32,
    pub unchanged: u32,
    pub failed: u32,
    pub converged_resources: HashSet<String>,
    /// FJ-63: Track failed resource IDs for dependency-cascade skipping.
    pub failed_resources: HashSet<String>,
}

impl MachineCounters {
    fn new() -> Self {
        Self {
            converged: 0,
            unchanged: 0,
            failed: 0,
            converged_resources: HashSet::new(),
            failed_resources: HashSet::new(),
        }
    }

    /// FJ-63: Check if a resource should be skipped because it depends on a failed resource.
    /// Returns the name of the failed dependency if found.
    pub(crate) fn failed_dependency<'a>(&self, depends_on: &'a [String]) -> Option<&'a str> {
        depends_on.iter().find_map(|dep| {
            if self.failed_resources.contains(dep) {
                Some(dep.as_str())
            } else {
                None
            }
        })
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
    // Dogfood #208 (family #212, logs-run-id-disagrees-with-history-audit-state-query):
    // the run id MUST be minted once per apply invocation and threaded to every
    // writer. Previously the event/audit stream minted its own id here while the
    // run-log directory used `cfg.run_id`, so one apply produced two ids and a run
    // id copied out of `history`/`audit` was unusable with `logs --run`.
    let run_id = cfg.run_id.clone().unwrap_or_else(eventlog::generate_run_id);

    // Container lifecycle: ensure container is running before apply
    if machine.is_container_transport() && !cfg.dry_run {
        transport::container::ensure_container(machine)?;
    }

    // FJ-252: Start SSH ControlMaster for connection multiplexing.
    //
    // forjar#404: `Ok(_) => true` claimed ownership of a master this frame did
    // NOT open — `start_control_master` returns `Ok(false)` for "one is already
    // running". Since the E02 fix opens the fleet's masters in `cmd_apply`
    // (before the drift gate), that stale `true` made the FIRST machine to
    // finish tear down a socket the run-level guard still owns and later
    // machines, hooks and the readback would then have to re-handshake for.
    // Only stop what this frame started.
    let ssh_mux = if !cfg.dry_run && transport::is_ssh_transport(machine) {
        match transport::ssh::start_control_master(machine) {
            Ok(started) => started,
            Err(e) => {
                eprintln!("warning: SSH multiplexing failed for {machine_name}: {e}");
                false
            }
        }
    } else {
        false
    };

    let mut lock = locks
        .remove(machine_name)
        .unwrap_or_else(|| state::new_lock(machine_name, &machine.hostname));

    let mut trace_session = tracer::TraceSession::start(&run_id);

    // FJ-2002: Compute config hash for provenance tracking
    // GH-212: the audit trail's config_hash must be reproducible. The plain
    // serialisation ordered `HashMap` fields by iteration order, so two runs
    // over the same file recorded different provenance hashes.
    let config_hash = crate::core::config_hash::config_hash(cfg.config).ok();

    log_tripwire(
        cfg.state_dir,
        machine_name,
        cfg.config.policy.tripwire,
        ProvenanceEvent::ApplyStarted {
            machine: machine_name.to_string(),
            run_id: run_id.clone(),
            forjar_version: env!("CARGO_PKG_VERSION").to_string(),
            operator: Some(get_operator_identity()),
            config_hash,
            param_count: Some(cfg.config.params.len() as u32),
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

    let result = execute_machine_changes(
        cfg,
        &machine_changes,
        machine,
        &mut ctx,
        &mut trace_session,
        machine_name,
        &mut counters,
    );

    // FJ-252: Tear down SSH ControlMaster after apply completes
    if ssh_mux {
        let _ = transport::ssh::stop_control_master(machine);
    }

    result?;

    // plan-apply-equivalence-v1 contract: the executor records at most one
    // outcome per planned change — it never executes an action the plan did
    // not predict (filters, cascades, and jidoka stops may record fewer).
    debug_assert!(
        (counters.converged + counters.unchanged + counters.failed) as usize
            <= machine_changes.len(),
        "PLAN-APPLY-EQUIVALENCE violated: outcomes ({}) exceed planned changes ({})",
        counters.converged + counters.unchanged + counters.failed,
        machine_changes.len()
    );

    finalize_machine(
        cfg,
        ctx.lock,
        &mut trace_session,
        machine_name,
        &run_id,
        &machine_start,
        &counters,
    )
}

/// Execute all resource changes for a machine.
///
/// Refs #412 (CRUX audit E09): there is ONE scheduler. "Sequential" is not a
/// second implementation, it is this scheduler running a schedule whose every
/// wave has width 1 — so a feature added to the wave path is a feature the
/// sequential path has, and the two cannot drift apart again.
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
    let waves = schedule_waves(cfg, machine_changes);
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

/// The schedule: dependency waves when parallel, width 1 in PLAN ORDER when not.
///
/// The width-1 schedule deliberately does NOT go through
/// `compute_resource_waves`: that sorts each wave alphabetically, which would
/// reorder the console output, the event stream and the run log of every
/// existing sequential apply. Plan order is already topological.
pub(super) fn schedule_waves(
    cfg: &ApplyConfig,
    machine_changes: &[&PlannedChange],
) -> Vec<Vec<String>> {
    let use_parallel = cfg.parallel.unwrap_or(cfg.config.policy.parallel_resources);
    if !use_parallel || machine_changes.len() <= 1 {
        return machine_changes
            .iter()
            .map(|c| vec![c.resource_id.clone()])
            .collect();
    }
    let change_ids: Vec<&str> = machine_changes
        .iter()
        .map(|c| c.resource_id.as_str())
        .collect();
    let raw_waves = compute_resource_waves(cfg.config, &change_ids);
    split_waves_by_max_parallel(raw_waves, cfg.max_parallel)
}

/// Execute one wave of the schedule. Refs #412 (CRUX audit E09): there is
/// no width-1 special case here on purpose — a one-resource wave is the wave
/// scheduler running one resource, so `--retry`, `--trace`, `--progress`, the
/// input cache, hook timing and failure attribution are the same code at
/// every width. The old single-resource path (`apply_and_record_outcome`)
/// was the second implementation the ticket retires.
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

/// FJ-272: the `[n/total] id` prefix, position taken from the PLAN order so it
/// is the same number whichever wave the resource landed in.
pub(super) fn progress_prefix(machine_changes: &[&PlannedChange], resource_id: &str) -> String {
    let pos = machine_changes
        .iter()
        .position(|c| c.resource_id == resource_id)
        .map(|i| i + 1)
        .unwrap_or(0);
    format!("[{}/{}] {}", pos, machine_changes.len(), resource_id)
}

/// The single word `--progress` prints once a resource has an outcome.
pub(super) fn progress_word(outcome: &ResourceOutcome) -> &'static str {
    match outcome {
        ResourceOutcome::Converged => "converged",
        ResourceOutcome::Unchanged => "unchanged",
        ResourceOutcome::Skipped => "skipped",
        ResourceOutcome::Failed => "FAILED",
    }
}

pub(super) use super::machine_b::*;

/// FJ-1391: Get operator identity for drift forensics attribution.
fn get_operator_identity() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let host = std::fs::read_to_string("/etc/hostname")
        .map(|h| h.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    format!("{user}@{host}")
}
