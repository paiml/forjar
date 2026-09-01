//! Refs #390: the failure text an operator reads, pinned.
//!
//! Every test here fails against the pre-#390 tree, where the operator's only
//! failure string was `format!("exit code {}: {}", out.exit_code,
//! out.stderr.trim())` — stdout structurally absent, and a task whose command
//! exited 0 reported as `exit code 1`.

use super::failure_text::*;
use crate::core::types::Resource;
use crate::transport::ExecOutput;
use std::path::Path;

/// A task resource carrying only what `failure_text` reads off it.
fn task(check: Option<&str>) -> Resource {
    Resource {
        completion_check: check.map(String::from),
        ..Default::default()
    }
}

fn out(code: i32, stdout: &str, stderr: &str) -> ExecOutput {
    ExecOutput {
        exit_code: code,
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
    }
}

fn site<'a>(resolved: &'a Resource, log: Option<&'a Path>) -> Site<'a> {
    Site {
        resource_id: "llama-cpp-build",
        state_dir: Path::new("/abs/state"),
        run_id: Some("r-3534c7969c62"),
        log,
        resolved,
    }
}

/// The GH-254 marker as the generated script emits it, plus cmake's stderr.
fn not_converged_stderr() -> String {
    format!(
        "CMake Warning:\n  CMAKE_BUILD_TYPE=Release\n{}\ntask=not-converged: the declared state \
         was not reached",
        crate::resources::task::NOT_CONVERGED_MARKER
    )
}

#[test]
fn the_stdout_the_reporter_hunted_for_six_times_is_in_the_message() {
    let r = task(Some("test -x /opt/llama.cpp/build/bin/llama-server"));
    let o = out(
        1,
        "nvcc: /usr/local/cuda/bin/nvcc\nGGML_CUDA grep result: GGML_CUDA:BOOL=ON",
        &not_converged_stderr(),
    );
    let msg = exec_failure(&site(&r, None), &o);
    // The four diagnostics whose absence made #390 look like a caching bug.
    assert!(msg.contains("nvcc: /usr/local/cuda/bin/nvcc"), "{msg}");
    assert!(msg.contains("GGML_CUDA:BOOL=ON"), "{msg}");
    // And the stderr that always did survive, so the fix is not a swap.
    assert!(msg.contains("CMAKE_BUILD_TYPE=Release"), "{msg}");
}

#[test]
fn a_command_failure_and_a_not_converged_task_do_not_read_the_same() {
    let r = task(Some("test -x /opt/x"));
    let failed = exec_failure(&site(&r, None), &out(127, "", "bash: cmake: not found"));
    assert!(failed.contains("the command FAILED"), "{failed}");
    assert!(!failed.contains("NOT CONVERGED"), "{failed}");

    let stuck = exec_failure(&site(&r, None), &out(1, "", &not_converged_stderr()));
    assert!(stuck.contains("NOT CONVERGED"), "{stuck}");
    assert!(stuck.contains("the command itself exited 0"), "{stuck}");
    // The check that is actually false, printed verbatim — named nowhere before.
    assert!(stuck.contains("test -x /opt/x"), "{stuck}");
}

#[test]
fn a_nested_shell_task_does_not_claim_the_command_succeeded() {
    let mut r = task(Some("test -x /opt/x"));
    r.timeout = Some(600);
    let msg = exec_failure(&site(&r, None), &out(1, "", &not_converged_stderr()));
    assert!(msg.contains("#390-E"), "{msg}");
    assert!(
        msg.contains("does NOT inherit `set -euo pipefail`"),
        "{msg}"
    );

    let mut sudo = task(Some("test -x /opt/x"));
    sudo.sudo = true;
    let smsg = exec_failure(&site(&sudo, None), &out(1, "", &not_converged_stderr()));
    assert!(smsg.contains("#390-E"), "{smsg}");
}

#[test]
fn an_empty_stream_is_stated_and_not_merely_omitted() {
    let r = task(None);
    let msg = exec_failure(&site(&r, None), &out(1, "  \n ", "boom"));
    // "my echoes printed nothing" vs "forjar is hiding my echoes" — the
    // distinction #390's reporter could not make across six builds.
    assert!(msg.contains("--- stdout: (empty) ---"), "{msg}");
    assert!(msg.contains("--- stderr (4 bytes) ---"), "{msg}");
}

