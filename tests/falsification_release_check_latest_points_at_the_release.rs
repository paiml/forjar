//! PMAT-534: GitHub's "latest release" pointed at v1.25.2 for four releases.
//!
//! # What was measured
//!
//! On 2026-09-12, before it was corrected by hand,
//! `repos/paiml/forjar/releases/latest` resolved to **v1.25.2** while v1.26.0,
//! v1.27.0 and v1.28.0 were all `prerelease=false draft=false` and days newer.
//! Every `curl -L …/releases/latest/download/…`, every badge and every script
//! that follows that URL got a four-version-old binary, for days, while gate R
//! said the release was fine.
//!
//! # Why it happened
//!
//! A release is BORN a prerelease here (`release.yml`, PMAT-166) so no consumer
//! sees a half-uploaded asset set, and **GitHub never makes a prerelease
//! latest**. Clearing the flag afterwards does not recompute the pointer:
//! `make_latest` is fixed when the flag is written, so
//! `gh release edit <tag> --prerelease=false` leaves latest exactly where it
//! was. The promotion has to say `--latest`, and the step that was supposed to
//! perform it was written down nowhere and checked by nothing.
//!
//! # What the arm does and does not do
//!
//! A PRERELEASE IS A DELIBERATE STATE. forjar publishes a rolling `nightly`
//! prerelease, and a release still waiting on `make dogfood-published` is
//! legitimately one — so that case is REPORTED, with the command that ends it,
//! and is not refused. What is refused is a FULL release that `/releases/latest`
//! does not point at, which is the shape every stale pointer above had.

#![cfg(unix)]

#[path = "release_check_fixture/mod.rs"]
mod fx;
use fx::*;

/// The exact shape v1.26.0, v1.27.0 and v1.28.0 were in: a full release that
/// `/releases/latest` does not resolve to.
#[test]
fn a_full_release_that_is_not_latest_is_named_and_red() {
    let f = published_fixture();
    let dir = f.root.parent().expect("tempdir").to_path_buf();
    let bin = stub_release_tools(&dir, PREV_VERSION, "true");
    let gh = stub_gh_published_as(&dir, &f.head, "false", "false", PREV_PREV_TAG);
    let r = run_published(&f, &gh, &bin);

    r.assert_not_green("a full release that /releases/latest does not point at");
    // It must name BOTH: the release being checked and what the URL answers,
    // or the reader cannot tell which of the two is wrong.
    r.assert_says(PREV_TAG);
    r.assert_says(PREV_PREV_TAG);
    // And it must say what to run. The defect lasted four releases because the
    // promotion step existed only in one line of a workflow's output.
    r.assert_says("--latest");
}

/// The pointer agreeing is green, and green for this reason rather than by
/// dying earlier.
#[test]
fn a_full_release_that_is_latest_passes() {
    let f = published_fixture();
    let dir = f.root.parent().expect("tempdir").to_path_buf();
    let bin = stub_release_tools(&dir, PREV_VERSION, "true");
    let gh = stub_gh_published_as(&dir, &f.head, "false", "false", PREV_TAG);
    let r = run_published(&f, &gh, &bin);

    r.assert_green("a full release that /releases/latest resolves to");
    r.assert_never_says(
        "still resolves to",
        "the pointer agrees, so the arm has nothing to report",
    );
}

/// A prerelease is not forced to latest — it is REPORTED, with the command.
///
/// GitHub never makes a prerelease latest, so demanding it here would refuse
/// the rolling `nightly` release and every release between its tag and
/// `make dogfood-published`. What the gate owes the reader is the fact and the
/// remedy, not a refusal.
#[test]
fn a_prerelease_is_reported_and_not_forced_to_latest() {
    let f = published_fixture();
    let dir = f.root.parent().expect("tempdir").to_path_buf();
    let bin = stub_release_tools(&dir, PREV_VERSION, "true");
    let gh = stub_gh_published_as(&dir, &f.head, "false", "true", PREV_PREV_TAG);
    let r = run_published(&f, &gh, &bin);

    r.assert_green("a prerelease, which GitHub will never make latest");
    r.assert_says("PRERELEASE");
    r.assert_says("also pending");
    // The remedy, verbatim enough to paste.
    r.assert_says("--prerelease=false --latest");
}

/// A `gh api` that cannot answer is UNMEASURED, and unmeasured is a failure.
///
/// The one state this arm must never reach is "the pointer could not be read,
/// so assume it points here" — that is precisely the silence the four stale
/// releases lived in.
#[test]
fn a_pointer_that_cannot_be_read_is_unmeasured_and_red() {
    let f = published_fixture();
    let dir = f.root.parent().expect("tempdir").to_path_buf();
    let bin = stub_release_tools(&dir, PREV_VERSION, "true");
    let gh = stub_gh_published_api_broken(&dir, &f.head);
    let r = run_published(&f, &gh, &bin);

    r.assert_not_green("a gh api that cannot answer where /releases/latest points");
    r.assert_says("UNMEASURED");
    r.assert_says("releases/latest");
}

/// `gh` exits 0 and says nothing, which is neither a tag nor an error.
///
/// MEASURED on this branch before the case existed: deleting the guard left
/// all four cases green, so nothing defended it. Without the guard the empty
/// answer falls through to the comparison below and the gate refuses the
/// release with `still resolves to ` and a blank where the tag goes — the
/// reader is told the pointer is wrong when what happened is that it never
/// answered.
#[test]
fn a_pointer_that_answers_with_nothing_is_unmeasured_and_red() {
    let f = published_fixture();
    let dir = f.root.parent().expect("tempdir").to_path_buf();
    let bin = stub_release_tools(&dir, PREV_VERSION, "true");
    let gh = stub_gh_published_api_empty(&dir, &f.head);
    let r = run_published(&f, &gh, &bin);

    r.assert_not_green("a gh api that exits 0 and prints no tag name");
    r.assert_says("UNMEASURED");
    r.assert_says("no tag_name");
    // The message must not accuse the pointer of resolving somewhere: it did
    // not resolve at all, and the two are different findings.
    r.assert_never_says(
        "still resolves to",
        "an empty answer is not a pointer that resolves elsewhere",
    );
}

/// A prerelease whose repository has no full release behind it.
///
/// `/releases/latest` serves only non-prerelease, non-draft releases and 404s
/// when there is none — MEASURED 2026-09-12 on electron/electron, whose newest
/// release `v45.0.0-alpha.6` is a prerelease and whose `/releases/latest`
/// answers `v44.3.0`, and on a repository with no releases, where `gh` exits 1
/// with HTTP 404. For a repository whose releases are all prereleases that 404
/// is the ORDINARY state. Refusing it would force to latest exactly the
/// deliberate prerelease this arm promises not to force — the paragraph at the
/// top of this file would be false.
#[test]
fn a_prerelease_with_no_full_release_behind_it_is_reported_not_refused() {
    let f = published_fixture();
    let dir = f.root.parent().expect("tempdir").to_path_buf();
    let bin = stub_release_tools(&dir, PREV_VERSION, "true");
    let gh = stub_gh_published_api_broken_as(&dir, &f.head, "true");
    let r = run_published(&f, &gh, &bin);

    r.assert_green("a prerelease whose repository has no release latest can serve");
    r.assert_says("PRERELEASE");
    r.assert_says("also pending");
    r.assert_says("--prerelease=false --latest");
    // And it must not claim to have measured what it could not read.
    r.assert_never_says(
        "UNMEASURED",
        "a 404 is the ordinary state for a prerelease, not a broken instrument",
    );
}
