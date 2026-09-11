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