#[test]
fn a_megabyte_on_both_streams_stays_bounded_and_keeps_both_ends() {
    let big: String = (0..40_000)
        .map(|i| format!("progress line {i}\n"))
        .collect();
    let r = task(None);
    let msg = exec_failure(&site(&r, None), &out(1, &big, &big));
    assert!(msg.len() < 5_000, "unbounded: {} bytes", msg.len());
    // Head kept: #390's diagnostics ran BEFORE the build, so a tail-only
    // window is exactly the one that would have hidden them again.
    assert!(msg.contains("progress line 0\n"), "head lost");
    assert!(msg.contains("progress line 39999"), "tail lost");
    assert!(msg.contains("elided from the middle"), "{msg}");
}

#[test]
fn multibyte_output_does_not_panic_at_the_cut() {
    let body = "你好世界🎉".repeat(4_000);
    let r = task(None);
    // A diagnostic that panics is worse than the bug it was printing.
    let msg = exec_failure(&site(&r, None), &out(1, &body, "é"));
    assert!(msg.contains("elided from the middle"), "{msg}");
}

#[test]
fn the_pointer_names_the_log_and_the_command_that_prints_it() {
    let dir = std::env::temp_dir().join("fj390_pointer_test");
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("llama-cpp-build.create.log");
    std::fs::write(&log, "=== STDOUT ===\nnvcc\n").unwrap();

    let r = task(None);
    let msg = exec_failure(&site(&r, Some(&log)), &out(1, "x", "y"));
    assert!(msg.contains("llama-cpp-build.create.log"), "{msg}");
    assert!(msg.contains("forjar logs --state-dir"), "{msg}");
    assert!(msg.contains("r-3534c7969c62"), "{msg}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_relative_state_dir_is_flagged_because_that_is_how_the_evidence_died() {
    let dir = std::env::temp_dir().join("fj390_relative_test");
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("r.create.log");
    std::fs::write(&log, "x").unwrap();
    let r = task(None);
    let s = Site {
        resource_id: "r",
        // The `--state-dir` default. A stateless CI runner deletes it with the
        // checkout, which is why #390's transcript was gone when it mattered.
        state_dir: Path::new("state"),
        run_id: Some("r-1"),
        log: Some(&log),
        resolved: &r,
    };
    let msg = exec_failure(&s, &out(1, "x", "y"));
    assert!(msg.contains("NOTE --state-dir is relative"), "{msg}");
    assert!(msg.contains("stateless CI runner"), "{msg}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_log_means_no_pointer_and_the_message_says_which() {
    let r = task(None);
    // The `--parallel` shape: no run log is written at all (#390-A). The
    // message must not name a file that was never created.
    let msg = exec_failure(&site(&r, None), &out(1, "x", "y"));
    assert!(msg.contains("no run log was written"), "{msg}");
    assert!(!msg.contains("forjar logs --state-dir"), "{msg}");
}

#[test]
fn the_host_verdict_keeps_the_stderr_it_used_to_destroy() {
    // FJ-2732's arm, which reported stdout and threw stderr away — the exact
    // mirror of #390, on the branch every task without a completion_check hits.
    let msg = host_verdict(&out(
        1,
        "task=pending",
        "test: /opt/x: No such file or directory",
    ));
    assert!(msg.contains("does not report the declared state"), "{msg}");
    assert!(msg.contains("No such file or directory"), "{msg}");
    assert!(msg.contains("task=pending"), "{msg}");
}

#[test]
fn a_verify_failure_shows_the_applys_own_output_too() {
    let r = task(None);
    let applied = out(0, "BUILD-STDOUT-MARKER", "");
    let msg = verify_failure(&site(&r, None), &applied, "task=pending");
    assert!(msg.starts_with("NOT CONVERGED"), "{msg}");
    assert!(msg.contains("BUILD-STDOUT-MARKER"), "{msg}");
}

#[test]
fn a_hook_failure_shows_the_stdout_hooks_actually_print_to() {
    // People write `echo "nginx config invalid"` without `>&2`.
    let msg = hook_failure("pre_apply", &out(1, "nginx: config invalid: line 42", ""));
    assert!(msg.starts_with("pre_apply hook failed (exit 1)"), "{msg}");
    assert!(msg.contains("nginx: config invalid: line 42"), "{msg}");
}

#[test]
fn a_transport_error_keeps_its_prefix_and_clips_from_the_head() {
    let long = format!(
        "I8 violation — script failed bashrs validation{}",
        "x".repeat(20_000)
    );
    let msg = transport_failure(&long);
    // `core::error::DECLARED_MARKERS` classifies on a marker at the HEAD; a
    // tail cut would re-code a validation failure as a retryable one.
    assert!(msg.starts_with("transport error: I8 violation"), "{msg}");
    assert!(msg.contains("elided"), "{msg}");
    assert!(msg.contains("no run log exist"), "{msg}");
    assert!(msg.len() < 5_000, "unbounded: {}", msg.len());
}
