//! PMAT-540: the window rule reads BRANCH, then TITLE, then BODY — and until
//! now nothing checked any of them against what the work itself claims.
//!
//! # What went wrong
//!
//! PR #532 was pushed from `PMAT-520-book-v1.29.0` while every commit's
//! `Pmat-Ticket:` trailer said PMAT-531. Every window arm credited PMAT-520 —
//! which had already SHIPPED in 1.29.0 — to the v1.30.0 window, gate T demanded
//! `release:v1.30.0` on it, and PMAT-531 was invisible to all three gates.
//!
//! PMAT-535 closed the BRANCH half: `scripts/quorum-gate.sh` refuses, at push
//! time, a branch naming a `PMAT-<n>` no commit being pushed claims. It cannot
//! close the other two — at push time the pull request does not exist and its
//! title is typed afterwards — so `fix/ci-lint` with a PR titled `(PMAT-520)`
//! reproduced the whole defect untouched.
//!
//! Gate A is where the PR does exist: it holds the PR object, the merge commit
//! and the receipt, so it can compare the id it files the PR under against what
//! the merge commit claims.
//!
//! # Why the line and not git's trailer parser
//!
//! `%(trailers:key=…)` reads trailers from the LAST PARAGRAPH only, and a
//! squash message ends with whatever bullets GitHub assembled. On `b4719737`,
//! the merge commit of #532 itself, git's parser returns NOTHING while the line
//! is plainly there — an arm built on it would be blind to the exact commit it
//! exists for. pmat's CB-2113 asks git the same way, which is why it saw
//! nothing either.
//!
//! # The floor
//!
//! A rule cannot condemn a record it arrived after. `TRAILER_FLOOR` is PR
//! #532's own merge commit, the ONE mismatch in the thirty most recently merged
//! PRs; every PR merged after it is judged. A floor this repository does not
//! carry exempts NOTHING, because an exemption that cannot be found must make
//! the gate stricter and not looser.

#![cfg(unix)]

#[path = "dogfood_gates_harness/mod.rs"]
mod harness;

use harness::*;

/// A squash message in the shape GitHub actually writes one: a subject, the
/// branch commits' bodies, and the trailer block repeated once per commit.
fn squash_claiming(ticket: &str) -> String {
    format!(
        "work that landed (#77)\n\n* the first commit\n\nPmat-Ticket: {ticket}\n\
         Co-Authored-By: t <t@t>\n\n* the second commit\n\nPmat-Ticket: {ticket}\n\
         Co-Authored-By: t <t@t>\n"
    )
}

/// The exact shape PR #532 was in: filed under one ticket, claimed by another.
#[test]
fn a_pr_whose_commits_claim_another_ticket_is_named_and_red() {
    let fx = fixture_msg(
        Some(&good_impl_receipt()),
        Some(good_quorum_receipt()),
        &squash_claiming("PMAT-111"),
    );
    let gh = fx.gh_reporting_the_pr();
    let out = run(&fx, "harness.sh", &gh);
    out.assert_not_green(
        "A",
        "a PR filed under one ticket whose commits claim another",
    );
    out.assert_says(TICKET);
    out.assert_says("PMAT-111");
    // It must say what the id DECIDES, or the next person edits the trailer to
    // match the branch and moves the defect instead of fixing it.
    out.assert_says("RELEASE WINDOW");
}

/// A PR whose commits claim the ticket it is filed under passes.
///
/// Asserted as a PASS by name, not merely as "exit 0": an arm that refused
/// everything would still satisfy the case above.
#[test]
fn a_pr_whose_commits_claim_its_own_ticket_passes() {
    let fx = fixture_msg(
        Some(&good_impl_receipt()),
        Some(good_quorum_receipt()),
        &squash_claiming(TICKET),
    );
    let gh = fx.gh_reporting_the_pr();
    run(&fx, "harness.sh", &gh)
        .assert_green("A", "a PR whose commits claim the ticket it is filed under");
}

