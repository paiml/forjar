//! Falsifiers (labels).

use crate::*;

// ---- r5: GitHub routes by label SUBSET (rule (e)) ----

/// A benign ci.yml plus one extra workflow file `x.yml`.
pub(crate) fn with_x(name: &str, x: &str) -> PathBuf {
    let d = scratch(name, "on: push\njobs: {}\n");
    fs::write(d.join("x.yml"), x).unwrap();
    d
}

/// A PR-triggered job that holds a secret, on the given runs-on text.
pub(crate) fn secret_job(on: &str, runs_on: &str) -> String {
    format!(
        "on: {on}\njobs:\n  j:\n    runs-on: {runs_on}\n    env:\n      T: ${{{{ secrets.GH_TOKEN }}}}\n    steps: []\n"
    )
}

pub(crate) fn r5_red(name: &str, x: &str, needle: &str) {
    match judge(&with_x(name, x), &expected_canary()) {
        Ok(_) => panic!("r5 plant {name} was accepted:\n{x}"),
        Err(w) => assert!(
            w.iter().any(|m| m.contains(needle)),
            "r5 plant {name} refused for the wrong reason (want `{needle}`): {w:?}"
        ),
    }
}

#[test]
fn r5_the_five_lane_plants_fail() {
    for (n, ro) in [
        ("self-hosted", "[self-hosted]"),
        ("linux", "[self-hosted, Linux]"),
        ("unpooled", "[self-hosted, unpooled]"),
        ("ffmpeg", "[self-hosted, ffmpeg]"),
        ("group-only", "{group: g1}"),
    ] {
        r5_red(
            n,
            &secret_job("pull_request", ro),
            "can land on a spot seat",
        );
    }
}

#[test]
fn r5_subset_variants_fail() {
    for (n, ro) in [
        ("scalar", "self-hosted"),
        ("case", "[SELF-HOSTED, linux, UNPOOLED]"),
        (
            "x64-unpooled",
            "[self-hosted, Linux, X64, unpooled, ffmpeg]",
        ),
        ("order", "[ffmpeg, unpooled, self-hosted, linux]"),
        ("empty", "[]"),
        ("group-and-labels", "{group: g1, labels: [clean-room]}"),
        ("group-unlisted", "{group: some-new-group}"),
        ("group-expr", "{group: \"${{ vars.G }}\"}"),
        ("labels-only-spot", "{labels: [self-hosted, unpooled]}"),
        ("group-named-like-a-pool", "{group: clean-room}"),
        ("odd-key", "{labels: [clean-room], extra: x}"),
        ("hosted-lookalike", "[self-hosted, ubuntu]"),
        ("expr-inputs", "${{ inputs.r }}"),
        ("expr-fromjson", "${{ fromJSON(vars.R) }}"),
        ("label-expr", "[self-hosted, \"${{ vars.L }}\"]"),
    ] {
        r5_red(n, &secret_job("pull_request", ro), "spot");
    }
}

#[test]
fn r5_every_pr_like_trigger_counts() {
    for (n, on) in [
        ("list", "[push, pull_request]"),
        ("target", "pull_request_target"),
        ("review", "{pull_request_review: {}}"),
        ("merge-group", "merge_group"),
        ("call", "workflow_call"),
        (
            "run",
            "{workflow_run: {workflows: [a], types: [completed]}}",
        ),
    ] {
        r5_red(
            n,
            &secret_job(on, "[self-hosted, unpooled]"),
            "can land on a spot seat",
        );
    }
    // an unreadable `on` counts as reachable
    r5_red(
        "no-on",
        &secret_job("{}", "[self-hosted]").replace("on: {}\n", ""),
        "can land",
    );
}

#[test]
fn r5_matrix_options_are_judged_one_by_one() {
    let m = |opts: &str| {
        format!(
            "on: pull_request\njobs:\n  j:\n    runs-on: ${{{{ matrix.os }}}}\n    strategy:\n      matrix:\n{opts}\n    steps: []\n"
        )
    };
    r5_red(
        "m-list",
        &m("        os: [[self-hosted, clean-room], [self-hosted, unpooled]]"),
        "can land on a spot seat",
    );
    r5_red(
        "m-include",
        &m("        include:\n          - os: [self-hosted, Linux]"),
        "can land on a spot seat",
    );
    r5_red("m-none", &m("        other: [a]"), "no literal options");
    assert!(judge(
        &with_x("m-ok", &m("        include:\n          - os: [self-hosted, clean-room]\n          - os: [self-hosted, clean-room, ARM64]")),
        &expected_canary()
    )
    .is_ok());
}

#[test]
fn r5_pool_labels_and_hosted_runners_pass() {
    for (n, ro) in [
        ("clean-room", "[self-hosted, clean-room, X64]"),
        ("gate-light", "[self-hosted, gate-light]"),
        ("hosted", "ubuntu-latest"),
        ("hosted-list", "[macos-14]"),
        ("pool-mixed-case", "[self-hosted, Gate-Light]"),
        ("labels-map", "{labels: [self-hosted, clean-room]}"),
    ] {
        let d = with_x(n, &secret_job("pull_request", ro));
        assert!(judge(&d, &expected_canary()).is_ok(), "{n} refused");
    }
    // no trigger is out of scope: see the r6 tests
    let d = with_x("push-only", &secret_job("push", "[self-hosted, unpooled]"));
    assert!(judge(&d, &expected_canary()).is_err());
}

#[test]
fn r5_pool_table_is_self_consistent() {
    assert!(POOL_LABELS.iter().all(|l| *l == l.to_lowercase()));
    assert!(POOL_LABELS
        .iter()
        .all(|l| !l.contains("burst") && !l.contains("canary")));
    assert!(!POOL_LABELS.contains(&"self-hosted"));
}

// ---- r6: the subset rule applies to EVERY job of EVERY workflow, whatever its trigger ----

#[test]
fn r6_every_trigger_type_is_judged() {
    let ons = [
        "issue_comment",
        "[issue_comment]",
        "{issue_comment: {types: [created]}}",
        "push",
        "[push]",
        "{push: {branches: [main]}}",
        "{schedule: [{cron: '0 0 * * *'}]}",
        "workflow_dispatch",
        "repository_dispatch",
        "{repository_dispatch: {types: [x]}}",
        "release",
        "issues",
        "discussion",
        "{workflow_run: {workflows: [a], types: [completed]}}",
        "[pull_request, push]",
    ];
    let ros = [
        "[self-hosted]",
        "[self-hosted, Linux]",
        "[self-hosted, unpooled]",
        "{group: g1}",
    ];
    for (i, on) in ons.iter().enumerate() {
        for (k, ro) in ros.iter().enumerate() {
            r5_red(
                &format!("r6-on-{i}-{k}"),
                &secret_job(on, ro),
                "can land on a spot seat",
            );
        }
    }
    r5_red(
        "r6-no-on",
        &secret_job("{}", "[self-hosted]").replace("on: {}\n", ""),
        "can land on a spot seat",
    );
}

#[test]
fn r6_exclusive_labels_pass_on_every_trigger() {
    for (i, on) in [
        "issue_comment",
        "push",
        "schedule",
        "workflow_dispatch",
        "repository_dispatch",
    ]
    .iter()
    .enumerate()
    {
        let d = with_x(
            &format!("r6-ok-{i}"),
            &secret_job(on, "[self-hosted, clean-room, X64]"),
        );
        assert!(judge(&d, &expected_canary()).is_ok(), "{on} refused");
    }
}
