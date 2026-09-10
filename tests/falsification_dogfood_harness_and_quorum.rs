//! PMAT-201: gates A (a harness receipt per merged PR) and E (a quorum receipt
//! per merged PR) are MECHANICAL, and these are the falsifiers that say so.
//!
//! # Why this file exists
//!
//! The dogfood contract promises eight standing requirements enforced "from
//! Make, with no agent". Six of them were shell; A and E were prose in
//! `.claude/skills/forjar-dogfood/SKILL.md`, run by an agent that read GitHub
//! and decided. An agent that decides is not a gate: it cannot be shown to go
//! red, it does not run in `make dogfood-release`, and its verdict is a
//! paragraph rather than an exit code. The PMAT-163 merge review refuted the
//! branch on exactly that.
//!
//! So A and E are now `scripts/dogfood/harness.sh` and
//! `scripts/dogfood/quorum.sh`, sharing the release window in
//! `scripts/dogfood/lib/window.sh`.
//!
//! # What a green run here proves
//!
//! That both scripts exist, and for each of them:
//!
//! 1. A `gh` that cannot answer is a FAIL carrying the word UNMEASURED — never
//!    a PASS over "no PRs found". The set of PRs merged since the last tag is a
//!    fact only GitHub holds, and an unenumerable window is not an empty one.
//! 2. A page that fills `--limit` is UNMEASURED, because a truncated window
//!    would let the gate check a subset while reporting the whole.
//! 3. Each defect the gate exists to catch — a merged PR with no ticket id, a
//!    missing harness receipt, a receipt that was truncated (no END marker),
//!    a receipt with two verdicts, a missing quorum receipt, a quorum below the
//!    lane floor, a quorum with a waiver key — produces a non-zero exit AND a
//!    `GATE <letter> FAIL` line naming the offender. An exit code with no
//!    verdict line is a death, not a judgement, and is asserted against.
//! 4. Both gates CAN pass. Without that half, every assertion above would be
//!    satisfied by a script that only ever fails.
//!
//! # Why these run the real scripts
//!
//! The subject is a shell script; a Rust re-implementation of its logic would
//! stay green over a script that no longer exists. Each case builds a small
//! repository shaped the way the gates read one — a `v*` tag, a commit after it
//! standing in for the squash merge, the receipts committed in that commit —
//! copies the REAL scripts into it, and runs them. `gh` is injected through
//! `$GH`, exactly as `scripts/dogfood/release-check.sh` names the tool it
//! requires: `GH=false` is "gh cannot answer", a stub is "gh answers this".
//! Nothing here touches the network or the real repository.

#![cfg(unix)]

#[path = "dogfood_gates_harness/mod.rs"]
mod harness;

use harness::*;

// ------------------------------------------------------------------ gate A

#[test]
fn gate_a_a_gh_that_cannot_answer_is_unmeasured_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    // `false` is a tool that runs and refuses to answer — the shape of a
    // missing, unauthenticated or rate-limited gh.
    let r = run(&fx, "harness.sh", "false");
    r.assert_not_green(
        "A",
        "gh could not enumerate the merged PRs, so the window is UNMEASURED and \
         UNMEASURED is a failure, never a pass over an empty set",
    );
    r.assert_says("UNMEASURED");
}

#[test]
fn gate_a_a_full_page_is_unmeasured_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let gh = fx.gh_filling_the_page();
    let r = run(&fx, "harness.sh", &gh);
    r.assert_not_green(
        "A",
        "GitHub filled the --limit, so the window may be truncated and the gate \
         would check a subset while reporting the whole",
    );
    r.assert_says("UNMEASURED");
}

#[test]
fn gate_a_a_merged_pr_with_no_ticket_id_is_named_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let gh = fx.gh_reporting("fix/no-ticket-anywhere", "a title", "a body");
    let r = run(&fx, "harness.sh", &gh);
    r.assert_not_green(
        "A",
        "a merged PR that names no ticket in its branch, title or body has no \
         address at which a harness receipt could be looked for",
    );
    r.assert_says(&format!("#{PR}"));
}

/// PMAT-225: PR #496's branch was named after a ticket that never existed
/// (PMAT-218) while its title named the real one (PMAT-219). The first id is
/// the address, as it always was — and an id that is not a roadmap row is
/// named and red rather than skipped for the next one: skipping would let a
/// branch whose row was forgotten fall through to an older ticket in the body
/// whose receipt already exists, and pass over the missing work.
#[test]
fn gate_a_a_first_id_that_is_not_a_roadmap_row_is_named_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let gh = fx.gh_reporting("PMAT-997/misnamed", "the real work (PMAT-999)", "");
    let r = run(&fx, "harness.sh", &gh);
    r.assert_not_green(
        "A",
        "the branch names PMAT-997, which is no roadmap row and which no row \
         declares as its alias; the title's PMAT-999 must not be reached for",
    );
    r.assert_says("PMAT-997");
    r.assert_says("alias:");
}

/// The declared way through: the row that owns the work says `alias:PMAT-997`
/// and carries the receipt, so the misnamed branch resolves to it.
#[test]
fn gate_a_a_stray_id_resolves_through_the_row_that_declares_it_as_alias() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    fx.declare_alias("PMAT-997", "PMAT-998");
    let gh = fx.gh_reporting("PMAT-997/misnamed", "the real work", "");
    let r = run(&fx, "harness.sh", &gh);
    r.assert_green(
        "A",
        "PMAT-998 declares alias:PMAT-997 and carries its receipt at HEAD",
    );
    r.assert_says("PMAT-998");
}

