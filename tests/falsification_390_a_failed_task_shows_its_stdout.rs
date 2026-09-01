//! Refs #390: a failed task's STDOUT must reach the operator.
//!
//! THE FLAW THIS CLOSES.
//!
//! An operator building llama.cpp with CUDA on `gx10` ran `forjar apply` six
//! times, editing `command:` between runs to add `echo`, `nvcc --version` and
//! `grep GGML_CUDA` diagnostics. Not one of them ever appeared. The two
//! `CMake Warning` lines appeared every time, byte-identical. They concluded —
//! reasonably, and wrongly — that forjar was replaying a cached transcript, and
//! filed a caching bug.
//!
//! Nothing was cached. Seven reproduction lanes proved with append-only counter
//! files that the command re-ran on every apply. The whole symptom was stream
//! routing: `echo` and `nvcc --version` write to STDOUT, cmake's warnings write
//! to STDERR, and the operator's only failure line was
//! `format!("exit code {}: {}", out.exit_code, out.stderr.trim())`. `out.stdout`
//! was structurally absent, so an edit that touched only stdout could not change
//! the message — "identical across six runs" was forced by construction.
//!
//! WHAT THIS TEST MUST NOT BECOME. Asserting on the renderer in isolation would
//! prove nothing about what an operator sees: the defect lived in the wiring
//! between the executor and the console, not in any one function. Every case
//! here therefore spawns the REAL binary against a REAL config and greps the
//! REAL stderr, exactly as the reporter did.

use std::process::Command;

/// A task whose command writes a marker to stdout and a cmake-shaped warning to
/// stderr, and whose completion_check always fails — the reporter's shape,
/// minimised.
fn config(dir: &std::path::Path, extra_stdout: &str) -> String {
    format!(
        r#"version: '1.0'
name: fj390
machines:
  local:
    hostname: localhost
    addr: localhost
    transport: local
resources:
  llama-cpp-build:
    machine: local
    type: task
    working_dir: {}
    command: |
      echo "FJ390_STDOUT_nvcc: /usr/local/cuda/bin/nvcc"
      {extra_stdout}
      echo "CMake Warning: CMAKE_BUILD_TYPE=Release" >&2
    completion_check: |
      test -e /nonexistent-fj390
"#,
        dir.display()
    )
}

/// Run `forjar apply` and return its combined output.
fn apply(dir: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir)
        .args([
            "apply",
            "--yes",
            "-f",
            "forjar.yaml",
            "--state-dir",
            "state",
        ])
        .args(args)
        .output()
        .expect("forjar binary must run");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn setup(extra_stdout: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("forjar.yaml"),
        config(dir.path(), extra_stdout),
    )
    .unwrap();
    dir
}

#[test]
fn the_console_shows_the_stdout_the_reporter_spent_six_runs_looking_for() {
    let dir = setup("");
    let out = apply(dir.path(), &[]);
    // THE REGRESSION. Before this change the marker appeared ZERO times in the
    // operator-visible output, at any verbosity, which is what made a correctly
    // re-running command look like a replayed transcript.
    assert!(
        out.contains("FJ390_STDOUT_nvcc"),
        "a failed task's stdout must reach the operator; this is #390 verbatim.\n\
         --- got ---\n{out}"
    );
}

#[test]
fn the_console_still_shows_the_stderr_that_already_worked() {
    // Guards against "fixing" #390 by swapping which stream is destroyed.
    let dir = setup("");
    let out = apply(dir.path(), &[]);
    assert!(
        out.contains("CMAKE_BUILD_TYPE=Release"),
        "--- got ---\n{out}"
    );
}

#[test]
fn a_task_whose_command_exited_zero_is_not_called_a_command_failure() {
    let dir = setup("");
    let out = apply(dir.path(), &[]);
    // The other half of the misdiagnosis: the command exited 0 every time, and
    // forjar printed `exit code 1`. Six builds were spent hunting a compiler
    // error that never happened.
    assert!(out.contains("NOT CONVERGED"), "--- got ---\n{out}");
    assert!(
        out.contains("test -e /nonexistent-fj390"),
        "the completion_check is the thing that is false and must be named.\n\
         --- got ---\n{out}"
    );
}

#[test]
fn the_console_names_the_log_that_holds_the_unelided_output() {
    let dir = setup("");
    let out = apply(dir.path(), &[]);
    assert!(
        out.contains("llama-cpp-build.update.log") || out.contains("llama-cpp-build.create.log"),
        "the transcript was on disk the whole time and nothing named it.\n\
         --- got ---\n{out}"
    );
    assert!(
        out.contains("forjar logs --state-dir"),
        "--- got ---\n{out}"
    );
}

#[test]
fn editing_the_command_changes_the_message_on_the_very_next_apply() {
    // The reporter's actual experiment: edit the command, re-apply, and see
    // whether the new diagnostic appears. Before this fix a stdout-only edit
    // could not move the message, which is precisely why it read as caching.
    let dir = setup("");
    let first = apply(dir.path(), &[]);
    assert!(!first.contains("FJ390_SECOND_EDIT"), "--- got ---\n{first}");

    std::fs::write(
        dir.path().join("forjar.yaml"),
        config(dir.path(), r#"echo "FJ390_SECOND_EDIT ran""#),
    )
    .unwrap();

    let second = apply(dir.path(), &[]);
    assert!(
        second.contains("FJ390_SECOND_EDIT"),
        "a newly added stdout line must be visible on the next apply.\n\
         --- got ---\n{second}"
    );
}

#[test]
fn the_recorded_event_carries_stdout_too_and_stays_bounded() {
    let dir = setup("");
    let _ = apply(dir.path(), &[]);
    let events = dir.path().join("state/local/events.jsonl");
    let body = std::fs::read_to_string(&events).unwrap_or_default();
    assert!(
        body.contains("FJ390_STDOUT_nvcc"),
        "`forjar history` replays this log; it must carry what the console showed.\n\
         --- got ---\n{body}"
    );
    // The string is appended to an unbounded log on every failed apply, so the
    // ceiling matters more here than on the console.
    for line in body.lines() {
        assert!(
            line.len() < 20_000,
            "an event line grew unbounded: {} bytes",
            line.len()
        );
    }
}
