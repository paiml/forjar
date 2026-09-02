//! Pre-flight: has this task already converged, and pipeline-stage scripts.

use super::helpers::{extract_absolute_binary, service_rid};
use crate::core::shell_escape::sh_squote;
use crate::core::types::{Resource, TaskMode};
use crate::resources::verdict;

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
pub(super) fn pipeline_script(resource: &Resource) -> String {
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
