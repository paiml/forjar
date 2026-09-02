//! Wave execution: the record phase of the one scheduler.
//!
//! Refs #412: split out of `machine_wave.rs` when the wide-wave path gained the
//! run-log metadata and `--progress` output that only the width-1 path had, so
//! neither file carries two responsibilities (and neither approaches the 500-line
//! ceiling).

use super::machine::{MachineCounters, PreparedResource};
use super::machine_wave::WaveResult;
use super::*;

/// Everything the record phase needs about the wave being recorded.
///
/// Bundled rather than passed as six more parameters: `record_wave_outcomes`
/// already carried ten, and the plan-order slice needed for `--progress` would
/// have made eleven.
pub(super) struct WaveRecord<'a> {
    /// The changes of THIS wave, indexed by the `change_idx` in each result.
    pub wave_changes: &'a [&'a PlannedChange],
    /// Every change planned for the machine, in PLAN order — the denominator
    /// and the position `--progress` reports.
    pub machine_changes: &'a [&'a PlannedChange],
    pub skipped_or_unchanged: &'a [(usize, ResourceOutcome)],
    pub prepared: &'a [PreparedResource],
}

/// Phase 3: Record wave outcomes sequentially.
#[allow(clippy::too_many_arguments)]
pub(super) fn record_wave_outcomes(
    cfg: &ApplyConfig,
    wave: &WaveRecord,
    exec_results: Vec<WaveResult>,
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> Result<bool, String> {
    record_skipped(cfg, wave, trace_session, machine_name, counters);

    for (idx, duration, output, executed_script) in exec_results {
        let change = wave.wave_changes[idx];
        let Some(resource) = cfg.config.resources.get(&change.resource_id) else {
            continue;
        };
        let Some(prep) = wave.prepared.iter().find(|p| p.change_idx == idx) else {
            continue;
        };
        let outcome = record_one(
            cfg,
            &Recorded {
                change,
                resource,
                prep,
                duration,
                output,
                executed_script: &executed_script,
            },
            machine,
            ctx,
            trace_session,
            machine_name,
            counters,
        );
        if cfg.progress {
            eprintln!(
                "{} {}",
                super::machine::progress_prefix(wave.machine_changes, &change.resource_id),
                super::machine::progress_word(&outcome)
            );
        }
    }
    // FJ-63: Never stop between waves — dependency skipping handles cascade
    Ok(false)
}

/// The resources this wave never executed: filtered out, or already converged.
fn record_skipped(
    cfg: &ApplyConfig,
    wave: &WaveRecord,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) {
    for (idx, outcome) in wave.skipped_or_unchanged {
        let change = wave.wave_changes[*idx];
        let resource_rt = resource_type_label(cfg, &change.resource_id);
        if let ResourceOutcome::Unchanged = outcome {
            counters.unchanged += 1;
            trace_session.record_noop(&change.resource_id, &resource_rt, machine_name);
        }
        if cfg.progress {
            eprintln!(
                "{} {}",
                super::machine::progress_prefix(wave.machine_changes, &change.resource_id),
                super::machine::progress_word(outcome)
            );
        }
    }
}

/// One executed resource and everything the record phase learned about it.
struct Recorded<'a> {
    change: &'a PlannedChange,
    resource: &'a Resource,
    prep: &'a PreparedResource,
    duration: f64,
    output: Result<transport::ExecOutput, String>,
    executed_script: &'a str,
}

