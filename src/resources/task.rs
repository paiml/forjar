//! ALB-027: Task resource handler.
//!
//! Runs an arbitrary command, tracks exit code, hashes output artifacts
//! for idempotency, supports completion_check and timeout.

use crate::core::shell_escape::{sh_squote, slugify_identifier};
use crate::core::types::{Resource, TaskMode};
use crate::resources::verdict;

/// Slugified service id used in `/tmp/forjar-svc-<rid>.{pid,log}` paths.
///
/// FJ-154: the resource name flows into shared filesystem paths; slugify it to
/// `[A-Za-z0-9._-]` so it can never split a redirect target or inject shell.
fn service_rid(resource: &Resource) -> String {
    slugify_identifier(resource.name.as_deref().unwrap_or("task"))
}

/// Extract absolute binary path from a command string.
///
/// Handles: `nohup /path/bin ...`, `sudo /path/bin`, `/path/bin --args`,
/// `LD_LIBRARY_PATH=/foo nohup /path/bin ...`.
pub(crate) fn extract_absolute_binary(cmd: &str) -> Option<&str> {
    for token in cmd.split_whitespace() {
        // Skip env var assignments
        if token.contains('=') && !token.starts_with('/') {
            continue;
        }
        // Skip shell builtins and prefixes
        if matches!(token, "nohup" | "sudo" | "bash" | "sh" | "env" | "exec") {
            continue;
        }
        if token.starts_with('/') {
            return Some(token);
        }
        break;
    }
    None
}

/// Generate shell script to check if a task has already completed.
///
/// If `completion_check` is set, runs it: exit 0 = already done.
/// If `output_artifacts` are set, checks if all exist.
/// FJ-2700/E21: Service mode checks PID file for running process.
/// Otherwise, always reports as needing execution.
pub fn check_script(resource: &Resource) -> String {
    // Service mode: check if process is running via PID file
    // FJ-3030: also inject ldd check for absolute-path binaries
    if resource.task_mode.as_ref() == Some(&TaskMode::Service) {
        let rid = service_rid(resource);
        let pidfile = sh_squote(&format!("/tmp/forjar-svc-{rid}.pid"));
        let ldd_check = extract_absolute_binary(resource.command.as_deref().unwrap_or(""))
            .map(|bin| {
                let b = sh_squote(bin);
                format!(
                    "if command -v ldd >/dev/null 2>&1 && [ -f {b} ]; then \
                     if ldd {b} 2>&1 | grep -q 'not found'; then \
                     echo 'task=ldd-fail'; exit 1; fi; fi; "
                )
            })
            .unwrap_or_default();
        return format!(
            "{ldd_check}{}",
            verdict::single(
                &format!("[ -f {pidfile} ] && kill -0 \"$(cat {pidfile})\" 2>/dev/null"),
                "task=completed",
                "task=pending",
            )
        );
    }

    if let Some(ref check) = resource.completion_check {
        return verdict::single(check, "task=completed", "task=pending");
    }

    if !resource.output_artifacts.is_empty() {
        // Artifacts are declared RELATIVE TO `working_dir`, which is where the
        // command ran and produced them. Testing them relative to whatever the
        // check happens to be invoked from asks about a different filesystem
        // location than the one the resource wrote to.
        //
        // This was harmless while nothing on the apply path consulted
        // check_script. FJ-2732 made the executor verify against the host after
        // every apply, and the mismatch surfaced immediately: a task with
        // `working_dir: <tmp>/work` and `output_artifacts: ["narration.srt"]`
        // produced the file, exited 0, and then failed verification with
        // `task=pending:narration.srt` because the check looked in the CWD.
        //
        // `probe_base_dir` is the same resolution the FJ-2731 output check and
        // the build prober already use, so all three now agree about where an
        // artifact lives.
        let base = crate::core::task::probe::probe_base_dir(resource);
        let assertions: Vec<String> = resource
            .output_artifacts
            .iter()
            .map(|a| {
                // With no `working_dir` the base is "." and the artifact is
                // already relative to the invoking directory — emit it bare so
                // the script stays readable (`[ -e 'out/x' ]`, not
                // `[ -e './out/x' ]`). Behaviour is identical either way.
                let resolved = crate::core::task::probe::resolve_under(&base, a);
                let path = if base == std::path::Path::new(".") {
                    a.clone()
                } else {
                    resolved.to_string_lossy().into_owned()
                };
                verdict::assert_that(
                    &format!("[ -e {} ]", sh_squote(&path)),
                    &format!("task=completed:{a}"),
                    &format!("task=pending:{a}"),
                )
            })
            .collect();
        return verdict::check_script_from(&assertions);
    }

    // No completion_check and no output_artifacts: there is no evidence this
    // task ever ran. It previously echoed `task=pending` and exited 0, which
    // `forjar check` read as a pass. Absence of evidence is not convergence.
    verdict::check_script_from(&[verdict::always_diverged("task=pending")])
}

