//! PMAT-234: gate R's verdict line, after a release that actually shipped.
//!
//! `scripts/dogfood/release-check.sh` kept ONE flat `pending` string and read
//! it as "the tag is not cut yet". Arm 6 adds a note after EVERY SUCCESSFUL
//! RELEASE — `Cargo.toml` is back at the newest tag's version, so no cut is in
//! flight and no crux document is owed — so the verdict said
//!
//!   GATE R PASS pre-tag: … PENDING until the tag is cut: no docs/audits/crux-…
//!
//! about a version that was tagged, released, on crates.io and rendered on
//! docs.rs. The words were not merely imprecise: they were the opposite of the
//! truth, printed at the one moment a reader most wants to believe the gate.
//!
//! The notes are now two sets. `note_pretag` is for the arms whose obligation
//! does not exist yet BECAUSE the tag does not: the tag, the GitHub release,
//! crates.io, docs.rs. `note_pending` is for everything else, and it never
//! licenses the words `pre-tag`.

#![cfg(unix)]

#[path = "release_check_fixture/mod.rs"]
mod fx;
use fx::*;

/// The state a finished release leaves on main, with every arm answering.
#[test]
fn after_a_published_release_the_verdict_never_says_pre_tag() {
    let fx = published_fixture();
    let dir = fx.root.parent().expect("tempdir").to_path_buf();
    let bin = stub_release_tools(&dir, PREV_VERSION, "true");
    let gh = stub_gh_published(&dir, &fx.head, "false");
    let r = run_published(&fx, &gh, &bin);

    r.assert_green("the tag is on main and on origin, the release is published, crates.io serves it and docs.rs built it");
    r.assert_never_says(
        "pre-tag",
        "the tag is on origin, so nothing here is before the tag",
    );
    r.assert_never_says(
        "PENDING until the tag is cut",
        "the tag IS cut, and the one remaining note has nothing to do with the tag",
    );
    // The remaining note is still reported — as what it actually is.
    r.assert_says("also pending");
    r.assert_says("no release is being cut");
}

/// And the pre-tag verdict still says pre-tag, because that state is real.
///
/// The fix must not buy its correctness by deleting the words: before the tag
/// exists, "PENDING until the tag is cut" is exactly right, and four notes
/// belong under it.
#[test]
fn before_the_tag_is_cut_the_verdict_still_says_so() {
    let fx = fixture(false, Some(good_receipt()));
    let gh = fx.gh_reporting_the_pr();
    let r = run(&fx, &gh);

    r.assert_green("a release being prepared, with its PR's receipt in place");
    r.assert_says("pre-tag");
    r.assert_says("PENDING until the tag is cut");
    r.assert_says("not on crates.io");
    r.assert_says("not on docs.rs");
    // Nothing else is pending in that fixture, so there is no second list.
    r.assert_never_says(
        "also pending",
        "the only notes in this state are the four the tag owes",
    );
}

/// A docs.rs that reports a failed build is still RED after the split.
///
/// The split touches the sentence, not the verdict, and the cheapest way for a
/// rewording to become a hole is for a FAIL to start reading as a note. This
/// drives the published state with `doc_status: false` and asserts the gate is
/// red and names it.
#[test]
fn a_published_release_whose_docs_never_built_is_still_red() {
    let fx = published_fixture();
    let dir = fx.root.parent().expect("tempdir").to_path_buf();
    let bin = stub_release_tools(&dir, PREV_VERSION, "false");
    let gh = stub_gh_published(&dir, &fx.head, "false");
    let r = run_published(&fx, &gh, &bin);

    r.assert_not_green("docs.rs reports the documentation never built");
    r.assert_says("doc_status=false");
}
