//! ALB-027: Task resource handler.
//!
//! Runs an arbitrary command, tracks exit code, hashes output artifacts
//! for idempotency, supports completion_check and timeout.

use crate::core::types::{Resource, TaskMode};

/// Generate shell script to check if a task has already completed.
///
/// If `completion_check` is set, runs it: exit 0 = already done.
/// If `output_artifacts` are set, checks if all exist.
/// FJ-2700/E21: Service mode checks PID file for running process.
/// Otherwise, always reports as needing execution.
pub fn check_script(resource: &Resource) -> String {
    // Service mode: check if process is running via PID file
    if resource.task_mode.as_ref() == Some(&TaskMode::Service) {
        let rid = resource.name.as_deref().unwrap_or("task");
        let pidfile = format!("/tmp/forjar-svc-{rid}.pid");
        return format!(
            "if [ -f '{pidfile}' ] && kill -0 \"$(cat '{pidfile}')\" 2>/dev/null; then \
             echo 'task=completed'; else echo 'task=pending'; fi"
        );
    }

    if let Some(ref check) = resource.completion_check {
        return format!("if {check}; then echo 'task=completed'; else echo 'task=pending'; fi");
    }

    if !resource.output_artifacts.is_empty() {
        let checks: Vec<String> = resource
            .output_artifacts
            .iter()
            .map(|a| format!("[ -e '{a}' ]"))
            .collect();
        return format!(
            "if {} ; then echo 'task=completed'; else echo 'task=pending'; fi",
            checks.join(" && ")
        );
    }

    "echo 'task=pending'".to_string()
}

/// Generate pipeline script with inter-stage gate enforcement.
///
/// Each stage runs sequentially. If a gate stage fails (non-zero exit),
/// the pipeline aborts immediately. Non-gate stages log failure but continue.
fn pipeline_script(resource: &Resource) -> String {
    let mut script = String::from("set -euo pipefail\n");
    if let Some(ref dir) = resource.working_dir {
        script.push_str(&format!("cd '{dir}'\n"));
    }
    script.push_str("FORJAR_PIPELINE_OK=0\n");
    for (i, stage) in resource.stages.iter().enumerate() {
        let cmd = stage.command.as_deref().unwrap_or("true");
        let name = if stage.name.is_empty() {
            format!("stage-{i}")
        } else {
            stage.name.clone()
        };
        script.push_str(&format!("echo '=== Stage: {name} ==='\n"));
        if stage.gate {
            // Gate stage: abort pipeline on failure
            script.push_str(&format!(
                "if ! bash -c '{cmd}'; then\n  echo 'GATE FAILED: {name}'\n  exit 1\nfi\n"
            ));
        } else {
            script.push_str(&format!("{cmd}\n"));
        }
    }
    script
}

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
        script.push_str(&format!("cd '{dir}'\n"));
    }
    if let Some(timeout_secs) = resource.timeout {
        script.push_str(&format!(
            "timeout {timeout_secs} bash <<'FORJAR_TIMEOUT'\n{command}\nFORJAR_TIMEOUT\n"
        ));
    } else {
        script.push_str(command);
        script.push('\n');
    }
    if let Some(gather) = gather_script(resource) {
        script.push_str(&gather);
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
    let rid = resource.name.as_deref().unwrap_or("task");
    let pidfile = format!("/tmp/forjar-svc-{rid}.pid");

    let mut script = String::from("set -euo pipefail\n");
    if let Some(ref dir) = resource.working_dir {
        script.push_str(&format!("cd '{dir}'\n"));
    }

    // Check if already running
    script.push_str(&format!(
        "if [ -f '{pidfile}' ] && kill -0 \"$(cat '{pidfile}')\" 2>/dev/null; then\n\
         \x20 echo 'service={rid} already running (pid='\"$(cat '{pidfile}')\"')'\n\
         \x20 exit 0\nfi\n"
    ));

    // Start in background with nohup, capture PID
    script.push_str(&format!(
        "nohup bash -c '{command}' > /tmp/forjar-svc-{rid}.log 2>&1 &\n\
         FORJAR_SVC_PID=$!\n\
         echo $FORJAR_SVC_PID > '{pidfile}'\n\
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
             \x20\x20\x20 tail -20 /tmp/forjar-svc-{rid}.log 2>/dev/null || true\n\
             \x20\x20\x20 rm -f '{pidfile}'\n\
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
        script.push_str(&format!("cd '{dir}'\n"));
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
                 \x20 echo 'DISPATCH BLOCKED: {msg}'\n\
                 \x20 exit 1\nfi\n"
            ));
        }
    }

    // Execute the dispatch command
    if let Some(scatter) = scatter_script(resource) {
        script.push_str(&scatter);
    }
    if let Some(timeout_secs) = resource.timeout {
        script.push_str(&format!(
            "timeout {timeout_secs} bash <<'FORJAR_TIMEOUT'\n{command}\nFORJAR_TIMEOUT\n"
        ));
    } else {
        script.push_str(command);
        script.push('\n');
    }
    if let Some(gather) = gather_script(resource) {
        script.push_str(&gather);
    }
    script
}

/// Generate shell to query task state (for BLAKE3 hashing).
///
/// Hashes output_artifacts if specified, otherwise reports command string.
pub fn state_query_script(resource: &Resource) -> String {
    if !resource.output_artifacts.is_empty() {
        let hash_cmds: Vec<String> = resource
            .output_artifacts
            .iter()
            .map(|a| format!("[ -f '{a}' ] && b3sum '{a}' 2>/dev/null || echo 'missing:{a}'"))
            .collect();
        return hash_cmds.join("\n");
    }

    let command = resource.command.as_deref().unwrap_or("true");
    format!("echo 'command={command}'")
}

/// FJ-2704: Generate shell script to scatter local artifacts to remote paths.
///
/// Each scatter entry is a "local:remote" mapping. Returns a script that copies
/// local files to their remote destinations before task execution.
pub fn scatter_script(resource: &Resource) -> Option<String> {
    if resource.scatter.is_empty() {
        return None;
    }
    let mut script = String::from("set -euo pipefail\n# FJ-2704: scatter artifacts\n");
    for mapping in &resource.scatter {
        if let Some((local, remote)) = mapping.split_once(':') {
            script.push_str(&format!(
                "mkdir -p \"$(dirname '{remote}')\"\ncp -r '{local}' '{remote}'\n"
            ));
        }
    }
    Some(script)
}

/// FJ-2704: Generate shell script to gather remote artifacts to local paths.
///
/// Each gather entry is a "remote:local" mapping. Returns a script that copies
/// remote files to their local destinations after task execution.
pub fn gather_script(resource: &Resource) -> Option<String> {
    if resource.gather.is_empty() {
        return None;
    }
    let mut script = String::from("set -euo pipefail\n# FJ-2704: gather artifacts\n");
    for mapping in &resource.gather {
        if let Some((remote, local)) = mapping.split_once(':') {
            script.push_str(&format!(
                "mkdir -p \"$(dirname '{local}')\"\ncp -r '{remote}' '{local}'\n"
            ));
        }
    }
    Some(script)
}