/// Generate pipeline script with inter-stage gate enforcement.
///
/// Each stage runs sequentially. If a gate stage fails (non-zero exit),
/// the pipeline aborts immediately. Non-gate stages log failure but continue.
fn pipeline_script(resource: &Resource) -> String {
    let mut script = String::from("set -euo pipefail\n");
    if let Some(ref dir) = resource.working_dir {
        script.push_str(&format!("cd {}\n", sh_squote(dir)));
    }
    script.push_str("FORJAR_PIPELINE_OK=0\n");
    for (i, stage) in resource.stages.iter().enumerate() {
        let cmd = stage.command.as_deref().unwrap_or("true");
        let name = if stage.name.is_empty() {
            format!("stage-{i}")
        } else {
            stage.name.clone()
        };
        script.push_str(&format!(
            "echo {}\n",
            sh_squote(&format!("=== Stage: {name} ==="))
        ));
        if stage.gate {
            // Gate stage: abort pipeline on failure. `cmd` is intentionally
            // arbitrary shell; the stage name in the message is escaped.
            script.push_str(&format!(
                "if ! bash -c '{cmd}'; then\n  echo {}\n  exit 1\nfi\n",
                sh_squote(&format!("GATE FAILED: {name}"))
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
        script.push_str(&format!("cd {}\n", sh_squote(dir)));
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
        script.push_str(
            "  echo 'task=not-converged: command exited 0 but completion_check still fails' >&2\n",
        );
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
            .map(|a| {
                let q = sh_squote(a);
                format!(
                    "[ -f {q} ] && b3sum {q} 2>/dev/null || echo {}",
                    sh_squote(&format!("missing:{a}"))
                )
            })
            .collect();
        return hash_cmds.join("\n");
    }

    let command = resource.command.as_deref().unwrap_or("true");
    // `command` is intentionally arbitrary; quote the whole label so a stray
    // quote can't break the echo.
    format!("echo {}", sh_squote(&format!("command={command}")))
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
            let (l, r) = (sh_squote(local), sh_squote(remote));
            script.push_str(&format!("mkdir -p \"$(dirname {r})\"\ncp -r {l} {r}\n"));
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
            let (r, l) = (sh_squote(remote), sh_squote(local));
            script.push_str(&format!("mkdir -p \"$(dirname {l})\"\ncp -r {r} {l}\n"));
        }
    }
    Some(script)
}

#[cfg(test)]
mod fj154_tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn service_resource(name: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Task,
            machine: MachineTarget::Single("m1".to_string()),
            name: Some(name.to_string()),
            command: Some("/usr/bin/myd".to_string()),
            task_mode: Some(TaskMode::Service),
            ..Default::default()
        }
    }

    #[test]
    fn fj154_service_name_slugified_in_log_path() {
        // Defect #13: a name with spaces/metachars must not split the log
        // redirect target. It is slugified to [A-Za-z0-9._-].
        let r = service_resource("x; rm -rf ~ #");
        let script = apply_script(&r);
        // Slug is "x--rm--rf"; the log redirect is a single quoted word.
        assert!(
            script.contains("> '/tmp/forjar-svc-x--rm--rf.log'"),
            "{script}"
        );
        // No bare `rm -rf ~` appears in the redirect target.
        assert!(!script.contains("/tmp/forjar-svc-x; rm -rf ~"), "{script}");
    }

    #[test]
    fn fj154_service_log_and_pid_paths_consistent() {
        let r = service_resource("my svc");
        let apply = apply_script(&r);
        let check = check_script(&r);
        // Both apply and check agree on the slugified pidfile.
        assert!(apply.contains("'/tmp/forjar-svc-my-svc.pid'"), "{apply}");
        assert!(check.contains("'/tmp/forjar-svc-my-svc.pid'"), "{check}");
        assert!(apply.contains("'/tmp/forjar-svc-my-svc.log'"), "{apply}");
    }

    #[test]
    fn fj154_service_benign_name_unchanged() {
        let r = service_resource("web");
        let script = apply_script(&r);
        assert!(script.contains("> '/tmp/forjar-svc-web.log'"), "{script}");
        assert!(script.contains("'/tmp/forjar-svc-web.pid'"), "{script}");
    }

    #[test]
    fn fj154_output_artifacts_quoted() {
        let mut r = service_resource("t");
        r.task_mode = None;
        r.output_artifacts = vec!["/out/x';id;'".to_string()];
        let q = state_query_script(&r);
        assert!(q.contains("'\\''"), "{q}");
        assert!(!q.contains("b3sum '/out/x';id"), "{q}");
    }
}
