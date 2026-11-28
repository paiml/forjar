//! PMAT-178 / PMAT-180 / PMAT-163 (PMAT-178 decision): what
//! `scripts/dogfood/release-check.sh` counts as the set of PRs in a release,
//! what it counts as the tag, and what it accepts as a PR's quorum receipt.
//!
//! # The defects these pin
//!
//! **The PR window (PMAT-178).** Arm 5 enumerated the release's PRs with
//! `git log --merges`. This repository squash-merges, and a squash merge leaves
//! one ordinary commit with no second parent — so on a squash-merged release
//! the enumeration returned NOTHING, the receipt loop under it ran zero times,
//! and the arm reported PASS over an empty set. That is the exact vacuity the
//! whole dogfood contract exists to refuse: a loop over an empty directory is
//! indistinguishable, in CI output, from a check that ran.
//!
//! **The tag (PMAT-180).** Arms 1–4 decided PENDING from the LOCAL tag list.
//! `git tag` lists what this checkout happens to have fetched, so a version
//! that was tagged and pushed but never released read as "not cut yet" —
//! PENDING — which is the one state that must never cover a broken release.
//! The remote is the authority: PENDING is available only when origin has no
//! such tag.
//!
//! **The receipt itself (PMAT-178's re-decision, tracked as PMAT-163).**
//! `docs/audits/quorum-<pr>.md` was a placeholder name nothing ever wrote.
//! `scripts/quorum-gate.sh` already enforces, on every push, that a branch's
//! quorum lives in the COMMITTED `.quorum/${branch//\//-}.json`, and that file
//! survives a squash merge because it was committed on the branch. Arm 5 now
//! reads that exact path — first at the PR's merge commit, falling back to
//! HEAD — parses it as JSON, refuses a top-level `waived` key outright, and
//! requires `quorum.lanes` (>=3) and `quorum.judges` (>=3), the same floor
//! `scripts/quorum-gate.sh` enforces at push time.
//!
//! # Why these run the real script
//!
//! The gate's subject is a shell script, and a test that re-implements its
//! logic in Rust would pass over a script that no longer exists. So each case
//! builds a small repository shaped the way the script reads one — a
//! `Cargo.toml` version, two tags' worth of history, an `origin` — copies the
//! REAL `scripts/dogfood/release-check.sh` (and, since Arm 6 now calls it,
//! `scripts/dogfood/crux-reconcile.sh`) into it, and runs it. `gh` is injected
//! through `$GH`, which is how the script names the tool it requires:
//! `GH=false` is "gh cannot answer", and a two-line stub is "gh answers this".
//! Nothing here touches the network or the real repository.

#![cfg(unix)]

#[path = "release_check_fixture/mod.rs"]
mod fx;
use fx::*;
use std::process::Command;

#[test]
fn a_gh_that_cannot_answer_is_a_fail_and_never_a_pass() {
    let fx = fixture(false, Some(good_receipt()));
    // `false` is a tool that runs and refuses to answer — the shape of a
    // missing, unauthenticated or rate-limited gh.
    let r = run(&fx, "false");
    r.assert_not_green(
        "gh could not enumerate the release's PRs, so the window is UNMEASURED \
         and UNMEASURED is a failure",
    );
    r.assert_says("pr list");
}

#[test]
fn an_empty_pr_set_over_commits_that_landed_is_red() {
    let fx = fixture(false, Some(good_receipt()));
    // GitHub reports no merged PR in the window while a commit sits between
    // the previous tag and HEAD: either the enumeration is broken (the
    // `git log --merges` defect) or work reached the release without review.
    let gh = stub_gh(fx.root.parent().expect("tempdir"), "gh-empty", "[]");
    let r = run(&fx, &gh);
    r.assert_not_green(
        "an empty PR set over a non-empty commit range is the vacuity this arm \
         used to pass on",
    );
    r.assert_says(PREV_TAG);
}

