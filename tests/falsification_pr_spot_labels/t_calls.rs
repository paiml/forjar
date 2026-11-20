//! Falsifiers (calls).

use crate::*;

/// A new workflow `x.yml` holding one job that calls `uses`, plus extra job lines.
pub(crate) fn call_file(on: &str, uses: &str, extra: &str) -> String {
    format!("on: {on}\njobs:\n  c:\n    uses: {uses}\n{extra}")
}

pub(crate) const EXT_BAD: &str =
    "someone/else/.github/workflows/w.yml@0123456789012345678901234567890123456789";
pub(crate) const EXT_OK_CI: &str =
    "paiml/.github/.github/workflows/sovereign-ci.yml@8adbee1d793df71a31867eb875829b55e04a7c02";
pub(crate) const EXT_OK_GATE: &str =
    "paiml/.github/.github/workflows/pr-gate.yml@fbee3788a0ed92d09974ac21ea9bc97d373af6f0";

#[test]
fn r6_external_call_not_on_allowlist_fails_on_every_trigger() {
    for (i, on) in [
        "pull_request",
        "issue_comment",
        "push",
        "schedule",
        "workflow_dispatch",
    ]
    .iter()
    .enumerate()
    {
        for (k, extra) in [
            "",
            "    secrets: inherit\n",
            "    with:\n      labels: '[\"self-hosted\",\"unpooled\"]'\n",
            "    secrets: inherit\n    with:\n      labels: '[\"self-hosted\",\"unpooled\"]'\n",
            "    with:\n      repo: x\n",
        ]
        .iter()
        .enumerate()
        {
            r5_red(
                &format!("r6-ext-{i}-{k}"),
                &call_file(on, EXT_BAD, extra),
                "not on the external-call allowlist",
            );
        }
    }
    for (n, u) in [
        ("expr", "${{ vars.W }}"),
        ("docker", "docker://alpine:3"),
        (
            "other-ref",
            "paiml/.github/.github/workflows/sovereign-ci.yml@main",
        ),
        (
            "other-file",
            "paiml/.github/.github/workflows/evil.yml@8adbee1d793df71a31867eb875829b55e04a7c02",
        ),
    ] {
        r5_red(
            &format!("r6-ext-{n}"),
            &call_file("pull_request", u, ""),
            "not on the external-call allowlist",
        );
    }
}

/// A scratch workflow dir holding `content` at `<file>`, over the benign ci.yml.
pub(crate) fn with_file(name: &str, file: &str, content: &str) -> PathBuf {
    let d = scratch(name, "on: push\njobs: {}\n");
    fs::write(d.join(file), content).unwrap();
    d
}

pub(crate) const BENCH: &str =
    "paiml/.github/.github/workflows/sovereign-ci.yml@fbee3788a0ed92d09974ac21ea9bc97d373af6f0";
pub(crate) const CI_RUNS_ON: &str = "      runs_on: '[\"self-hosted\",\"clean-room\",\"X64\"]'\n";

/// RED-first: each plant is the allowed call with ONE scoping dimension changed.
fn site_job(file: &str) -> &str {
    match file {
        "ci.yml" => "ci",
        "nightly-bench.yml" => "bench",
        "pr-gate.yml" => "authorize",
        _ => "c",
    }
}

fn site_text(file: &str, uses: &str, extra: &str) -> String {
    call_file("push", uses, extra).replacen("\n  c:\n", &format!("\n  {}:\n", site_job(file)), 1)
}

fn ext_red(name: &str, file: &str, uses: &str, extra: &str, needle: &str) {
    let d = with_file(name, file, &site_text(file, uses, extra));
    match judge(&d, &expected_canary()) {
        Ok(_) => panic!("r7 plant {name} was accepted"),
        Err(w) => assert!(
            w.iter().any(|m| m.contains(needle)),
            "r7 plant {name} refused for the wrong reason (want `{needle}`): {w:?}"
        ),
    }
}

fn ext_ok(name: &str, file: &str, uses: &str, extra: &str) {
    let d = with_file(name, file, &site_text(file, uses, extra));
    let r = judge(&d, &expected_canary());
    assert!(r.is_ok(), "{name} refused: {r:?}");
}

#[test]
fn r7_each_allowlisted_call_passes_at_its_own_site() {
    ext_ok(
        "r7-ok-bench",
        "nightly-bench.yml",
        BENCH,
        "    with:\n      repo: x\n      run_benchmarks: true\n    secrets: inherit\n",
    );
    ext_ok("r7-ok-bench-bare", "nightly-bench.yml", BENCH, "");
    ext_ok("r7-ok-gate", "pr-gate.yml", EXT_OK_GATE, "");
    let ci = format!("    with:\n      repo: x\n{CI_RUNS_ON}    secrets: inherit\n");
    ext_ok("r7-ok-ci", "ci.yml", EXT_OK_CI, &ci);
}

