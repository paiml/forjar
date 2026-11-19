//! PMAT-1164: gate A pages a window past 200 PRs; a read that cannot finish is UNMEASURED.
#![cfg(unix)]
#[path = "dogfood_gates_harness/mod.rs"]
mod harness;
use harness::*;
fn go(n: usize, mid: usize, mode: &str) -> Run {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let ct = |r: &str| -> i64 {
        stdout_of(&git(&fx.root, &["log", "-1", "--format=%ct", r]))
            .parse()
            .unwrap()
    };
    let (a, b) = (ct(&fx.tagged), ct(&fx.head) + 86_400);
    let t = |i: usize| match i < n {
        true => a + 1 + (i as i64) * (b - a - 2) / (n as i64 - 1),
        false => (a + b) / 2,
    };
    let row = |i: usize| {
        let (num, t, h) = (i + 1, t(i), &fx.head);
        format!(
            r#"{{"number":{num},"t":{t},"mergedAt":"x","mergeCommit":{{"oid":"{h}"}},"headRefName":"{HEAD_REF}","title":"t","body":"b"}}"#
        )
    };
    let rows = format!("[{}]", (0..n + mid).map(row).collect::<Vec<_>>().join(","));
    let gh = stub_gh_paged(fx.root.parent().expect("dir"), &rows, mode);
    run(&fx, "harness.sh", &gh)
}
#[test]
fn a_450_pr_window_is_fully_measured_and_loses_none_at_the_seams() {
    go(450, 0, "honest").assert_green("A", "450 PRs are read to the end");
    go(450, 3, "honest").assert_says("453 of 453");
}
#[test]
fn a_read_that_cannot_finish_is_unmeasured() {
    for (n, mode, says) in [
        (450, "always-full", "single second"),
        (450, "fail-second-call", "UNMEASURED"),
        (450, "child-drops-a-row", "UNMEASURED"),
        (6000, "honest", "more than 40"),
    ] {
        let r = go(n, 0, mode);
        r.assert_not_green("A", mode);
        r.assert_says(says);
    }
}
#[test]
fn a_page_that_is_not_an_array_of_prs_is_unmeasured() {
    for (i, body) in ["[1]", "", "[] []", r#"{"m":1}"#].into_iter().enumerate() {
        let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
        let r = run(
            &fx,
            "harness.sh",
            &stub_gh(fx.root.parent().unwrap(), &format!("bad{i}"), body),
        );
        r.assert_not_green("A", body);
        r.assert_says("one JSON array of PRs");
    }
}