#[test]
fn a_pr_with_its_receipt_passes_before_the_tag_is_cut() {
    let fx = fixture(false, Some(good_receipt()));
    let r = run(&fx, &fx.gh_reporting_the_pr());
    assert_eq!(
        r.code, 0,
        "a release whose one PR has its (committed, 3-lane, 3-judge) receipt \
         must pass:\n{}",
        r.text
    );
    // The anti-vacuity half of the case above: the arm CAN pass, so its
    // failures are about the PR set and not about the fixture. The count is
    // asserted because "0 PR(s) all have receipts" is the vacuous pass this
    // whole file exists to refuse — a green line here must say ONE.
    assert!(r.text.contains("GATE R PASS"), "no PASS line:\n{}", r.text);
    r.assert_says("1 PR(s)");
    r.assert_says(&format!("#{PR} {} receipt=ok", slug()));
    r.assert_says("PENDING");
}

#[test]
fn a_pr_without_its_receipt_is_named_and_red() {
    let fx = fixture(false, None);
    let r = run(&fx, &fx.gh_reporting_the_pr());
    r.assert_not_green(&format!(
        "PR #{PR} is in this release with no .quorum/{}.json receipt",
        slug()
    ));
    r.assert_says(&format!("#{PR} {} receipt=missing", slug()));
}

#[test]
fn a_waived_receipt_is_red() {
    let fx = fixture(false, Some(waived_receipt()));
    let r = run(&fx, &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "a top-level `waived` key is an unrefuted claim, not a passed quorum, \
         and this arm must never treat it as a pass",
    );
    r.assert_says(&format!("#{PR} {} receipt=waived", slug()));
}

#[test]
fn a_nested_override_is_red() {
    let fx = fixture(false, Some(nested_override_receipt()));
    let r = run(&fx, &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "an `override` key nested under `quorum` is a waiver a top-level check \
         cannot see; the arm applies gate E's predicate, which refuses it anywhere",
    );
    r.assert_says(&format!("#{PR} {} receipt=waived", slug()));
}

#[test]
fn a_receipt_that_refuted_nothing_is_red() {
    let fx = fixture(false, Some(unhunted_receipt()));
    let r = run(&fx, &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "claims_refuted = 0 is a round that did not hunt — the vacuous receipt \
         gate E refuses before the tag, and this arm must refuse after it",
    );
    r.assert_says(&format!("#{PR} {} receipt=thin", slug()));
}

#[test]
fn a_thin_receipt_is_red() {
    let fx = fixture(false, Some(thin_receipt()));
    let r = run(&fx, &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "2 lanes is below scripts/quorum-gate.sh's floor of 3, so the receipt \
         never cleared the bar a real quorum must clear",
    );
    r.assert_says(&format!("#{PR} {} receipt=thin", slug()));
}

#[test]
fn a_tag_that_exists_on_the_remote_is_never_pending() {
    let fx = fixture(true, Some(good_receipt()));
    let r = run(&fx, &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "origin serves the release tag, so the release has happened and no arm \
         about it may be reported as not-yet-happened",
    );
    r.assert_says(TAG);
    assert!(
        !r.text.contains("PENDING"),
        "the gate called a released version PENDING:\n{}",
        r.text
    );
}

#[test]
fn a_window_over_one_page_is_read_whole() {
    for n in [200, 450] {
        let fx = fixture(false, Some(good_receipt()));
        // Every one of the n members is a PR of its own, so each needs its own
        // head-bound store receipt (forjar#1327).
        for i in 1..=n {
            write(
                &fx.store,
                &format!("quorum/paiml/forjar/{i}/{}.json", fx.head),
                &store_receipt_for(
                    i as u32,
                    &fx.head,
                    r#"["a","b","c"]"#,
                    3,
                    "claude-sonnet-5-5",
                ),
            );
        }
        let r = run(&fx, &stub_gh_release_paged(&fx, n, "honest"));
        r.assert_green(&format!("a window of {n} PRs is read in full"));
        r.assert_says(&format!("{n} PR(s)"));
    }
}

#[test]
fn a_truncated_or_failing_page_is_unmeasured() {
    for (n, mode, says) in [
        (450, "always-full", "single second"),
        (450, "fail-second-call", "pr list exited"),
        (450, "child-drops-a-row", "two halves"),
        (6000, "honest", "more than 40"),
    ] {
        let fx = fixture(false, Some(good_receipt()));
        let r = run(&fx, &stub_gh_release_paged(&fx, n, mode));
        r.assert_not_green(mode);
        r.assert_says(says);
    }
}

