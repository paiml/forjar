//! The apply path: mode dispatch plus the batch, service and gate scripts.

use super::check::pipeline_script;
use super::helpers::{service_rid, timeout_wrapped, NOT_CONVERGED_MARKER};
use super::query::{gather_script, scatter_script};
use crate::core::shell_escape::sh_squote;
use crate::core::types::{Resource, TaskMode};

/// Generate shell script to execute the task command.
///
/// - Uses `set -euo pipefail` for strict error handling
/// - Supports `working_dir` to cd before execution
/// - Supports `timeout` for time-limited execution
/// - FJ-2700: Mode-aware script generation:
///   - Pipeline: sequential stages with gate enforcement
///   - Service: background process with PID file and health check
///   - Dispatch: pre-flight gate check before execution
///   - Batch (default): run-once with scatter/gather
/// - FJ-2704: Runs scatter before command, gather after command
pub fn apply_script(resource: &Resource) -> String {
    // FJ-2700: Pipeline tasks with stages get stage-aware script
    if !resource.stages.is_empty() {
        return pipeline_script(resource);
    }

    // FJ-2700/E21: Mode-aware script dispatch
    match resource.task_mode.as_ref().unwrap_or(&TaskMode::Batch) {
        TaskMode::Service => return service_script(resource),
        TaskMode::Dispatch => return dispatch_script(resource),
        TaskMode::Pipeline | TaskMode::Batch => {} // fall through to batch
    }

    batch_script(resource)
}

/// Generate batch-mode script (default): run command once with scatter/gather.
fn batch_script(resource: &Resource) -> String {
    let command = resource.command.as_deref().unwrap_or("true");
    let mut script = String::from("set -euo pipefail\n");

    if let Some(scatter) = scatter_script(resource) {
        script.push_str(&scatter);
    }
    if let Some(ref dir) = resource.working_dir {
        script.push_str(&format!("cd {}\n", sh_squote(dir)));
    }
    if let Some(timeout_secs) = resource.timeout {
        script.push_str(&timeout_wrapped(command, timeout_secs));
    } else {
        script.push_str(command);
        script.push('\n');
    }
    if let Some(gather) = gather_script(resource) {
        script.push_str(&gather);
    }

    // GH-254: re-assert the completion_check AFTER running.
    //
    // The check was used only as a guard on whether to run, never as evidence
    // that running worked, so `converged` meant "the command exited 0" rather
    // than "the resource reached its declared state". A task could do
    // everything right, exit 0, and leave the declared condition false — and
    // the lock recorded success, so the next `plan` reported `no changes` over
    // a host that never converged.
    //
    // Observed on paiml/infra's `lean-toolchain`: `sudo: true` made $HOME=/root,
    // so the toolchain installed where the runner user could not read it. Every
    // command succeeded, `forjar apply` reported `1 converged, 0 failed`, and
    // `command -v lean` failed immediately afterwards.
    //
    // The check is already written and already cheap — it just ran. Running it
    // once more turns an exit code into a statement about the world.
    if let Some(ref check) = resource.completion_check {
        // THE CHECK GETS A LINE OF ITS OWN.
        //
        // This used to emit `if ! { <check> ; }; then` — all on ONE physical
        // line. A `completion_check` written as a YAML FOLDED scalar (`>-`)
        // arrives already collapsed onto one line, so a loop inside it produced
        //
        //     if ! { sh -c '... for p in ...; do ...; done; exit 0' ; }; then
        //
        // and bashrs' line-based rules read that as a malformed `if`:
        // SC2136 (`\bif\b[^\n]*;\s*do\b`) and SC2135
        // (`\bfor\b[^\n]*\bthen\b`). Both are SC2*, which purifier does not
        // filter, and both are Error severity — so `forjar apply` aborted the
        // resource as an I8 violation on a check containing no `if` at all.
        //
        // This is the SAME defect fixed in verdict.rs for #281, at a generator
        // that fix missed. Found by applying a real `nas_archive`-era resource
        // to gx10: the script and unit deployed, and the enable task failed I8.
        //
        // A newline is a command separator, so `{` NEWLINE cmd NEWLINE `}` is a
        // well-formed group and needs no `;` — verified running under both dash
        // and bash, for a check that passes AND one that fails.
        //
        // trim_end() still matters: a YAML `|` block scalar keeps its trailing
        // newline, and a blank line before `}` is harmless but noisy.
        script.push_str("if ! {\n");
        script.push_str(check.trim_end());
        script.push_str("\n}\nthen\n");
        script.push_str(&format!("  echo '{NOT_CONVERGED_MARKER}' >&2\n"));
        script.push_str("  echo 'task=not-converged: the declared state was not reached' >&2\n");
        script.push_str("  exit 1\n");
        script.push_str("fi\n");
    }

    script
}

