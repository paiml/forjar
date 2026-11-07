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

// ---- C58-3 round 4: planted mutants for the remaining workflow invariants ----

/// Workflow lines with comments and blanks removed.
fn yaml_lines(w: &str) -> Vec<String> {
    w.lines()
        .map(strip_comment)
        .filter(|l| !l.trim().is_empty())
        .collect()
}

/// The zero-PR guard is exactly `[ "${#prs[@]}" -eq 0 ]`.
fn zero_pr_guard_is_exact(w: &str) -> bool {
    let guards: Vec<String> = code_lines(w)
        .into_iter()
        .filter(|l| l.contains("${#prs[@]}"))
        .collect();
    guards.len() == 1 && guards[0].contains(r#"[ "${#prs[@]}" -eq 0 ]"#)
}

/// Indented lines directly under a top-level `permissions:` key, or None if it is inline.
fn permission_entries(lines: &[String]) -> Option<Vec<String>> {
    let at = lines.iter().position(|l| l.starts_with("permissions:"))?;
    if lines[at].trim() != "permissions:" {
        return None;
    }
    let mut v: Vec<String> = lines[at + 1..]
        .iter()
        .take_while(|l| l.starts_with(' '))
        .map(|l| l.trim().to_string())
        .collect();
    v.sort();
    Some(v)
}

/// Only `contents: read` and `pull-requests: read` under `permissions:`; no write anywhere.
fn permissions_are_read_only(w: &str) -> bool {
    let lines = yaml_lines(w);
    !lines.iter().any(|l| l.contains("write"))
        && permission_entries(&lines)
            .is_some_and(|p| p == ["contents: read", "pull-requests: read"])
}

/// Every `secrets.` reference is `secrets.GITHUB_TOKEN`.
fn secrets_are_only_github_token(w: &str) -> bool {
    yaml_lines(w).iter().all(|l| {
        l.split("secrets.")
            .skip(1)
            .all(|r| r.starts_with("GITHUB_TOKEN"))
    })
}

fn names_pr_code(t: &str) -> bool {
    ["FETCH_HEAD", "refs/pull", "pull/"]
        .iter()
        .any(|k| t.contains(k))
}

fn changes_tree(t: &str) -> bool {
    [
        "git checkout",
        "git switch",
        "git reset",
        "git merge",
        "git worktree",
    ]
    .iter()
    .any(|k| t.contains(k))
}

/// PR code never becomes the working tree: no checkout/switch/reset of FETCH_HEAD or pull refs,
/// and no `ref:` input other than the default branch.
fn pr_code_never_checked_out(w: &str) -> bool {
    yaml_lines(w).iter().all(|l| {
        let t = l.trim();
        let ref_input = t.starts_with("ref:");
        !(names_pr_code(t) && (changes_tree(t) || ref_input))
            && !(ref_input && !t.contains("default_branch"))
    })
}

/// Cron is a single daily schedule: 5 fields, numeric minute and hour, `*` for the rest.
fn cron_is_daily(w: &str) -> bool {
    let crons: Vec<String> = yaml_lines(w)
        .into_iter()
        .filter(|l| l.contains("cron:"))
        .collect();
    if crons.len() != 1 {
        return false;
    }
    let v = crons[0].split("cron:").nth(1).unwrap_or("").trim();
    let v = v.trim_matches('"').trim_matches('\'');
    let f: Vec<&str> = v.split_whitespace().collect();
    f.len() == 5
        && f[0].parse::<u8>().is_ok_and(|m| m < 60)
        && f[1].parse::<u8>().is_ok_and(|h| h < 24)
        && f[2..].iter().all(|x| *x == "*")
}

fn runs_on_is_clean_room(w: &str) -> bool {
    let r: Vec<String> = yaml_lines(w)
        .into_iter()
        .filter(|l| l.trim_start().starts_with("runs-on:"))
        .collect();
    r.len() == 1 && r[0].trim() == "runs-on: [self-hosted, clean-room, ARM64]"
}

/// Every `uses:` is pinned to a `vN...` tag or a 40-hex sha, never a branch.
fn uses_are_pinned(w: &str) -> bool {
    yaml_lines(w)
        .iter()
        .filter_map(|l| l.split("uses:").nth(1))
        .all(|u| {
            let u = u.trim();
            match u.split_once('@') {
                Some((_, r)) => {
                    let sha = r.len() == 40 && r.chars().all(|c| c.is_ascii_hexdigit());
                    let tag =
                        r.starts_with('v') && r[1..].starts_with(|c: char| c.is_ascii_digit());
                    sha || tag
                }
                None => false,
            }
        })
}

#[test]
fn live_workflow_passes_hardening_checks() {
    let w = workflow();
    assert!(zero_pr_guard_is_exact(&w), "zero-PR guard");
    assert!(permissions_are_read_only(&w), "permissions");
    assert!(secrets_are_only_github_token(&w), "secrets");
    assert!(pr_code_never_checked_out(&w), "PR code checked out");
    assert!(cron_is_daily(&w), "cron");
    assert!(runs_on_is_clean_room(&w), "runs-on");
    assert!(uses_are_pinned(&w), "uses pinning");
    assert!(w.contains("actions/checkout@v4"), "checkout still present");
}

#[test]
fn hardening_mutants_are_flagged() {
    let w = workflow();
    let guard = r#"[ "${#prs[@]}" -eq 0 ]"#;
    let cron = r#"cron: "17 3 * * *""#;
    let runs = "runs-on: [self-hosted, clean-room, ARM64]";
    let perm = "  pull-requests: read";
    for (what, m, check) in [
        (
            "guard -le 1",
            w.replace(guard, r#"[ "${#prs[@]}" -le 1 ]"#),
            zero_pr_guard_is_exact as fn(&str) -> bool,
        ),
        (
            "guard -ge 0",
            w.replace(guard, r#"[ "${#prs[@]}" -ge 0 ]"#),
            zero_pr_guard_is_exact,
        ),
        (
            "pr write",
            w.replace(perm, "  pull-requests: write"),
            permissions_are_read_only,
        ),
        (
            "contents write",
            w.replace("  contents: read", "  contents: write"),
            permissions_are_read_only,
        ),
        (
            "write-all",
            w.replace(
                "permissions:\n  contents: read\n  pull-requests: read",
                "permissions: write-all",
            ),
            permissions_are_read_only,
        ),
        (
            "extra perm",
            w.replace(perm, "  pull-requests: read\n  issues: read"),
            permissions_are_read_only,
        ),
        (
            "other secret",
            w.replace("${{ github.token }}", "${{ secrets.PAT }}"),
            secrets_are_only_github_token,
        ),
        (
            "secrets inherit",
            w.replace("INPUT_PR:", "X: ${{ secrets.GH_PAT }}\n          INPUT_PR:"),
            secrets_are_only_github_token,
        ),
        (
            "checkout FETCH_HEAD",
            w.replace(
                "          set -uo pipefail",
                "          set -uo pipefail\n          git checkout FETCH_HEAD",
            ),
            pr_code_never_checked_out,
        ),
        (
            "checkout refs/pull",
            w.replace(
                "          set -uo pipefail",
                "          set -uo pipefail\n          git checkout refs/pull/1/head",
            ),
            pr_code_never_checked_out,
        ),
        (
            "ref input",
            w.replace(
                "          fetch-depth: 0",
                "          fetch-depth: 0\n          ref: refs/pull/1/merge",
            ),
            pr_code_never_checked_out,
        ),
        (
            "cron every minute",
            w.replace(cron, r#"cron: "* * * * *""#),
            cron_is_daily,
        ),
        (
            "cron hourly",
            w.replace(cron, r#"cron: "17 * * * *""#),
            cron_is_daily,
        ),
        (
            "cron sub-daily",
            w.replace(cron, r#"cron: "*/5 3 * * *""#),
            cron_is_daily,
        ),
        (
            "cron weekday",
            w.replace(cron, r#"cron: "17 3 * * 1""#),
            cron_is_daily,
        ),
        (
            "cron 6 fields",
            w.replace(cron, r#"cron: "17 3 * * * *""#),
            cron_is_daily,
        ),
        (
            "second cron",
            w.replace(cron, &format!("{cron}\n    - cron: \"5 4 * * *\"")),
            cron_is_daily,
        ),
        (
            "hosted runner",
            w.replace(runs, "runs-on: ubuntu-latest"),
            runs_on_is_clean_room,
        ),
        (
            "no ARM64",
            w.replace(runs, "runs-on: [self-hosted, clean-room]"),
            runs_on_is_clean_room,
        ),
        (
            "uses @main",
            w.replace("actions/checkout@v4", "actions/checkout@main"),
            uses_are_pinned,
        ),
        (
            "uses @master",
            w.replace("actions/checkout@v4", "actions/checkout@master"),
            uses_are_pinned,
        ),
        (
            "uses unpinned",
            w.replace("actions/checkout@v4", "actions/checkout"),
            uses_are_pinned,
        ),
    ] {
        assert!(w.contains("actions/checkout@v4"));
        assert_ne!(m, w, "mutant did not apply: {what}");
        assert!(!check(&m), "mutant missed: {what}");
    }
}

// ---- exact-content pin: the whole file, so pattern-shaped tests cannot be slipped past ----

const PIN_HELP: &str = "changing the LAB workflow .github/workflows/quorum.yml requires updating \
tests/fixtures/quorum-lab-workflow.pinned.yml in the same PR, with a non-author quorum round";

/// Normalise only line endings and trailing whitespace (and trailing blank lines).
fn normalise(w: &str) -> String {
    let mut s: String = w
        .replace("\r\n", "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    s.push('\n');
    while s.ends_with("\n\n") {
        s.pop();
    }
    s
}

fn pinned() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/quorum-lab-workflow.pinned.yml"),
    )
    .expect("pinned fixture")
}

fn matches_pin(w: &str) -> bool {
    normalise(w) == normalise(&pinned())
}

/// Cheap explicit checks so a failure names its cause. Returns the first cause found.
fn explicit_cause(w: &str) -> Option<&'static str> {
    let y = yaml_lines(w);
    let has = |f: &dyn Fn(&str) -> bool| y.iter().any(|l| f(l));
    if has(&|l| l.contains("continue-on-error")) {
        return Some("continue-on-error");
    }
    if has(&|l| {
        let t = l.trim_start().trim_start_matches("- ");
        t.starts_with("if:")
    }) {
        return Some("job/step-level if:");
    }
    if has(&|l| {
        let t = l.trim();
        t == "gate:" || t == "name: gate" || t == "name: 'gate'" || t == "name: \"gate\""
    }) {
        return Some("job id/name gate");
    }
    if has(&|l| l.contains("toJSON(secrets") || l.contains("toJson(secrets")) {
        return Some("toJSON(secrets");
    }
    if !secrets_are_only_github_token(w) {
        return Some("secret other than GITHUB_TOKEN");
    }
    if has(&|l| {
        l.split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|x| x == "eval")
    }) {
        return Some("eval");
    }
    None
}

#[test]
fn workflow_equals_its_pin_byte_for_byte() {
    let w = workflow();
    assert_eq!(explicit_cause(&w), None, "explicit LAB check failed");
    assert!(matches_pin(&w), "{PIN_HELP}");
}

#[test]
fn pin_and_explicit_checks_flag_planted_mutants() {
    let w = workflow();
    // (name, mutant, explicit cause that must name it, if any)
    let cases: Vec<(&str, String, Option<&str>)> = vec![
        (
            "continue-on-error job",
            w.replace(
                "    name: quorum receipt\n",
                "    name: quorum receipt\n    continue-on-error: true\n",
            ),
            Some("continue-on-error"),
        ),
        (
            "continue-on-error step",
            w.replace(
                "      - name: Verify quorum receipt for each PR\n",
                "      - name: Verify quorum receipt for each PR\n        continue-on-error: true\n",
            ),
            Some("continue-on-error"),
        ),
        (
            "job if",
            w.replace(
                "    name: quorum receipt\n",
                "    name: quorum receipt\n    if: false\n",
            ),
            Some("job/step-level if:"),
        ),
        (
            "step if",
            w.replace(
                "      - name: Verify quorum receipt for each PR\n",
                "      - name: Verify quorum receipt for each PR\n        if: always()\n",
            ),
            Some("job/step-level if:"),
        ),
        (
            "exit 0 after rc",
            w.replace(r#"exit "$rc""#, r#"exit "$rc"; exit 0"#),
            None,
        ),
        (
            "state closed",
            w.replace("--state open", "--state closed"),
            None,
        ),
        (
            "trailing limit 1",
            w.replace("--limit 1000", "--limit 1000 --limit 1"),
            None,
        ),
        (
            "out emptied",
            w.replace("            prs=()\n", "            out=\"\"\n            prs=()\n"),
            None,
        ),
        (
            "QUORUM_STORE env",
            w.replace(
                "          INPUT_PR: ${{ inputs.pr_number }}\n",
                "          INPUT_PR: ${{ inputs.pr_number }}\n          QUORUM_STORE: /tmp/x\n",
            ),
            None,
        ),
        (
            "toJSON(secrets)",
            w.replace("${{ github.token }}", "${{ toJSON(secrets) }}"),
            Some("toJSON(secrets"),
        ),
        (
            "eval gh pr view",
            w.replace(
                "read -r head base < <(gh pr view",
                "eval \"$(gh pr view",
            ),
            Some("eval"),
        ),
        (
            "base=head",
            w.replace(
                "            echo \"== PR #$n head $head\"\n",
                "            base=$head\n            echo \"== PR #$n head $head\"\n",
            ),
            None,
        ),
        (
            "jq headRefOid twice",
            w.replace(
                r#"\(.headRefOid) \(.baseRefOid)"#,
                r#"\(.headRefOid) \(.headRefOid)"#,
            ),
            None,
        ),
        (
            "job name gate",
            w.replace("    name: quorum receipt\n", "    name: gate\n"),
            Some("job id/name gate"),
        ),
        (
            "job id gate",
            w.replace("  receipt:\n", "  gate:\n"),
            Some("job id/name gate"),
        ),
        (
            "other secret",
            w.replace("${{ github.token }}", "${{ secrets.PAT }}"),
            Some("secret other than GITHUB_TOKEN"),
        ),
    ];
    assert!(cases.len() >= 12);
    for (what, m, cause) in cases {
        assert_ne!(m, w, "mutant did not apply: {what}");
        assert!(!matches_pin(&m), "pin missed mutant: {what}");
        if let Some(c) = cause {
            assert_eq!(
                explicit_cause(&m),
                Some(c),
                "explicit check wrong for: {what}"
            );
        }
    }
    // normalisation only forgives line endings and trailing whitespace
    assert!(matches_pin(&w.replace('\n', "  \r\n")));
    assert!(!matches_pin(&w.replace("exit 0", "exit  0")));
}