#[test]
fn a_page_that_is_not_an_array_of_prs_is_unmeasured_at_once() {
    // exit 0 with these bodies: one read, then a verdict, never a bare abort or a storm
    let bodies = [
        "[1]",
        r#"["err"]"#,
        "",
        "[] []",
        r#"{"message":"x"}"#,
        r#"[{"mergedAt":"x"}]"#,
    ];
    for (i, body) in bodies.into_iter().enumerate() {
        let fx = fixture(false, Some(good_receipt()));
        let (gh, log) = stub_gh_counting(fx.root.parent().expect("dir"), &format!("bad{i}"), body);
        let r = run(&fx, &gh);
        r.assert_not_green(body);
        r.assert_says("one JSON array of PRs (jq exit 5)");
        assert_eq!(
            std::fs::read_to_string(&log).map_or(0, |s| s.lines().count()),
            1,
            "{body}"
        );
    }
}

// PMAT-1174: window membership. Only the first test is red without the fix;
// the others pin one mutant each of `dogfood_pr_member_of`.
fn row(n: u32, oid: &str, head_ref: &str) -> String {
    let m = r#""mergedAt":"2026-09-05T00:00:00Z""#;
    // The fixture lands each PR in one commit, so its head is its merge commit.
    format!(
        r#"{{"number":{n},{m},"mergeCommit":{{"oid":"{oid}"}},"headRefName":"{head_ref}","headRefOid":"{oid}"}}"#
    )
}

fn run_rows(fx: &Fixture, rows: &[String]) -> Run {
    let json = format!("[{}]", rows.join(","));
    run(
        fx,
        &stub_gh(fx.root.parent().expect("dir"), "gh-rows", &json),
    )
}

/// A commit one step ahead of HEAD: not an ancestor of it.
fn later(fx: &Fixture) -> String {
    let a = ["commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", "later"];
    stdout_of(&git(&fx.root, &a))
}

#[test]
fn the_previous_releases_own_pr_is_outside_the_window_by_membership() {
    let fx = fixture(false, Some(good_receipt()));
    let prev = stdout_of(&git(&fx.root, &["rev-parse", PREV_TAG]));
    let rows = [row(PR, &fx.head, HEAD_REF), row(76, &prev, "release/prev")];
    let r = run_rows(&fx, &rows);
    r.assert_green("the previous release's own PR is not a member");
    r.assert_says("1 PR(s)");
    r.assert_says(&format!("#{PR} {} receipt=ok", slug()));
    r.assert_never_says("release-prev", "a previous PR has no receipt row");
}