/// FJ-2700/E21: Service mode — background process with PID file and health check.
///
/// Generates a script that:
/// 1. Checks if already running via PID file
/// 2. Starts the command in background with nohup
/// 3. Writes PID file for lifecycle tracking
/// 4. Runs initial health check if configured
fn service_script(resource: &Resource) -> String {
    let command = resource.command.as_deref().unwrap_or("true");
    let rid = service_rid(resource);
    let pidfile = sh_squote(&format!("/tmp/forjar-svc-{rid}.pid"));
    let logfile = sh_squote(&format!("/tmp/forjar-svc-{rid}.log"));

    let mut script = String::from("set -euo pipefail\n");
    if let Some(ref dir) = resource.working_dir {
        script.push_str(&format!("cd {}\n", sh_squote(dir)));
    }

    // Check if already running
    script.push_str(&format!(
        "if [ -f {pidfile} ] && kill -0 \"$(cat {pidfile})\" 2>/dev/null; then\n\
         \x20 echo 'service={rid} already running (pid='\"$(cat {pidfile})\"')'\n\
         \x20 exit 0\nfi\n"
    ));

    // Start in background with nohup, capture PID. `command` is intentionally
    // arbitrary shell; the log redirect target is now a slugified, quoted path.
    script.push_str(&format!(
        "nohup bash -c '{command}' > {logfile} 2>&1 &\n\
         FORJAR_SVC_PID=$!\n\
         echo $FORJAR_SVC_PID > {pidfile}\n\
         echo 'service={rid} started (pid='$FORJAR_SVC_PID')'\n"
    ));

    // FJ-3000: PID-aware health check — verify process is alive before each probe
    if let Some(ref hc) = resource.health_check {
        let timeout = hc
            .timeout
            .as_deref()
            .and_then(|t| t.strip_suffix('s'))
            .unwrap_or("5");
        let retries = hc.retries.unwrap_or(3);
        script.push_str(&format!(
            "sleep 1\nfor _i in $(seq 1 {retries}); do\n\
             \x20 if ! kill -0 \"$FORJAR_SVC_PID\" 2>/dev/null; then\n\
             \x20\x20\x20 echo 'service={rid} DIED during startup (pid='$FORJAR_SVC_PID')'\n\
             \x20\x20\x20 tail -20 {logfile} 2>/dev/null || true\n\
             \x20\x20\x20 rm -f {pidfile}\n\
             \x20\x20\x20 exit 1\n\
             \x20 fi\n\
             \x20 if timeout {timeout} bash -c '{}'; then\n\
             \x20\x20\x20 echo 'service={rid} healthy'\n\
             \x20\x20\x20 exit 0\n\
             \x20 fi\n\
             \x20 sleep 1\ndone\n\
             echo 'service={rid} started but health check pending'\n",
            hc.command
        ));
    }

    script
}

/// FJ-2700/E21: Dispatch mode — pre-flight gate check before execution.
///
/// If a quality_gate is configured, runs it as a pre-flight check.
/// Gate failure aborts the dispatch with the gate message.
fn dispatch_script(resource: &Resource) -> String {
    let command = resource.command.as_deref().unwrap_or("true");
    let mut script = String::from("set -euo pipefail\n");

    if let Some(ref dir) = resource.working_dir {
        script.push_str(&format!("cd {}\n", sh_squote(dir)));
    }

    // Pre-flight gate check
    if let Some(ref gate) = resource.quality_gate {
        if let Some(ref gate_cmd) = gate.command {
            let msg = gate
                .message
                .as_deref()
                .unwrap_or("dispatch gate check failed");
            script.push_str(&format!(
                "if ! bash -c '{gate_cmd}'; then\n\
                 \x20 echo {}\n\
                 \x20 exit 1\nfi\n",
                sh_squote(&format!("DISPATCH BLOCKED: {msg}"))
            ));
        }
    }

    // Execute the dispatch command
    if let Some(scatter) = scatter_script(resource) {
        script.push_str(&scatter);
    }
    if let Some(timeout_secs) = resource.timeout {
        script.push_str(&timeout_wrapped(command, timeout_secs));
    } else {
        script.push_str(command);
        script.push('\n');
    }
    if let Some(gather) = gather_script(resource) {
        script.push_str(&gather);
    }
    script
}
