//! Wave execution helpers for per-machine apply.

use super::machine::{MachineCounters, PreparedResource};
use super::*;

/// Phase 2: Execute transport I/O in parallel threads.
pub(super) fn execute_wave_io(
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
                            transport::exec_script_retry(
                                machine,
                                &script,
                                cfg.timeout_secs,
                                ssh_retries,
                            )
                        })
                    };
                    let output =
                        run_post_hook_if_success(output, &prep.resolved, machine, cfg.timeout_secs);
                    (prep.change_idx, start.elapsed().as_secs_f64(), output)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| match h.join() {
                Ok(result) => result,
                Err(panic_payload) => {
                    let msg = extract_panic_message(panic_payload);
                    eprintln!("error: wave execution thread panicked: {msg}");
                    (0, 0.0, Err(format!("thread panic: {msg}")))
                }
            })
            .collect()
    })
}

/// Extract a human-readable message from a thread panic payload.
fn extract_panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "thread panicked".to_string()
    }
}

/// Run pre_apply hook, returning error string on failure.
/// I8 invariant: hook script is validated via bashrs before execution.
fn run_pre_hook(machine: &Machine, hook: &str, timeout: Option<u64>) -> Option<String> {
    exec_validated_hook(machine, hook, timeout, "pre_apply").err()
}

/// Execute a validated hook script, returning error on failure.
fn exec_validated_hook(
    machine: &Machine,
    hook: &str,
    timeout: Option<u64>,
    label: &str,
) -> Result<(), String> {
    if let Err(e) = crate::core::purifier::validate_script(hook) {
        return Err(format!("{label} hook failed I8 validation: {e}"));
    }
    // Refs #390: the third byte-identical copy of the hook-failure string, all
    // three stderr-only. A hook that says `echo "nginx config invalid: line 42"`
    // without `>&2` — which is what people actually write — lost its diagnostic
    // exactly the way the reporter's task did.
    match transport::exec_script_timeout(machine, hook, timeout) {
        Ok(out) if !out.success() => Err(super::failure_text::hook_failure(label, &out)),
        Err(e) => Err(super::failure_text::hook_error(label, &e)),
        _ => Ok(()),
    }
}

/// Run post_apply hook after successful execution.
/// I8 invariant: hook script is validated via bashrs before execution.
fn run_post_hook_if_success(
    output: Result<transport::ExecOutput, String>,
    resolved: &Resource,
    machine: &Machine,
    timeout: Option<u64>,
) -> Result<transport::ExecOutput, String> {
    let Ok(ref out) = output else {
        return output;
    };
    if !out.success() {
        return output;
    }
    let Some(ref post_hook) = resolved.post_apply else {
        return output;
    };
    exec_validated_hook(machine, post_hook, timeout, "post_apply")?;
    output
}

/// Phase 3: Record wave outcomes sequentially.
#[allow(clippy::too_many_arguments)]
pub(super) fn record_wave_outcomes(
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
    for (idx, duration, output) in exec_results {
        let change = wave_changes[idx];
        let Some(resource) = cfg.config.resources.get(&change.resource_id) else {
            continue;
        };
        let Some(prep) = prepared.iter().find(|p| p.change_idx == idx) else {
            continue;
        };

        // Refs #390: ONE failure text for both schedulers, so the sequential
        // and the parallel message cannot drift apart the way the two copies of
        // `format!("exit code {}: {}", ...)` did.
        //
        // `log: None` is the honest answer here and not an oversight. This path
        // never calls `run_capture`, so under `--parallel` no transcript exists
        // at all and naming one would send an operator to a file that was never
        // created — the second wrong turn after the one #390 already took.
        // Tracked as #390-A; until it lands this message IS the only copy of a
        // failed wave resource's output, which is why the excerpt below is not
        // merely a convenience.
        let site = super::failure_text::Site {
            resource_id: &change.resource_id,
            state_dir: ctx.state_dir,
            run_id: cfg.run_id.as_deref(),
            log: None,
            resolved: &prep.resolved,
        };

        match output {
            Ok(out) if out.success() => {
                record_success(
                    ctx,
                    &change.resource_id,
                    resource,
                    &prep.resolved,
                    machine,
                    duration,
                );
                counters.converged += 1;
                counters
                    .converged_resources
                    .insert(change.resource_id.clone());
                let rt = resource_type_label(cfg, &change.resource_id);
                let action = if change.action == PlanAction::Create {
                    "create"
                } else {
                    "update"
                };
                trace_session.record_span(
                    &change.resource_id,
                    &rt,
                    machine_name,
                    action,
                    std::time::Duration::from_secs_f64(duration),
                    0,
                    None,
                );
            }
            Ok(out) => {
                let error = super::failure_text::exec_failure(&site, &out);
                let _ = record_failure(
                    ctx,
                    &change.resource_id,
                    &resource.resource_type,
                    duration,
                    &error,
                );
                counters.failed += 1;
                counters.failed_resources.insert(change.resource_id.clone());
                let rt = resource_type_label(cfg, &change.resource_id);
                trace_session.record_span(
                    &change.resource_id,
                    &rt,
                    machine_name,
                    "create",
                    std::time::Duration::from_secs_f64(duration),
                    1,
                    None,
                );
            }
            Err(e) => {
                let error = super::failure_text::transport_failure(&e);
                let _ = record_failure(
                    ctx,
                    &change.resource_id,
                    &resource.resource_type,
                    duration,
                    &error,
                );
                counters.failed += 1;
                counters.failed_resources.insert(change.resource_id.clone());
                let rt = resource_type_label(cfg, &change.resource_id);
                trace_session.record_span(
                    &change.resource_id,
                    &rt,
                    machine_name,
                    "create",
                    std::time::Duration::from_secs_f64(duration),
                    1,
                    None,
                );
            }
        }
    }
    // FJ-63: Never stop between waves — dependency skipping handles cascade
    Ok(false)
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
