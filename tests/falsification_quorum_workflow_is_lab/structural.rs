use crate::common::*;

#[test]
pub(crate) fn quorum_has_no_pr_path_trigger() {
    let t = triggers();
    assert!(forbidden(&t).is_empty(), "PR-path trigger in on:\n{t}");
}

#[test]
pub(crate) fn parser_catches_inline_and_block_forms() {
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
pub(crate) fn quorum_runs_nightly_and_by_hand_with_pr_number_input() {
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
pub(crate) fn quorum_job_sets_pr_number_for_the_gate() {
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

pub(crate) fn reads_gate_from_base(w: &str) -> bool {
    w.contains(r#"git show "$base:scripts/quorum-store-gate.sh""#)
        && !w.contains(r#"git show "$head:"#)
}

pub(crate) fn propagates_failure(w: &str) -> bool {
    w.contains("|| rc=1") && w.contains(r#"exit "$rc""#)
}

pub(crate) fn validates_pr_number(w: &str) -> bool {
    w.contains("=~ ^[0-9]+$") && w.contains(r#"for n in "${prs[@]}""#)
}

/// The pre-loop check on the untrusted `INPUT_PR` itself (the in-loop regex on `$n` is separate).
pub(crate) fn validates_input_before_loop(w: &str) -> bool {
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
pub(crate) fn lists_prs_fail_closed(w: &str) -> bool {
    !w.contains("< <(gh pr list")
        && w.contains("$(gh pr list")
        && w.contains("--limit 1000")
        && w.contains("(UNMEASURED)'; exit 1; }")
        && w.contains("no open PRs; nothing checked")
}

#[test]
pub(crate) fn quorum_judges_from_base_and_propagates_failure() {
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
pub(crate) fn base_and_failure_mutants_are_flagged() {
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
pub(crate) fn input_validation_precedes_loop_and_listing_fails_closed() {
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
pub(crate) fn live_workflow_passes_structural_checks() {
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
pub(crate) fn gate_invocation_mutants_are_flagged() {
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
pub(crate) fn fail_closed_mutants_are_flagged() {
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
pub(crate) fn expression_in_run_mutants_are_flagged() {
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
