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

/// A squash message in the shape GitHub actually writes one.
///
/// The shape matters and a review lane measured that the first version of this
/// helper did not have it. GitHub appends its OWN last paragraph to a squash
/// message: after the branch commits' bodies it writes a `---------` separator
/// and a `Co-authored-by:` block, and that block becomes the last paragraph —
/// which is the only place `%(trailers:key=…)` looks. Without the separator
/// git parses the `Pmat-Ticket:` lines happily, and every case here would pass
/// over an arm that had been "simplified" to use git's parser, which is the one
/// regression this suite exists to prevent.
///
/// Measured on `b4719737`, PR #532's own merge commit: `%(trailers)` returns
/// the `Co-authored-by:` line alone.
fn squash_claiming(ticket: &str) -> String {
    format!(
        "work that landed (#77)\n\n* the first commit\n\nPmat-Ticket: {ticket}\n\
         Co-Authored-By: t <t@t>\n\n* the second commit\n\nPmat-Ticket: {ticket}\n\
         Co-Authored-By: t <t@t>\n\n---------\n\nCo-authored-by: t <t@t>\n"
    )
}

/// What git's OWN parser sees on the fixture's HEAD.
fn git_sees_trailer(repo: &std::path::Path) -> String {
    String::from_utf8_lossy(
        &git(
            repo,
            &[
                "log",
                "-1",
                "--format=%(trailers:key=Pmat-Ticket,valueonly=true)",
            ],
        )
        .stdout,
    )
    .trim()
    .to_string()
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

/// The floor the script ships with names a real commit — measured where the
/// history is there to measure it against.
///
/// A floor naming a commit the repository does not carry would silently stop
/// exempting anything. That is SAFE — `a_floor_this_repository_does_not_carry_exempts_nothing`
/// pins the direction — but it is not what the comment beside the constant
/// claims, so the constant is checked too.
///
/// A CI checkout is shallow by default, and `b4719737` is not in a
/// `--depth 1` clone. An assertion that cannot be taken there must not fail and
/// must not quietly pass: this case MEASURES which of the two repositories it is
/// in, says so in its own failure text, and asserts something real in each.
#[test]
fn the_shipped_floor_is_the_commit_it_says_it_is() {
    let floor = shipped_floor();
    // Measurable anywhere: the constant is an abbreviated object name.
    assert!(
        floor.len() >= 7 && floor.chars().all(|c| c.is_ascii_hexdigit()),
        "PMAT-540: the shipped TRAILER_FLOOR {floor:?} is not a hex object name, \
         so it can never resolve and the floor exempts nothing anywhere"
    );

    let resolves = git_here(&["rev-parse", "--verify", &format!("{floor}^{{commit}}")]);
    if resolves {
        return;
    }
    // It did not resolve. Exactly one explanation is acceptable: this checkout
    // does not carry the history. Anything else is a typo in the constant.
    let shallow = std::process::Command::new("git")
        .args(["rev-parse", "--is-shallow-repository"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git must run");
    let shallow = String::from_utf8_lossy(&shallow.stdout).trim() == "true";
    assert!(
        shallow,
        "PMAT-540: the shipped TRAILER_FLOOR {floor:?} does not resolve in a \
         repository that is NOT shallow, so it names no commit here and the \
         comment beside it describes a record it cannot reach"
    );
    // And in the shallow case, say what was measured rather than nothing: the
    // floor is unreachable here, which is precisely the state gate A treats as
    // "exempt nothing".
    assert!(
        !git_here(&["cat-file", "-e", &floor]),
        "PMAT-540: git calls this repository shallow and yet carries {floor:?}, \
         so the branch this case took does not describe the tree it is in"
    );
}

/// The `TRAILER_FLOOR` default, read out of gate A itself.
fn shipped_floor() -> String {
    let script = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/dogfood/harness.sh"),
    )
    .expect("gate A must exist");
    let line = script
        .lines()
        .find(|l| l.trim_start().starts_with("TRAILER_FLOOR="))
        .expect("PMAT-540: gate A no longer declares a TRAILER_FLOOR");
    line.split(":-")
        .nth(1)
        .and_then(|t| t.split('}').next())
        .unwrap_or_else(|| {
            panic!("PMAT-540: TRAILER_FLOOR is not a `${{VAR:-default}}` default: {line}")
        })
        .to_string()
}

/// A git command against the real repository: did it succeed?
fn git_here(args: &[&str]) -> bool {
    std::process::Command::new("git")
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git must run")
        .status
        .success()
}

/// THE CASE A REVIEW LANE NAMED AS MISSING: the arm must read what git cannot.
///
/// Every other case here would pass over an arm rewritten to use
/// `%(trailers:key=…)`, because `git interpret-trailers` and `git log
/// --format=%(trailers)` both parse a message whose last paragraph IS the
/// trailer block. The real defect only exists because GitHub writes a
/// `---------` + `Co-authored-by:` paragraph after it.
///
/// So this case asserts, in order: that git's own parser returns NOTHING for
/// the fixture's message, and that the gate nevertheless PASSES a PR whose
/// commits claim its own ticket. An arm using git's parser would read "claims
/// nothing", take the skip, and also pass — so the first assertion is what
/// makes the second mean something, and it fails loudly if git ever starts
/// reading this shape.
#[test]
fn the_arm_reads_a_claim_gits_own_parser_cannot_see() {
    let fx = fixture_msg(
        Some(&good_impl_receipt()),
        Some(good_quorum_receipt()),
        &squash_claiming(TICKET),
    );
    let by_git = git_sees_trailer(&fx.root);
    assert!(
        by_git.is_empty(),
        "PMAT-540: git now parses this message's Pmat-Ticket lines ({by_git:?}), \
         so the fixture no longer reproduces the shape that made #532 invisible \
         and no case here would catch an arm rewritten to use git's parser"
    );
    run(&fx, "harness.sh", &fx.gh_reporting_the_pr()).assert_green(
        "A",
        "a PR whose commits claim its own ticket in a message git cannot parse",
    );
}

/// The same shape, mismatching: the arm must REFUSE what git cannot see.
///
/// This is the half with teeth. An arm using git's parser reads "claims
/// nothing" and passes; this one reads the line and refuses.
#[test]
fn a_mismatch_gits_own_parser_cannot_see_is_still_refused() {
    let fx = fixture_msg(
        Some(&good_impl_receipt()),
        Some(good_quorum_receipt()),
        &squash_claiming("PMAT-111"),
    );
    let by_git = git_sees_trailer(&fx.root);
    assert!(
        by_git.is_empty(),
        "PMAT-540: git now parses this shape ({by_git:?}), so this case no \
         longer proves the arm must read the line itself"
    );
    let out = run(&fx, "harness.sh", &fx.gh_reporting_the_pr());
    out.assert_not_green(
        "A",
        "a mismatch written where git's own trailer parser cannot see it",
    );
    out.assert_says("PMAT-111");
}
