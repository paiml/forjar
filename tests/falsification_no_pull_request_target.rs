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

/// Trigger names a parsed workflow declares under `on` (string, sequence or mapping form).
fn trigger_names(text: &str) -> Vec<String> {
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(text).expect("workflow must parse");
    let on = doc
        .get("on")
        .or_else(|| doc.get(serde_yaml_ng::Value::Bool(true)));
    match on {
        Some(serde_yaml_ng::Value::String(s)) => vec![s.clone()],
        Some(serde_yaml_ng::Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect(),
        Some(serde_yaml_ng::Value::Mapping(m)) => m
            .keys()
            .filter_map(|k| k.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

fn guard_rc_on(file_body: &str) -> Option<i32> {
    let dir = tempfile::tempdir().expect("tempdir");
    let wf = dir.path().join("workflows");
    fs::create_dir(&wf).expect("mkdir");
    fs::write(wf.join("a.yml"), file_body).expect("write");
    let no_allowlist = dir.path().join("absent-allowlist.txt");
    sh(
        "scripts/guards/no-pull-request-target-guard.sh",
        &[&wf, &no_allowlist],
    )
    .status
    .code()
}

#[test]
fn no_workflow_declares_the_trigger_once_parsed() {
    // Second layer: a YAML parser decodes quoting and escapes the shell guard cannot.
    let dir = repo().join(".github/workflows");
    let mut scanned = 0;
    for e in fs::read_dir(&dir).expect("read .github/workflows") {
        let p = e.expect("dir entry").path();
        if !p.extension().is_some_and(|x| x == "yml" || x == "yaml") {
            continue;
        }
        scanned += 1;
        let text = fs::read_to_string(&p).expect("workflow must be readable");
        let names = trigger_names(&text);
        assert!(
            !names
                .iter()
                .any(|t| t.eq_ignore_ascii_case("pull_request_target")),
            "{} triggers on pull_request_target: {names:?}",
            p.display()
        );
    }
    assert!(scanned > 0, "0 workflows parsed is not a pass");
}

#[test]
fn the_parser_layer_decodes_an_escaped_trigger() {
    let names = trigger_names("on: { \"pull_request\\u005ftarget\": {} }\njobs: {}\n");
    assert_eq!(names, vec!["pull_request_target".to_owned()]);
}

#[test]
fn the_guard_refuses_forms_that_hide_the_trigger_from_text_matching() {
    for (name, body) in [
        (
            "quoted hash before the key",
            "{\"name\": \"ci #1\", \"on\": {\"pull_request_target\": {}}, \"jobs\": {}}\n",
        ),
        (
            "unicode escape",
            "on: { \"pull_request\\u005ftarget\": {} }\n",
        ),
        ("hex escape", "on: { \"pull_request\\x5ftarget\": {} }\n"),
        ("mixed case", "on: [Pull_Request_Target]\n"),
        ("trailing comment", "on: [push] # pull_request_target\n"),
    ] {
        assert_eq!(guard_rc_on(body), Some(1), "guard passed the {name} form");
    }
}

#[test]
fn the_guard_refuses_an_unreadable_workflow() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let wf = dir.path().join("workflows");
    fs::create_dir(&wf).expect("mkdir");
    let f = wf.join("evil.yml");
    fs::write(&f, "on:\n  pull_request_target:\n").expect("write");
    fs::set_permissions(&f, fs::Permissions::from_mode(0o000)).expect("chmod");
    let readable = fs::read(&f).is_ok(); // root reads mode 000
    let out = sh(
        "scripts/guards/no-pull-request-target-guard.sh",
        &[&wf, &dir.path().join("absent")],
    );
    fs::set_permissions(&f, fs::Permissions::from_mode(0o600)).expect("chmod back");
    let want = if readable { 1 } else { 2 };
    assert_eq!(
        out.status.code(),
        Some(want),
        "an unread workflow is unmeasured, never a pass"
    );
}

#[test]
fn the_pr_gate_job_runs_the_guard_for_every_same_repo_pr() {
    let text = fs::read_to_string(repo().join(".github/workflows/pr-gate.yml")).expect("read");
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("parse");
    let job = doc
        .get("jobs")
        .and_then(|j| j.get("no-pull-request-target"))
        .expect("pr-gate.yml must define the no-pull-request-target job");
    assert_eq!(
        job.get("if").and_then(|v| v.as_str()),
        Some("github.event.pull_request.head.repo.full_name == github.repository"),
        "the job's condition must be the same-repo head guard and nothing narrower"
    );
    assert!(
        job.get("continue-on-error").is_none(),
        "no third state on the gate"
    );
    let runs: Vec<&str> = job
        .get("steps")
        .and_then(|s| s.as_sequence())
        .expect("steps")
        .iter()
        .filter_map(|s| s.get("run").and_then(|r| r.as_str()))
        .collect();
    assert!(
        runs.iter().any(
            |r| r.contains("scripts/guards/no-pull-request-target-guard.sh")
                && r.contains("$RUNNER_TEMP/allow.txt")
        ),
        "the job must run the guard with the base branch's allowlist: {runs:?}"
    );
}

#[test]
fn ci_runs_this_falsifier_in_a_required_job() {
    let text = fs::read_to_string(repo().join(".github/workflows/ci.yml")).expect("read");
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("parse");
    let job = doc
        .get("jobs")
        .and_then(|j| j.get("dogfood-guards"))
        .expect("ci.yml dogfood-guards job");
    let runs: String = job
        .get("steps")
        .and_then(|s| s.as_sequence())
        .expect("steps")
        .iter()
        .filter_map(|s| s.get("run").and_then(|r| r.as_str()))
        .collect();
    assert!(
        runs.contains("--test falsification_no_pull_request_target "),
        "ci.yml dogfood-guards must run --test falsification_no_pull_request_target"
    );
    let gate_needs = doc
        .get("jobs")
        .and_then(|j| j.get("gate"))
        .and_then(|g| g.get("needs"))
        .and_then(|n| n.as_sequence())
        .expect("ci.yml gate.needs");
    assert!(
        gate_needs
            .iter()
            .any(|n| n.as_str() == Some("dogfood-guards")),
        "the required gate job must need dogfood-guards"
    );
}
