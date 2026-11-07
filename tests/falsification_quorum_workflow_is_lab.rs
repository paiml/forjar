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

/// Allow-list: ONLY these may appear under `on:`; anything else is a PR-path (or other) trigger.
const ALLOWED_TRIGGERS: [&str; 2] = ["schedule", "workflow_dispatch"];

/// Trigger names under `on:` (inline `[a, b]` / `a` / `{a: ..}` forms and 2-space block keys).
fn trigger_names(t: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (i, l) in t.lines().enumerate() {
        if i == 0 {
            let inline = l.trim().trim_matches(|c| "[]{}".contains(c));
            for part in inline.split(',') {
                let n = part.split(':').next().unwrap_or("").trim();
                if !n.is_empty() {
                    names.push(n.to_string());
                }
            }
        } else if l.starts_with("  ") && !l.starts_with("   ") && !l.trim_start().starts_with('-') {
            let n = l.trim().trim_end_matches(':').trim();
            if !n.is_empty() {
                names.push(n.to_string());
            }
        }
    }
    names
}

/// Every trigger that is not on the allow-list.
fn forbidden(t: &str) -> Vec<String> {
    trigger_names(t)
        .into_iter()
        .filter(|n| !ALLOWED_TRIGGERS.contains(&n.as_str()))
        .collect()
}

/// Text of every `run: |` block (lines indented deeper than the `run:` key).
fn run_blocks(w: &str) -> String {
    let mut out = String::new();
    let mut indent: Option<usize> = None;
    for l in w.lines() {
        let ind = l.len() - l.trim_start().len();
        if let Some(i) = indent {
            if l.trim().is_empty() || ind > i {
                out.push_str(l);
                out.push('\n');
                continue;
            }
            indent = None;
        }
        if l.trim_start().starts_with("run:") {
            indent = Some(ind);
        }
    }
    out
}

/// User input reaches the shell only via `env:`; no `${{ }}` expression inside any `run:` block.
fn no_expression_in_run(w: &str) -> bool {
    !run_blocks(w).contains("${{")
}

