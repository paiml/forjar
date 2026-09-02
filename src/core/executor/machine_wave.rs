//! Wave execution: the transport I/O phase of the one scheduler.
//!
//! Refs #412 (CRUX audit E09): there is no "parallel path" and "sequential
//! path" any more — there is one scheduler whose waves are width 1 unless
//! `--parallel` widens them (see `machine::schedule_waves`). Everything a
//! resource gets on one width it gets on the other; this file is where the
//! per-resource work of a WIDE wave happens, and it is deliberately the same
//! work `resource_ops::execute_resource` does for a width-1 wave.

use super::machine::PreparedResource;
use super::*;

/// Phase 2: Execute transport I/O in parallel threads.
/// Refs #390-A: the fourth element is the SCRIPT THAT RAN.
///
/// It used to be built and dropped inside the closure
/// (`apply_script(..).and_then(|script| ..)`), which is the mechanical reason
/// this path never wrote a run log: `run_capture` needs the script, and nothing
/// outside the thread had it. Measured A/B with only `policy.parallel_resources`
/// flipped: sequential wrote 8 files including a full `=== STDOUT ===` section,
/// parallel produced no `runs/` directory at all. A failing task's transcript was
/// DESTROYED, not merely hidden from the console.
pub(super) type WaveResult = (usize, f64, Result<transport::ExecOutput, String>, String);

/// One attempt at a prepared resource: what came back, the script that ran, and
/// whether re-running it is safe.
struct Attempt {
    output: Result<transport::ExecOutput, String>,
    script: String,
    /// #165: a pre_apply GATE failure is never retried — re-running that hook
    /// would re-execute its side effects. Same rule as `ResourceOutcome::Failed`.
    retryable: bool,
}

pub(super) fn execute_wave_io(
    cfg: &ApplyConfig,
    prepared: &[PreparedResource],
    machine: &Machine,
) -> Vec<WaveResult> {
    std::thread::scope(|s| {
        let handles: Vec<(usize, _)> = prepared
            .iter()
            .map(|prep| {
                (
                    prep.change_idx,
                    s.spawn(move || run_prepared(cfg, prep, machine)),
                )
            })
            .collect();
        join_wave_results(handles)
    })
}

/// Join the wave's threads, keeping every result attached to ITS OWN resource.
///
/// Refs #412: the panic arm returned index 0, so a thread that panicked
/// recorded its failure against whichever resource happened to be first in the
/// wave — a converged resource could be written to the lock as failed while the
/// one that actually died was recorded as nothing at all. The index travels
/// with the handle now, so there is no index to guess.
pub(super) fn join_wave_results(
    handles: Vec<(usize, std::thread::ScopedJoinHandle<'_, WaveResult>)>,
) -> Vec<WaveResult> {
    handles
        .into_iter()
        .map(|(idx, handle)| match handle.join() {
            Ok(result) => result,
            Err(panic_payload) => {
                let msg = extract_panic_message(panic_payload);
                eprintln!("error: wave execution thread panicked: {msg}");
                (idx, 0.0, Err(format!("thread panic: {msg}")), String::new())
            }
        })
        .collect()
}

/// One prepared resource, hooks and FJ-283 retry included.
fn run_prepared(cfg: &ApplyConfig, prep: &PreparedResource, machine: &Machine) -> WaveResult {
    let start = Instant::now();
    let mut done = 0u32;
    loop {
        let attempt = attempt_prepared(cfg, prep, machine);
        if !should_retry(cfg, &attempt, done) {
            return (
                prep.change_idx,
                start.elapsed().as_secs_f64(),
                attempt.output,
                attempt.script,
            );
        }
        done += 1;
        // Refs #412: the one retry loop, at every wave width.
        let backoff = std::time::Duration::from_secs(1u64 << (done - 1).min(4));
        eprintln!(
            "  retry {}/{} for {} (backoff {:?})",
            done, cfg.retry, prep.resource_id, backoff
        );
        std::thread::sleep(backoff);
    }
}

/// FJ-283: is another attempt owed?
///
/// Retries a retryable failure while the policy is not `StopOnFirst`. Under
/// the default policy `--retry` is therefore inert — that is the shipped
/// behaviour the retired width-1 path had, not a gap.
fn should_retry(cfg: &ApplyConfig, attempt: &Attempt, done: u32) -> bool {
    if done >= cfg.retry || !attempt.retryable {
        return false;
    }
    if cfg.config.policy.failure == FailurePolicy::StopOnFirst {
        return false;
    }
    match &attempt.output {
        Ok(out) => !out.success(),
        Err(_) => true,
    }
}

/// pre_apply gate, then the resource's own script.
fn attempt_prepared(cfg: &ApplyConfig, prep: &PreparedResource, machine: &Machine) -> Attempt {
    if let Some(ref pre_hook) = prep.resolved.pre_apply {
        if let Some(err) =
            super::output_verify::run_pre_apply_hook(machine, pre_hook, cfg.timeout_secs)
        {
            // A pre_apply gate failure never reached the transport, so there
            // is no script and no transcript -- the empty string is the
            // honest value, and `capture_exec_output` is not called for
            // an Err output anyway.
            return Attempt {
                output: Err(err),
                script: String::new(),
                retryable: false,
            };
        }
    }
    let (output, script) = exec_prepared(cfg, prep, machine);
    Attempt {
        output,
        script,
        retryable: true,
    }
}

/// Run the resource's script, returning it alongside the result so the caller
/// can persist the transcript (#390-A).
fn exec_prepared(
    cfg: &ApplyConfig,
    prep: &PreparedResource,
    machine: &Machine,
) -> (Result<transport::ExecOutput, String>, String) {
    if prep.use_copia {
        return (
            copia_apply_file(machine, &prep.resolved, cfg.timeout_secs),
            String::new(),
        );
    }
    let script = match codegen::apply_script(&prep.resolved) {
        Ok(script) => script,
        Err(e) => return (Err(e), String::new()),
    };
    // FJ-1397: Debug trace mode — print generated script, as the width-1 path does.
    if cfg.trace {
        eprintln!("[TRACE] {} script:\n{}", prep.resource_id, script);
    }
    let out = transport::exec_script_retry(
        machine,
        &script,
        cfg.timeout_secs,
        cfg.config.policy.ssh_retries,
    );
    (out, script)
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
