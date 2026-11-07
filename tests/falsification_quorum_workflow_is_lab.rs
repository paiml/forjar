//! C58-3 / rule C324: the `Quorum` workflow cannot block a merge (the required ruleset context is
//! only `gate`), so it is LAB: no PR-path trigger. It runs nightly (`schedule`) and by hand
//! (`workflow_dispatch` with an optional `pr_number`). forjar#862.

use std::path::Path;

fn workflow() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/quorum.yml"),
    )
    .expect("quorum.yml")
}

/// The `on:` value: any inline text after `on:` plus the indented block under it.
fn triggers_of(w: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for l in w.lines() {
        if let Some(rest) = l.strip_prefix("on:") {
            inside = true;
            out.push_str(&strip_comment(rest));
            out.push('\n');
            continue;
        }
        if inside && !l.is_empty() && !l.starts_with(' ') && !l.starts_with('#') {
            break;
        }
        if inside && !l.trim_start().starts_with('#') {
            out.push_str(&strip_comment(l));
            out.push('\n');
        }
    }
    out
}

fn strip_comment(l: &str) -> String {
    l.split(" #").next().unwrap_or("").to_string()
}

fn forbidden(t: &str) -> Vec<&'static str> {
    ["pull_request", "pull_request_target", "push", "merge_group"]
        .into_iter()
        .filter(|b| t.contains(b))
        .collect()
}

fn triggers() -> String {
    triggers_of(&workflow())
}

#[test]
fn quorum_has_no_pr_path_trigger() {
    let t = triggers();
    assert!(forbidden(&t).is_empty(), "PR-path trigger in on:\n{t}");
}

#[test]
fn parser_catches_inline_and_block_forms() {
    // planted mutants: each must be flagged
    for m in [
        "on: [pull_request]\njobs:\n",
        "on: pull_request\njobs:\n",
        "on: [workflow_dispatch, push]\njobs:\n",
        "on:\n  pull_request:\n  workflow_dispatch:\njobs:\n",
        "on:\n  merge_group:\njobs:\n",
    ] {
        assert!(!forbidden(&triggers_of(m)).is_empty(), "missed: {m}");
    }
    // a clean one is not flagged
    let ok = "on:\n  schedule:\n    - cron: \"1 2 * * *\"\n  workflow_dispatch:\njobs:\n";
    assert!(forbidden(&triggers_of(ok)).is_empty());
}

#[test]
fn quorum_runs_nightly_and_by_hand_with_pr_number_input() {
    let t = triggers();
    assert!(
        t.contains("schedule:") && t.contains("cron:"),
        "schedule/cron missing:\n{t}"
    );
    assert!(
        t.contains("workflow_dispatch"),
        "workflow_dispatch missing:\n{t}"
    );
    assert!(
        t.contains("pr_number:"),
        "workflow_dispatch pr_number input missing:\n{t}"
    );
}

#[test]
fn quorum_job_sets_pr_number_for_the_gate() {
    let w = workflow();
    assert!(
        w.contains("PR_NUMBER=\"$n\""),
        "gate must be given PR_NUMBER"
    );
    assert!(
        w.contains("PR_HEAD_SHA=\"$head\""),
        "gate must be given PR_HEAD_SHA"
    );
    assert!(
        !w.contains("github.event.pull_request"),
        "no pull_request event context may remain"
    );
}

fn reads_gate_from_base(w: &str) -> bool {
    w.contains(r#"git show "$base:scripts/quorum-store-gate.sh""#)
        && !w.contains(r#"git show "$head:"#)
}

fn propagates_failure(w: &str) -> bool {
    w.contains("|| rc=1") && w.contains(r#"exit "$rc""#)
}

fn validates_pr_number(w: &str) -> bool {
    w.contains("=~ ^[0-9]+$") && w.contains(r#"for n in "${prs[@]}""#)
}

#[test]
fn quorum_judges_from_base_and_propagates_failure() {
    let w = workflow();
    assert!(
        reads_gate_from_base(&w),
        "gate must come from the base commit"
    );
    assert!(propagates_failure(&w), "a failing PR must fail the run");
    assert!(
        validates_pr_number(&w),
        "pr_number must be validated and iterated as an array"
    );
    assert!(
        !w.contains("rust-toolchain") && !w.contains("rust-cache"),
        "bash-only job"
    );
    assert!(
        w.contains("quorum-lab-${{ github.event_name }}"),
        "nightly and dispatch need distinct concurrency groups"
    );
}

#[test]
fn base_and_failure_mutants_are_flagged() {
    let w = workflow();
    let head = w.replace(r#"git show "$base:"#, r#"git show "$head:"#);
    assert!(!reads_gate_from_base(&head), "head-read mutant missed");
    let no_rc = w.replace("|| rc=1", "|| true");
    assert!(!propagates_failure(&no_rc), "removed `|| rc=1` missed");
    let no_exit = w.replace(r#"exit "$rc""#, "exit 0");
    assert!(
        !propagates_failure(&no_exit),
        "removed `exit \"$rc\"` missed"
    );
    let unvalidated = w.replace("=~ ^[0-9]+$", "=~ .");
    assert!(
        !validates_pr_number(&unvalidated),
        "dropped validation missed"
    );
    let split = w.replace(r#"for n in "${prs[@]}""#, "for n in $prs");
    assert!(!validates_pr_number(&split), "word-split iteration missed");
}