#[test]
fn both_bounds_are_ancestry_not_equality() {
    // root < lower tag < mid < HEAD. Equality on the lower bound makes root a
    // member; equality on the upper bound makes mid a non-member.
    let fx = fixture(false, Some(good_receipt()));
    let root = stdout_of(&git(&fx.root, &["rev-parse", PREV_TAG]));
    git(&fx.root, &["tag", "-f", PREV_TAG, &fx.head]);
    // mid is a PR head too, so it carries the author trailer the store gate
    // maps and its own head-bound store receipt (forjar#1327).
    git(
        &fx.root,
        &[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "mid\n\nCo-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>",
        ],
    );
    let mid = stdout_of(&git(&fx.root, &["rev-parse", "HEAD"]));
    write(
        &fx.store,
        &format!("quorum/paiml/forjar/{PR}/{mid}.json"),
        &store_receipt(&mid, r#"["a","b","c"]"#, 3, "claude-sonnet-5-5"),
    );
    git(&fx.root, &["commit", "-q", "--allow-empty", "-m", "tip"]);
    let rows = [row(PR, &mid, HEAD_REF), row(75, &root, "release/older")];
    let r = run_rows(&fx, &rows);
    r.assert_green("mid is a member, root is inside the previous tag");
    r.assert_says("1 PR(s)");
    r.assert_says(&format!(
        "#75 {root} is inside {PREV_TAG} (the previous release)"
    ));
    r.assert_never_says(
        "release-older",
        "a PR strictly before the tag has no receipt row",
    );
}

#[test]
fn a_merge_commit_this_checkout_lacks_is_unmeasured_and_named() {
    let fx = fixture(false, Some(good_receipt()));
    let rows = [
        row(PR, &fx.head, HEAD_REF),
        row(99, &"d".repeat(40), "feat/ghost"),
    ];
    let r = run_rows(&fx, &rows);
    r.assert_not_green("a commit the checkout lacks cannot be placed");
    r.assert_says("PR #99");
    r.assert_says("UNMEASURED");
}

#[test]
fn previous_and_after_carry_their_own_labels() {
    // Two after, one previous: swapping the labels moves both counts.
    let fx = fixture(false, Some(good_receipt()));
    let (prev, l) = (
        stdout_of(&git(&fx.root, &["rev-parse", PREV_TAG])),
        later(&fx),
    );
    let rows = [
        row(PR, &fx.head, HEAD_REF),
        row(76, &prev, "release/prev"),
        row(78, &l, "feat/later"),
        row(79, &l, "feat/later-2"),
    ];
    let r = run_rows(&fx, &rows);
    r.assert_green("one member, one previous, two after");
    r.assert_says("1 PR(s)");
    r.assert_says(&format!(
        "#76 {prev} is inside {PREV_TAG} (the previous release)"
    ));
    r.assert_never_says("#78 ", "an after PR is not labelled previous");
    r.assert_says("2 of them after this HEAD");
    r.assert_never_says("feat-later", "a PR merged after HEAD has no receipt row");
}

// forjar#1327: gate R ANDs the head-bound store receipt (forjar#876) with the
// committed `.quorum/<branch>.json` check, exactly as gate E does. The store
// receipt is keyed by the PR's HEAD (headRefOid), never by the merge commit.

#[test]
fn a_pr_with_no_store_receipt_at_its_head_is_red() {
    let fx = fixture(false, Some(good_receipt()));
    std::fs::remove_file(store_receipt_path(&fx)).expect("the fixture's store receipt");
    let r = run(&fx, &fx.gh_reporting_the_pr());
    r.assert_not_green(&format!(
        "PR #{PR} has its committed receipt but no store receipt at its head, \
         and the store receipt is the head-bound half of the quorum"
    ));
    r.assert_says(&format!("#{PR} {} receipt=missing", slug()));
}

#[test]
fn a_thin_store_receipt_at_its_head_is_red() {
    let fx = fixture(false, Some(good_receipt()));
    let thin = store_receipt(&fx.head, r#"["a","b"]"#, 3, "claude-sonnet-5-5");
    std::fs::write(store_receipt_path(&fx), thin).expect("write");
    let r = run(&fx, &fx.gh_reporting_the_pr());
    r.assert_not_green("a store receipt below the lane floor is not a quorum");
    r.assert_says(&format!("#{PR} {} receipt=thin", slug()));
}

#[test]
fn a_waived_store_receipt_at_its_head_is_red() {
    let fx = fixture(false, Some(good_receipt()));
    let waived = store_receipt(&fx.head, r#"["a","b","c"]"#, 3, "claude-sonnet-5-5").replacen(
        '{',
        r#"{"waived":{"reason":"no credits"},"#,
        1,
    );
    std::fs::write(store_receipt_path(&fx), waived).expect("write");
    let r = run(&fx, &fx.gh_reporting_the_pr());
    // The store gate refuses a waiver key itself, before the floors run, so the
    // refusal reads store=missing with the waiver named as the reason.
    r.assert_not_green("a waiver in the store receipt is never a pass");
    r.assert_says(&format!("#{PR} {} receipt=missing", slug()));
    r.assert_says("waiver/override text");
}

#[test]
fn an_unset_store_is_unmeasured_and_never_a_pass() {
    let fx = fixture(false, Some(good_receipt()));
    let gh = fx.gh_reporting_the_pr();
    let out = Command::new("bash")
        .arg(fx.root.join("scripts/dogfood/release-check.sh"))
        .current_dir(&fx.root)
        .env_remove("QUORUM_STORE")
        .env("GH", &gh)
        .output()
        .expect("bash must run");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let r = Run {
        code: out.status.code().unwrap_or(-1),
        text,
    };
    r.assert_not_green("QUORUM_STORE unset leaves the store receipts unreadable");
    r.assert_says("QUORUM_STORE");
}
