//! PMAT-1164: the release window's PR read pages past one 200-PR page instead of
//! refusing, and a read that cannot finish stays UNMEASURED (never a pass).
//! Runs the real gate A script against a stubbed `gh` that honours
//! `--search merged:A..B` and caps a page at 200, as GitHub does.

#![cfg(unix)]

#[path = "dogfood_gates_harness/mod.rs"]
mod harness;

use harness::*;
use std::process::Command;

fn ct(fx: &Fixture, rev: &str) -> i64 {
    stdout_of(&git(&fx.root, &["log", "-1", "--format=%ct", rev]))
        .parse()
        .expect("epoch")
}

/// The window the gate reads: from the tag commit to HEAD's time plus a day.
fn bounds(fx: &Fixture) -> (i64, i64) {
    (ct(fx, &fx.tagged), ct(fx, &fx.head) + 86_400)
}

fn row(n: usize, t: i64, oid: &str) -> String {
    format!(
        r#"{{"number":{n},"t":{t},"mergedAt":"x","mergeCommit":{{"oid":"{oid}"}},"headRefName":"{HEAD_REF}","title":"t","body":"b"}}"#
    )
}

/// `n` PRs spread over the window, plus `mid` more on the midpoint second.
fn rows(fx: &Fixture, n: usize, mid: usize) -> String {
    let (a, b) = bounds(fx);
    let mut v: Vec<String> = (0..n)
        .map(|i| {
            row(
                i + 1,
                a + 1 + (i as i64) * (b - a - 2) / (n as i64 - 1),
                &fx.head,
            )
        })
        .collect();
    v.extend((0..mid).map(|i| row(1000 + i, (a + b) / 2, &fx.head)));
    format!("[{}]", v.join(","))
}

fn paged(fx: &Fixture, rows: &str, mode: &str) -> String {
    stub_gh_paged(fx.root.parent().expect("tempdir"), mode, rows, mode)
}

fn fx() -> Fixture {
    fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()))
}

#[test]
fn gate_a_a_window_of_450_prs_is_fully_measured() {
    let fx = fx();
    let gh = paged(&fx, &rows(&fx, 450, 0), "honest");
    let r = run(&fx, "harness.sh", &gh);
    r.assert_green(
        "A",
        "450 PRs over a 200-PR page are read to the end and judged",
    );
    r.assert_says("450 of 450");
}

#[test]
fn gate_a_a_window_of_450_prs_loses_none_at_the_seams() {
    let fx = fx();
    let gh = paged(&fx, &rows(&fx, 450, 3), "honest");
    let r = run(&fx, "harness.sh", &gh);
    r.assert_green(
        "A",
        "PRs on the midpoint second sit in both halves, none lost",
    );
    r.assert_says("453 of 453");
}

#[test]
fn gate_a_a_truncating_gh_is_unmeasured() {
    let fx = fx();
    let gh = paged(&fx, &rows(&fx, 450, 0), "always-full");
    let r = run(&fx, "harness.sh", &gh);
    r.assert_not_green("A", "a gh that fills every page cannot be read to the end");
    r.assert_says("UNMEASURED");
}

#[test]
fn gate_a_a_failed_slice_is_unmeasured() {
    let fx = fx();
    let gh = paged(&fx, &rows(&fx, 450, 0), "fail-second-call");
    let r = run(&fx, "harness.sh", &gh);
    r.assert_not_green("A", "a slice that cannot be read is no partial result");
    r.assert_says("UNMEASURED");
}

#[test]
fn gate_a_inconsistent_bisect_is_unmeasured() {
    let fx = fx();
    let gh = paged(&fx, &rows(&fx, 450, 0), "child-drops-a-row");
    let r = run(&fx, "harness.sh", &gh);
    r.assert_not_green(
        "A",
        "a short half that lost a row the full page listed is partial",
    );
    r.assert_says("UNMEASURED");
}

#[test]
fn gate_a_release_pr_merged_1s_after_tag_and_late_dated_ancestor_both_counted() {
    let fx = fx();
    // An ancestor of HEAD dated two hours AFTER HEAD, then HEAD itself.
    let t0 = ct(&fx, &fx.head);
    let late = format!("@{} +0000", t0 + 7_200);
    let commit = |msg: &str, date: &str| {
        let ok = Command::new("git")
            .args(["commit", "-q", "--allow-empty", "-m", msg])
            .current_dir(&fx.root)
            .env("GIT_COMMITTER_DATE", date)
            .env("GIT_AUTHOR_DATE", date)
            .status()
            .expect("git commit")
            .success();
        assert!(ok, "commit {msg}");
        stdout_of(&git(&fx.root, &["rev-parse", "HEAD"]))
    };
    let x = commit("late-dated ancestor (#78)", &late);
    let h = commit("HEAD (#79)", &format!("@{} +0000", t0));
    let rows = format!("[{},{}]", row(78, t0 + 7_201, &x), row(79, t0 + 1, &h));
    let gh = paged(&fx, &rows, "honest");
    let r = run(&fx, "harness.sh", &gh);
    r.assert_green(
        "A",
        "both PRs are in the window though merged after HEAD's own time",
    );
    r.assert_says("2 of 2");
}