#[test]
fn gate_a_a_missing_harness_receipt_is_named_and_red() {
    let fx = fixture(None, Some(good_quorum_receipt()));
    let r = run(&fx, "harness.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "A",
        "PR #77 is in this window with no docs/audits/impl-PMAT-999-receipt.md, \
         so nothing says it was implemented under the harness",
    );
    r.assert_says(&impl_receipt_path());
}

#[test]
fn gate_a_a_receipt_without_the_end_marker_is_red() {
    let fx = fixture(Some(&truncated_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(&fx, "harness.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "A",
        "the END marker is what makes truncation detectable rather than silent; \
         a receipt without it may be missing the half that mattered",
    );
    r.assert_says(&format!("IMPL-{TICKET}-RECEIPT-END"));
}

#[test]
fn gate_a_a_receipt_with_two_verdicts_is_red() {
    let fx = fixture(
        Some(&two_verdict_impl_receipt()),
        Some(good_quorum_receipt()),
    );
    let r = run(&fx, "harness.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "A",
        "two verdict lines is no verdict: a reader cannot tell which one the \
         run reached",
    );
    r.assert_says("verdict");
}

#[test]
fn gate_a_a_pr_with_its_harness_receipt_passes() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(&fx, "harness.sh", &fx.gh_reporting_the_pr());
    r.assert_green(
        "A",
        "a window whose one PR carries a well-formed harness receipt must pass, \
         or every failure above proves nothing about the gate",
    );
    // The count is asserted because "0 of 0 PRs carry a receipt" is the vacuous
    // pass this whole file refuses: a green line here must say ONE.
    r.assert_says("1 of 1");
    r.assert_says(&format!("GATE A #{PR} {TICKET} {}", impl_receipt_path()));
}

#[test]
fn gate_a_the_previous_releases_own_pr_is_outside_the_window() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(
        &fx,
        "harness.sh",
        &fx.gh_reporting_the_pr_and_the_previous_release(),
    );
    r.assert_green(
        "A",
        "the previous release's PR (#76, merged at the tagged commit) carries no \
         ticket id and no receipt in this tree, and it belongs to the PREVIOUS \
         window: counting it would turn every release after the first RED",
    );
    r.assert_says("#76");
    r.assert_says("previous release");
    r.assert_says("1 of 1");
}

#[test]
fn gate_a_commits_since_the_tag_with_no_merged_pr_are_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(&fx, "harness.sh", &fx.gh_reporting_no_prs());
    r.assert_not_green(
        "A",
        "one commit reached HEAD after the tag and gh reports no merged PR \
         containing it: work that bypassed review must not read as an empty \
         window, or the gate passes vacuously over exactly what it exists to catch",
    );
    r.assert_says("no merged PR");
}

// ------------------------------------------------------------------ gate E

#[test]
fn gate_e_a_gh_that_cannot_answer_is_unmeasured_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(&fx, "quorum.sh", "false");
    r.assert_not_green(
        "E",
        "gh could not enumerate the merged PRs, so the window is UNMEASURED and \
         UNMEASURED is a failure",
    );
    r.assert_says("UNMEASURED");
}

#[test]
fn gate_e_a_missing_quorum_receipt_is_named_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), None);
    let r = run(&fx, "quorum.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "E",
        "PR #77 merged with no committed .quorum/<slug>.json, so its claims were \
         never refuted by anything this gate can read",
    );
    r.assert_says(&quorum_receipt_path());
}

#[test]
fn gate_e_a_thin_quorum_is_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(thin_quorum_receipt()));
    let r = run(&fx, "quorum.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "E",
        "2 lanes is below the floor of 3 that scripts/quorum-gate.sh enforces at \
         push time, so the receipt never cleared the bar it claims to record",
    );
    r.assert_says("lanes");
}

#[test]
fn gate_e_a_waiver_anywhere_in_the_receipt_is_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(waived_quorum_receipt()));
    let r = run(&fx, "quorum.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "E",
        "a waiver is an unrefuted claim wearing a receipt's clothes, and nesting \
         it one level deeper must not buy it a pass",
    );
    r.assert_says("waiv");
}

#[test]
fn gate_e_a_pr_with_its_quorum_receipt_passes() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(&fx, "quorum.sh", &fx.gh_reporting_the_pr());
    r.assert_green(
        "E",
        "a window whose one PR carries a receipt at the floor must pass, or \
         every failure above proves nothing about the gate",
    );
    r.assert_says("1 of 1");
    r.assert_says(&format!("GATE E #{PR} {}", quorum_receipt_path()));
}

#[test]
fn gate_e_the_previous_releases_own_pr_is_outside_the_window() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(
        &fx,
        "quorum.sh",
        &fx.gh_reporting_the_pr_and_the_previous_release(),
    );
    r.assert_green(
        "E",
        "the previous release's PR is the previous window's; its receipt lives \
         at that tag, not here",
    );
    r.assert_says("#76");
    r.assert_says("previous release");
    r.assert_says("1 of 1");
}

#[test]
fn gate_e_commits_since_the_tag_with_no_merged_pr_are_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(&fx, "quorum.sh", &fx.gh_reporting_no_prs());
    r.assert_not_green(
        "E",
        "a commit on main with no merged PR has no receipt anywhere; \
         \"PASS 0 of 0\" over it would be the vacuous pass this file refuses",
    );
    r.assert_says("no merged PR");
}
