//! The `no-pull-request-target` gate must hold on every push, not only when a PR opens.
//!
//! PMAT-913, #935. `pull_request_target` runs with a write token and secrets in
//! the base-repository context on a PR from a fork. The guard script refuses a
//! workflow that uses it. Two ways the gate could pass while the hole is open:
//!
//!   1. the guard script accepts a workflow that carries the trigger;
//!   2. the `PR Gate` workflow runs the guard only on `opened`/`reopened`, so a
//!      later push that adds the trigger to an open PR is never checked.
//!
//! THESE TESTS RUN THE SCRIPTS. They do not assert on script text.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sh(script: &str, args: &[&Path]) -> std::process::Output {
    Command::new("sh")
        .arg(repo().join(script))
        .args(args)
        .current_dir(repo())
        .output()
        .expect("sh must be runnable")
}

/// The event types listed under `on.pull_request.types` in a workflow file.
fn pull_request_types(workflow: &Path) -> Vec<String> {
    let text = fs::read_to_string(workflow).expect("workflow file must be readable");
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("workflow must parse");
    // YAML 1.1 parsers read the bare key `on` as boolean true; accept either.
    let on = doc
        .get("on")
        .or_else(|| doc.get(serde_yaml_ng::Value::Bool(true)))
        .expect("workflow must have an `on` block");
    on.get("pull_request")
        .and_then(|pr| pr.get("types"))
        .and_then(|t| t.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_guard_fixture_suite_passes() {
    let out = sh("scripts/guards/test-no-pull-request-target-guard.sh", &[]);
    assert!(
        out.status.success(),
        "fixture suite failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn the_guard_passes_on_this_repositorys_workflows() {
    let out = sh("scripts/guards/no-pull-request-target-guard.sh", &[]);
    assert!(
        out.status.success(),
        "a workflow in .github/workflows uses pull_request_target:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn the_guard_refuses_a_workflow_that_adds_the_trigger() {
    let dir = tempfile::tempdir().expect("tempdir");
    let wf = dir.path().join("workflows");
    fs::create_dir(&wf).expect("mkdir");
    fs::write(
        wf.join("evil.yml"),
        "name: evil\non:\n  pull_request_target:\n    types: [opened]\njobs: {}\n",
    )
    .expect("write");
    let no_allowlist = dir.path().join("absent-allowlist.txt");
    let out = sh(
        "scripts/guards/no-pull-request-target-guard.sh",
        &[&wf, &no_allowlist],
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "guard must exit 1 on pull_request_target, got {:?}",
        out.status.code()
    );
}

#[test]
fn the_guard_refuses_an_empty_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = sh(
        "scripts/guards/no-pull-request-target-guard.sh",
        &[dir.path()],
    );
    assert_eq!(out.status.code(), Some(2), "0 files scanned is not a pass");
}

#[test]
fn the_pr_gate_runs_again_on_every_push() {
    let types = pull_request_types(&repo().join(".github/workflows/pr-gate.yml"));
    for needed in ["opened", "reopened", "synchronize", "ready_for_review"] {
        assert!(
            types.iter().any(|t| t == needed),
            "pr-gate.yml on.pull_request.types lacks `{needed}` (has {types:?}): \
             a push to an open PR would skip the no-pull-request-target job"
        );
    }
}

#[test]
fn the_pr_gate_does_not_use_the_privileged_trigger() {
    let text = fs::read_to_string(repo().join(".github/workflows/pr-gate.yml")).expect("read");
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("parse");
    let on = doc
        .get("on")
        .or_else(|| doc.get(serde_yaml_ng::Value::Bool(true)))
        .expect("on block");
    assert!(
        on.get("pull_request_target").is_none(),
        "pr-gate.yml must not trigger on pull_request_target"
    );
}