#[allow(clippy::too_many_arguments)]
fn record_one(
    cfg: &ApplyConfig,
    rec: &Recorded,
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> ResourceOutcome {
    let change = rec.change;
    let duration = rec.duration;

    // Refs #390-A: PERSIST THE TRANSCRIPT, exactly as the width-1 path does.
    // This call is the whole fix, and it could not exist before because
    // the script was built and dropped inside the spawn closure so nothing
    // out here had it. Under `--parallel` a failing task's stdout was
    // DESTROYED rather than merely hidden, which is why #390's reporter could
    // not find their diagnostics "anywhere in the full raw apply log".
    let action = format!("{:?}", change.action).to_lowercase();
    let slot = run_capture::RunSlot::new(ctx.state_dir, ctx.machine_name, cfg.run_id.as_deref());
    let executed = run_capture::Executed {
        resource_id: &change.resource_id,
        resource_type: &rec.resource.resource_type,
        action: &action,
        script: rec.executed_script,
        // Refs #406: the SAME redaction policy at every width, from the same
        // UNRESOLVED declaration — under a wide wave this transcript is the
        // ONLY copy of a failing task's output, so it is written and cleaned here.
        transcript: run_capture::Transcript::for_resource(rec.resource, &cfg.config.secrets),
    };
    let log = rec
        .output
        .as_ref()
        .ok()
        .and_then(|out| run_capture::capture_exec_output(&slot, &executed, out, duration));

    // ONE failure text for every wave width, so the messages cannot drift
    // apart. `log` is a real path here now, so the message names the transcript
    // instead of reporting that none was written.
    let site = super::failure_text::Site {
        resource_id: &change.resource_id,
        state_dir: ctx.state_dir,
        run_id: cfg.run_id.as_deref(),
        log: log.as_deref(),
        resolved: &rec.prep.resolved,
    };

    match &rec.output {
        // Refs #390-B: ASK THE HOST, exactly as the width-1 path does.
        //
        // This arm went straight to `record_success`, so FJ-2731 (declared
        // output_artifacts) and FJ-2732 (the host reports the declared state)
        // silently did not run under `--parallel`. Two configs identical but
        // for `policy.parallel_resources` could report converged and failed.
        Ok(out) if out.success() => {
            match super::output_verify::post_apply_failure(
                &rec.prep.resolved,
                machine,
                ctx.timeout_secs,
            ) {
                Some(verdict) => {
                    let error = super::failure_text::verify_failure(&site, out, &verdict);
                    fail(cfg, rec, ctx, trace_session, machine_name, counters, &error)
                }
                None => converge(
                    cfg,
                    rec,
                    machine,
                    ctx,
                    trace_session,
                    machine_name,
                    counters,
                ),
            }
        }
        Ok(out) => {
            let error = super::failure_text::exec_failure(&site, out);
            let outcome = fail(cfg, rec, ctx, trace_session, machine_name, counters, &error);
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
            outcome
        }
        Err(e) => {
            let error = super::failure_text::transport_failure(e);
            fail(cfg, rec, ctx, trace_session, machine_name, counters, &error)
        }
    }
}

/// Record a converged resource: lock, event, run metadata, trace span, counter.
#[allow(clippy::too_many_arguments)]
fn converge(
    cfg: &ApplyConfig,
    rec: &Recorded,
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
) -> ResourceOutcome {
    let change = rec.change;
    record_success(
        ctx,
        &change.resource_id,
        rec.resource,
        &rec.prep.resolved,
        machine,
        rec.duration,
    );
    // FJ-2301: the run's meta.yaml, which only the width-1 path used to write.
    // `forjar logs --run` and every `--json` consumer read it, so a wide wave
    // produced a run directory whose `resources:` map was empty.
    update_run_meta(
        ctx,
        cfg.run_id.as_deref(),
        &change.resource_id,
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(rec.duration),
            failed: false,
        },
    );
    counters.converged += 1;
    counters
        .converged_resources
        .insert(change.resource_id.clone());
    let action = if change.action == PlanAction::Create {
        "create"
    } else {
        "update"
    };
    trace_session.record_span(
        &change.resource_id,
        &resource_type_label(cfg, &change.resource_id),
        machine_name,
        action,
        std::time::Duration::from_secs_f64(rec.duration),
        0,
        None,
    );
    ResourceOutcome::Converged
}

/// Record a failed resource: lock, event, trace span, counters.
#[allow(clippy::too_many_arguments)]
fn fail(
    cfg: &ApplyConfig,
    rec: &Recorded,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    counters: &mut MachineCounters,
    error: &str,
) -> ResourceOutcome {
    let change = rec.change;
    record_failure(
        ctx,
        &change.resource_id,
        &rec.resource.resource_type,
        rec.duration,
        error,
    );
    counters.failed += 1;
    counters.failed_resources.insert(change.resource_id.clone());
    // The width-1 path labels a verified-but-failed resource "update" and a
    // failure that never converged "create"; keep both labels here.
    let action = if matches!(&rec.output, Ok(out) if out.success()) {
        "update"
    } else {
        "create"
    };
    trace_session.record_span(
        &change.resource_id,
        &resource_type_label(cfg, &change.resource_id),
        machine_name,
        action,
        std::time::Duration::from_secs_f64(rec.duration),
        1,
        None,
    );
    ResourceOutcome::Failed
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