/// A merge commit that claims NOTHING is a different check's finding.
///
/// The commit-msg hook refuses a message with no `Pmat-Ticket:`, and pmat's
/// CB-2113 refuses it again. An arm that also reported it would be saying
/// someone else's finding in its own words — and the defect this arm is about
/// had a trailer; it was the wrong one.
#[test]
fn a_merge_commit_that_claims_nothing_is_not_this_arms_finding() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let gh = fx.gh_reporting_the_pr();
    let out = run(&fx, "harness.sh", &gh);
    out.assert_green("A", "a merge commit with no Pmat-Ticket line at all");
}

/// A claim that resolves THROUGH an alias agrees with the row that declares it.
///
/// PR #496's branch says PMAT-218, a ticket that never existed, and its commits
/// say PMAT-219 — whose roadmap row declares `alias:PMAT-218`. That PR is
/// correctly filed, and an arm comparing raw ids would have called it a
/// mismatch. Resolving before comparing is why it is green.
#[test]
fn a_claim_that_resolves_through_an_alias_agrees() {
    let fx = fixture_msg(
        Some(&good_impl_receipt()),
        Some(good_quorum_receipt()),
        &squash_claiming("PMAT-222"),
    );
    // The row that owns the work declares the misnomer the branch used.
    fx.declare_alias("PMAT-222", TICKET);
    let gh = fx.gh_reporting_the_pr();
    run(&fx, "harness.sh", &gh).assert_green(
        "A",
        "a PR whose commits claim an id the filed row declares as its alias",
    );
}

/// A PR at or below the floor is exempt, and the gate SAYS SO.
///
/// An exemption that printed nothing would be indistinguishable from a check
/// that ran, which is the thing this repository refuses everywhere else.
#[test]
fn a_pr_below_the_trailer_floor_is_not_judged_and_says_so() {
    let fx = fixture_msg(
        Some(&good_impl_receipt()),
        Some(good_quorum_receipt()),
        &squash_claiming("PMAT-111"),
    );
    let gh = fx.gh_reporting_the_pr();
    let head = fx.head.clone();
    let out = run_env(&fx, "harness.sh", &gh, &[("TRAILER_FLOOR", &head)]);
    out.assert_green("A", "a mismatching PR at the floor");
    out.assert_says("predates the trailer floor");
}

/// A floor this repository does not carry exempts NOTHING.
///
/// A typo, a shallow clone or a fixture repository must all make the gate
/// stricter rather than looser: an exemption that cannot be found is not an
/// exemption, and the direction that hides a defect is the one that must never
/// be taken by accident.
#[test]
fn a_floor_this_repository_does_not_carry_exempts_nothing() {
    let fx = fixture_msg(
        Some(&good_impl_receipt()),
        Some(good_quorum_receipt()),
        &squash_claiming("PMAT-111"),
    );
    let gh = fx.gh_reporting_the_pr();
    let out = run_env(
        &fx,
        "harness.sh",
        &gh,
        &[("TRAILER_FLOOR", "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef")],
    );
    out.assert_not_green("A", "a floor that names no commit in this repository");
    out.assert_says("PMAT-111");
}

/// The floor the script ships with is a real commit, and it is #532's.
///
/// A floor naming a commit the repository does not carry would silently stop
/// exempting anything — safe, but not what the comment beside it claims. This
/// case reads the constant out of the script and resolves it against the real
/// repository.
#[test]
fn the_shipped_floor_is_the_commit_it_says_it_is() {
    let script = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/dogfood/harness.sh"),
    )
    .expect("gate A must exist");
    let line = script
        .lines()
        .find(|l| l.starts_with("TRAILER_FLOOR="))
        .expect("PMAT-540: gate A no longer declares a TRAILER_FLOOR");
    let floor = line
        .split(":-")
        .nth(1)
        .and_then(|t| t.split('}').next())
        .expect("PMAT-540: TRAILER_FLOOR is not a `${VAR:-default}` default");
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--verify", &format!("{floor}^{{commit}}")])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git must run");
    assert!(
        out.status.success(),
        "PMAT-540: the shipped TRAILER_FLOOR {floor:?} is not a commit in this \
         repository, so it exempts nothing and the comment beside it describes \
         a record it cannot reach"
    );
}