#[test]
fn r7_allowlist_is_scoped_to_site_uses_secrets_and_inputs() {
    // wrong call site for an allowed uses string
    ext_red("r7-site-file", "x.yml", BENCH, "", "call site");
    ext_red("r7-site-file2", "pr-gate.yml", BENCH, "", "call site");
    // secrets forms: gate takes none; bench takes only the exact string
    ext_red(
        "r7-gate-inherit",
        "pr-gate.yml",
        EXT_OK_GATE,
        "    secrets: inherit\n",
        "secrets form",
    );
    ext_red(
        "r7-gate-map",
        "pr-gate.yml",
        EXT_OK_GATE,
        "    secrets:\n      T: x\n",
        "secrets form",
    );
    ext_red(
        "r7-bench-map",
        "nightly-bench.yml",
        BENCH,
        "    secrets:\n      T: x\n",
        "secrets form",
    );
    ext_red(
        "r7-bench-case",
        "nightly-bench.yml",
        BENCH,
        "    secrets: INHERIT\n",
        "secrets form",
    );
    // inputs: unlisted is red, on every entry
    ext_red(
        "r7-gate-input",
        "pr-gate.yml",
        EXT_OK_GATE,
        "    with:\n      repo: x\n",
        "not allowed",
    );
    ext_red(
        "r7-bench-input",
        "nightly-bench.yml",
        BENCH,
        "    with:\n      cmd: x\n",
        "not allowed",
    );
    ext_red(
        "r7-bench-runs-on",
        "nightly-bench.yml",
        BENCH,
        &format!("    with:\n{CI_RUNS_ON}"),
        "not allowed",
    );
    ext_red(
        "r7-bench-labels",
        "nightly-bench.yml",
        BENCH,
        "    with:\n      labels: x\n",
        "not allowed",
    );
    // ci: required runs_on must be exact and present
    ext_red(
        "r7-ci-missing",
        "ci.yml",
        EXT_OK_CI,
        "    with:\n      repo: x\n    secrets: inherit\n",
        "required input",
    );
    ext_red(
        "r7-ci-other",
        "ci.yml",
        EXT_OK_CI,
        "    with:\n      runs_on: '[\"self-hosted\"]'\n    secrets: inherit\n",
        "required input",
    );
    ext_red(
        "r7-ci-extra",
        "ci.yml",
        EXT_OK_CI,
        &format!("    with:\n{CI_RUNS_ON}      cmd: x\n    secrets: inherit\n"),
        "not allowed",
    );
    // a pin that is not the committed one is not on the list
    ext_red(
        "r7-pin",
        "ci.yml",
        "paiml/.github/.github/workflows/sovereign-ci.yml@0123456789012345678901234567890123456789",
        "",
        "not on the external-call allowlist",
    );
}

#[test]
fn r7_allowlist_table_is_self_consistent() {
    for e in EXTERNAL_CALLS.iter() {
        assert!(
            e.uses.contains('@') && e.uses.len() > 40,
            "{} is not pinned",
            e.uses
        );
        assert!(!e.why.is_empty());
        assert!(e.required.iter().all(|(k, _)| !e.optional.contains(k)));
        assert!(e.optional.iter().all(|k| !k.contains("runs")));
    }
}

/// x.yml calls ./.github/workflows/callee.yml; callee.yml holds `callee`.
pub(crate) fn with_callee(name: &str, caller: &str, callee: &str) -> PathBuf {
    let d = with_x(name, caller);
    fs::write(d.join("callee.yml"), callee).unwrap();
    d
}

pub(crate) fn r6_local_red(name: &str, caller: &str, callee: &str, needle: &str) {
    match judge(&with_callee(name, caller, callee), &expected_canary()) {
        Ok(_) => panic!("r6 plant {name} was accepted"),
        Err(w) => assert!(
            w.iter().any(|m| m.contains(needle)),
            "r6 plant {name} refused for the wrong reason (want `{needle}`): {w:?}"
        ),
    }
}

pub(crate) const LOCAL: &str = "./.github/workflows/callee.yml";

#[test]
fn r6_local_reusable_call_is_judged_with_the_callers_context() {
    let callee_spot =
        "on: workflow_call\njobs:\n  j:\n    runs-on: [self-hosted, unpooled]\n    steps: []\n";
    for on in ["pull_request", "issue_comment", "push", "schedule"] {
        r6_local_red(
            &format!("r6-local-{on}"),
            &call_file(on, LOCAL, ""),
            callee_spot,
            "via local call",
        );
    }
    let callee_in = "on:\n  workflow_call:\n    inputs:\n      l:\n        type: string\njobs:\n  j:\n    runs-on: ${{ inputs.l }}\n    steps: []\n";
    r6_local_red(
        "r6-local-input-spot",
        &call_file(
            "push",
            LOCAL,
            "    with:\n      l: '[\"self-hosted\",\"unpooled\"]'\n",
        ),
        callee_in,
        "via local call",
    );
    r6_local_red(
        "r6-local-input-missing",
        &call_file("push", LOCAL, ""),
        callee_in,
        "via local call",
    );
    r6_local_red(
        "r6-local-inherit",
        &call_file("push", LOCAL, "    secrets: inherit\n"),
        "on: workflow_call\njobs:\n  j:\n    runs-on: [self-hosted, clean-room]\n    steps: []\n",
        "secrets passed",
    );
    for (n, u) in [
        ("missing", "./.github/workflows/absent.yml"),
        ("dotdot", "./.github/workflows/../../x.yml"),
        ("subdir", "./.github/workflows/sub/callee.yml"),
        ("action-dir", "./.github/actions/changed-class"),
        ("self-loop", "./.github/workflows/x.yml"),
    ] {
        r5_red(
            &format!("r6-local-{n}"),
            &call_file("push", u, ""),
            "cannot judge",
        );
    }
}

#[test]
fn r6_local_reusable_call_on_exclusive_labels_passes() {
    let d = with_callee(
        "r6-local-ok",
        &call_file("issue_comment", LOCAL, ""),
        "on: workflow_call\njobs:\n  j:\n    runs-on: [self-hosted, clean-room, X64]\n    steps: []\n",
    );
    assert!(judge(&d, &expected_canary()).is_ok());
}

#[test]
fn r7_wrong_job_id_at_the_right_file_is_red() {
    let t = call_file("push", EXT_OK_GATE, "");
    let d = with_file("r7-job-id", "pr-gate.yml", &t);
    let w = judge(&d, &expected_canary()).unwrap_err();
    assert!(w.iter().any(|m| m.contains("call site")), "{w:?}");
}