fn code_lines(w: &str) -> Vec<String> {
    run_blocks(w)
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Exactly one gate invocation, and it runs the file extracted from `$base` into `$d`.
fn runs_only_base_extracted_gate(w: &str) -> bool {
    let extract =
        r#"git show "$base:scripts/quorum-store-gate.sh" > "$d/scripts/quorum-store-gate.sh""#;
    let lines = code_lines(w);
    let others: Vec<&String> = lines
        .iter()
        .filter(|l| l.contains("quorum-store-gate.sh") && !l.contains(extract))
        .collect();
    let d_ok = lines
        .iter()
        .any(|l| l.trim() == r#"d="$RUNNER_TEMP/quorum-base-gate-$n""#);
    others.len() == 1
        && others[0].contains(r#"bash "$d/scripts/quorum-store-gate.sh""#)
        && !others[0].contains("FETCH_HEAD")
        && !others[0].contains("| bash")
        && !others[0].contains("<(")
        && lines.iter().filter(|l| l.contains(extract)).count() == 1
        && d_ok
}

/// Every `continue` and every failure branch sets `rc=1` first.
fn fails_closed(w: &str) -> bool {
    let lines = code_lines(w);
    let continues_ok = lines.iter().filter(|l| l.contains("continue")).all(|l| {
        l.split("continue")
            .next()
            .is_some_and(|b| b.contains("rc=1"))
    });
    let marked = [
        "FAIL: bad PR number",
        "FAIL: gh pr view",
        "FAIL: fetch PR",
        "base lacks the gate",
    ]
    .iter()
    .all(|m| {
        let hit: Vec<&String> = lines.iter().filter(|l| l.contains(m)).collect();
        !hit.is_empty() && hit.iter().all(|l| l.contains("rc=1; continue"))
    });
    continues_ok && marked
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
        "on:\n  schedule:\n    - cron: \"1 2 * * *\"\n  workflow_run:\njobs:\n",
        "on:\n  workflow_dispatch:\n  pull_request_target:\njobs:\n",
        "on:\n  workflow_dispatch:\n  issue_comment:\njobs:\n",
        "on: [push]\njobs:\n",
        "on:\n  push:\n    branches: [main]\njobs:\n",
        "on:\n  workflow_dispatch:\n  release:\njobs:\n",
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

/// The pre-loop check on the untrusted `INPUT_PR` itself (the in-loop regex on `$n` is separate).
fn validates_input_before_loop(w: &str) -> bool {
    let (Some(v), Some(l)) = (
        w.find(r#"[[ "$INPUT_PR" =~ ^[0-9]+$ ]]"#),
        w.find(r#"for n in "${prs[@]}""#),
    ) else {
        return false;
    };
    v < l
}

/// Fail-closed listing: gh output is captured with an explicit exit, never fed through a process
/// substitution (whose failure is invisible to `||`).
fn lists_prs_fail_closed(w: &str) -> bool {
    !w.contains("< <(gh pr list")
        && w.contains("$(gh pr list")
        && w.contains("--limit 1000")
        && w.contains("(UNMEASURED)'; exit 1; }")
        && w.contains("no open PRs; nothing checked")
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

#[test]
fn input_validation_precedes_loop_and_listing_fails_closed() {
    let w = workflow();
    assert!(
        validates_input_before_loop(&w),
        "INPUT_PR must be validated before the loop"
    );
    assert!(lists_prs_fail_closed(&w), "gh pr list must fail closed");
    // mutant: delete the pre-loop INPUT_PR check (in-loop regex remains)
    let no_pre = w.replace(r#"[[ "$INPUT_PR" =~ ^[0-9]+$ ]]"#, "true");
    assert!(
        !validates_input_before_loop(&no_pre),
        "deleted pre-loop validation missed"
    );
    // mutant: fail-open process substitution feeding mapfile
    let open = w.replace("out=$(gh pr list", "mapfile -t prs < <(gh pr list");
    assert!(!lists_prs_fail_closed(&open), "fail-open shape missed");
    // mutant: capture without the explicit exit
    let no_exit = w.replace("(UNMEASURED)'; exit 1; }", "(UNMEASURED)'; }");
    assert!(!lists_prs_fail_closed(&no_exit), "missing exit 1 missed");
    let no_limit = w.replace("--limit 1000", "");
    assert!(!lists_prs_fail_closed(&no_limit), "missing --limit missed");
}

#[test]
fn live_workflow_passes_structural_checks() {
    let w = workflow();
    assert!(runs_only_base_extracted_gate(&w), "gate invocation shape");
    assert!(fails_closed(&w), "every failure branch sets rc=1 first");
    assert!(no_expression_in_run(&w), "no ${{ }} inside run:");
    assert!(
        w.contains("INPUT_PR: ${{ inputs.pr_number }}"),
        "input must arrive via env:"
    );
}

#[test]
fn gate_invocation_mutants_are_flagged() {
    let w = workflow();
    let run = r#"bash "$d/scripts/quorum-store-gate.sh""#;
    assert!(w.contains(run));
    for m in [
        w.replace(
            run,
            "git show FETCH_HEAD:scripts/quorum-store-gate.sh | bash",
        ),
        w.replace(run, r#"bash scripts/quorum-store-gate.sh"#),
        w.replace(run, r#"bash "$PWD/scripts/quorum-store-gate.sh""#),
        w.replace(
            run,
            r#"bash <(git show "$head:scripts/quorum-store-gate.sh")"#,
        ),
        // a second invocation alongside the legitimate one
        w.replace(
            "done\n          exit",
            "done\n          bash scripts/quorum-store-gate.sh\n          exit",
        ),
        w.replace(r#"d="$RUNNER_TEMP/quorum-base-gate-$n""#, r#"d="$PWD""#),
    ] {
        assert!(!runs_only_base_extracted_gate(&m), "missed:\n{m}");
    }
}

#[test]
fn fail_closed_mutants_are_flagged() {
    let w = workflow();
    // drop rc=1 on each branch in turn (base-missing, gh pr view, fetch, bad number)
    for from in [
        r#"base lacks the gate or allowlist" >&2; rc=1; continue;"#,
        r#"FAIL: gh pr view $n (UNMEASURED)" >&2; rc=1; continue;"#,
        r#"FAIL: fetch PR $n" >&2; rc=1; continue;"#,
        r#"FAIL: bad PR number" >&2; rc=1; continue;"#,
    ] {
        assert!(w.contains(from), "fixture drifted: {from}");
        let to = from.replace("rc=1; ", "");
        let m = w.replace(from, &to);
        assert!(!fails_closed(&m), "dropped rc=1 missed: {from}");
    }
    // a new bare `continue` is flagged
    let m = w.replace(
        "done\n          exit",
        "  continue\n          done\n          exit",
    );
    assert!(!fails_closed(&m), "bare continue missed");
}

#[test]
fn expression_in_run_mutants_are_flagged() {
    let w = workflow();
    for m in [
        w.replace(
            r#"[ -n "$INPUT_PR" ]"#,
            r#"[ -n "${{ inputs.pr_number }}" ]"#,
        ),
        w.replace(r#"prs=("$INPUT_PR")"#, r#"prs=(${{ inputs.pr_number }})"#),
        w.replace(
            r#"echo "== PR #$n head $head""#,
            r#"echo "${{ github.event.inputs.pr_number }}""#,
        ),
        w.replace(
            r#"set -uo pipefail"#,
            "set -uo pipefail\n          echo ${{ github.event.pull_request.title }}",
        ),
    ] {
        assert!(!no_expression_in_run(&m), "inlined expression missed");
    }
}
