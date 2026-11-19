//! A job with `id-token: write` must never resolve to a spot (burst) label. The shell guard
//! `tests/pr-spot-labels-guard.sh` decides; this runs it on the real tree (must pass) and on
//! mutants (must fail), so the guard is shown capable of failing.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn guard(dir: &std::path::Path) -> bool {
    Command::new("bash")
        .arg(repo().join("tests/pr-spot-labels-guard.sh"))
        .arg(dir)
        .output()
        .expect("bash runs the guard")
        .status
        .success()
}

fn scratch(name: &str, ci: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("forjar-spot-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    fs::write(d.join("ci.yml"), ci).unwrap();
    d
}

#[test]
fn real_tree_passes() {
    assert!(guard(&repo().join(".github/workflows")));
}

#[test]
fn reusable_runs_on_with_spot_label_fails() {
    let ci = fs::read_to_string(repo().join(".github/workflows/ci.yml")).unwrap();
    let mutant = ci.replace(
        "runs_on: '[\"self-hosted\",\"clean-room\",\"X64\"]'",
        "runs_on: '[\"self-hosted\",\"burst-x64\",\"burst-public\"]'",
    );
    assert_ne!(ci, mutant, "mutation must apply");
    assert!(!guard(&scratch("runs-on", &mutant)));
}

#[test]
fn signing_job_on_spot_label_fails() {
    let ci = "on: pull_request\njobs:\n  sign:\n    permissions:\n      id-token: write\n    runs-on: [self-hosted, burst-x64]\n    steps: []\n";
    assert!(!guard(&scratch("sign", ci)));
    let ok = ci.replace("burst-x64", "clean-room");
    assert!(guard(&scratch("sign-ok", &ok)));
}

#[test]
fn empty_dir_fails() {
    let d = std::env::temp_dir().join(format!("forjar-spot-empty-{}", std::process::id()));
    fs::create_dir_all(&d).unwrap();
    assert!(!guard(&d));
}

fn canary() -> String {
    fs::read_to_string(repo().join(".github/workflows/pool-canary.yml")).unwrap()
}

fn canary_dir(name: &str, text: &str) -> PathBuf {
    let d = scratch(name, "on: push\njobs: {}\n");
    fs::write(d.join("pool-canary.yml"), text).unwrap();
    d
}

#[test]
fn canary_real_file_passes() {
    assert!(guard(&canary_dir("canary-ok", &canary())));
}

#[test]
fn canary_with_secret_fails() {
    let m = canary().replace("IID: ${{ inputs.iid }}", "IID: ${{ secrets.TOKEN }}");
    assert_ne!(canary(), m);
    assert!(!guard(&canary_dir("canary-secret", &m)));
}

#[test]
fn canary_with_id_token_fails() {
    let m = canary().replace("contents: read", "contents: read\n  id-token: write");
    assert_ne!(canary(), m);
    assert!(!guard(&canary_dir("canary-idtoken", &m)));
}

#[test]
fn canary_with_extra_trigger_fails() {
    let m = canary().replace(
        "on:\n  workflow_dispatch:",
        "on:\n  pull_request:\n  workflow_dispatch:",
    );
    assert_ne!(canary(), m);
    assert!(!guard(&canary_dir("canary-trigger", &m)));
}

#[test]
fn canary_with_other_runs_on_fails() {
    let m = canary().replace(
        "[self-hosted, pool-canary]",
        "[self-hosted, clean-room, X64]",
    );
    assert_ne!(canary(), m);
    assert!(!guard(&canary_dir("canary-runs-on", &m)));
}
