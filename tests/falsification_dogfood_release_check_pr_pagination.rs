//! PMAT-1164: Arm 5 of `scripts/dogfood/release-check.sh` reads the release's
//! PRs past one 200-PR page (lib/prpage.sh) instead of refusing, and a read
//! that cannot finish stays UNMEASURED there. Real script, stubbed `gh` that
//! honours `--search merged:A..B` and caps a page at 200, as GitHub does.

#![cfg(unix)]

#[path = "release_check_fixture/mod.rs"]
mod fx;
#[path = "dogfood_gates_harness/mod.rs"]
mod harness;
use fx::*;

/// `n` PRs spread over the window (previous tag's commit time to HEAD's plus a
/// day), each merged into this release's own commit.
fn rows(fx: &Fixture, n: usize) -> String {
    let ct = |rev: &str| -> i64 {
        stdout_of(&git(&fx.root, &["log", "-1", "--format=%ct", rev]))
            .parse()
            .expect("epoch")
    };
    let (a, b) = (ct(PREV_TAG), ct("HEAD") + 86_400);
    let v: Vec<String> = (0..n)
        .map(|i| {
            let t = a + 1 + (i as i64) * (b - a - 2) / (n as i64 - 1);
            format!(
                r#"{{"number":{},"t":{t},"mergedAt":"x","mergeCommit":{{"oid":"{}"}},"headRefName":"{HEAD_REF}"}}"#,
                i + 1,
                fx.head
            )
        })
        .collect();
    format!("[{}]", v.join(","))
}

fn paged(fx: &Fixture, mode: &str) -> String {
    let dir = fx.root.parent().expect("tempdir");
    harness::stub_gh_paged(dir, mode, &rows(fx, 450), mode)
}

#[test]
fn arm_5_a_window_of_450_prs_is_fully_measured() {
    let fx = fixture(false, Some(good_receipt()));
    let r = run(&fx, &paged(&fx, "honest"));
    assert_eq!(
        r.code, 0,
        "450 PRs are read to the end and judged:\n{}",
        r.text
    );
    assert!(r.text.contains("GATE R PASS"), "no PASS line:\n{}", r.text);
    r.assert_says("450 PR(s)");
}

#[test]
fn arm_5_a_failed_slice_stays_unmeasured() {
    let fx = fixture(false, Some(good_receipt()));
    let r = run(&fx, &paged(&fx, "fail-second-call"));
    r.assert_not_green("a slice that cannot be read is no partial result");
    r.assert_says("UNMEASURED");
}

#[test]
fn arm_5_a_truncating_gh_stays_unmeasured() {
    let fx = fixture(false, Some(good_receipt()));
    let r = run(&fx, &paged(&fx, "always-full"));
    r.assert_not_green("a gh that fills every page cannot be read to the end");
    r.assert_says("UNMEASURED");
}
