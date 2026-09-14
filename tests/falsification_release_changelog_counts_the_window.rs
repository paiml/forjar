//! PMAT-520: while a cut is in flight, the CHANGELOG counts what merged.
//!
//! # The defect
//!
//! The 1.29.0 cut's CHANGELOG opened *"Thirteen PRs across sixteen tickets"*.
//! Twelve PRs and fifteen tickets had merged. Nobody miscounted: **the
//! thirteenth is the cut's own PR**, which has not merged and cannot have
//! merged when the sentence is written. Gate A enumerated twelve, gate E
//! enumerated twelve, gate T counted twelve — and the sentence a reader would
//! actually read said thirteen, in the release's permanent record.
//!
//! Three review lanes caught it by hand on the last read before the cut. No
//! gate asked, and the same mistake is available to every future release for
//! exactly the same reason.
//!
//! # What T9 does
//!
//! The window is already measured two arms above — the same set gate A and
//! gate E enumerate — so the join costs nothing. T9 reads the release's own
//! CHANGELOG section, takes the first `<N> PRs across <M> tickets` claim in it,
//! and refuses a disagreement with what merged.
//!
//! Three rules keep it from being noise. A section that makes NO such claim is
//! not failed for silence. The comparison is case-insensitive, because whether
//! a count opens a sentence is the writer's business. And a count outside the
//! range this arm can spell is UNMEASURED and says so, rather than passing as
//! "no claim".
//!
//! It runs ONLY while a cut is in flight: before one the section does not
//! exist, and after the tag the window has moved on and the sentence is
//! correctly about a window that has closed.

#[path = "release_goal_fixture/mod.rs"]
mod fx;
use fx::*;

/// The fixture's window is one merged PR carrying one ticket.
fn changelog(claim: &str) -> String {
    format!("# Changelog\n\n## [0.0.2] - 2026-01-04\n\n{claim}\n\n**A behaviour.** Something shipped.\n\n## [0.0.1] - 2026-01-02\n\nThe release before.\n")
}

/// The exact defect: a count that includes the cut's own unmerged PR.
#[test]
fn a_changelog_that_counts_its_own_pr_is_named_and_red() {
    let fx = fixture(Case {
        version: NEXT.trim_start_matches('v'),
        changelog: Box::leak(changelog("Two PRs across two tickets.").into_boxed_str()),
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("one PR merged and the CHANGELOG claims two");
    r.assert_says("Two PRs across two tickets");
    r.assert_says("One PRs across One tickets");
    // The verdict has to say WHY, or the next writer makes the same correction
    // and the same mistake.
    r.assert_says("A cut's own PR has not merged when the sentence is written");
}

/// The true count passes, whatever case the prose puts it in.
#[test]
fn the_measured_count_passes_in_either_case() {
    for claim in [
        "One PR across one ticket.",
        "one pr across ONE TICKET.",
        "One PRs across one tickets.",
    ] {
        let fx = fixture(Case {
            version: NEXT.trim_start_matches('v'),
            changelog: Box::leak(changelog(claim).into_boxed_str()),
            ..Case::default()
        });
        run(&fx, AN_HOUR).assert_green(&format!(
            "the window is one PR and one ticket, and {claim:?} says so"
        ));
    }
}

/// Silence is not a claim. A release section that counts nothing is not failed
/// for counting nothing — this arm judges a sentence that exists, and inventing
/// an obligation to write one would turn a prose convention into a gate.
#[test]
fn a_section_that_makes_no_claim_is_not_failed_for_silence() {
    let fx = fixture(Case {
        version: NEXT.trim_start_matches('v'),
        changelog: Box::leak(
            changelog("The second cut under the two-day cadence.").into_boxed_str(),
        ),
        ..Case::default()
    });
    run(&fx, AN_HOUR)
        .assert_green("the section makes no count, so there is no count to disagree with");
}

/// The claim is read from THIS release's section, not from a neighbour's.
///
/// A CHANGELOG carries every past release, and each one opens with its own
/// count of its own window. Reading the wrong section would make the gate red
/// on a correct cut and — worse — green on an incorrect one whose predecessor
/// happened to match.
#[test]
fn a_neighbouring_release_section_is_not_this_ones_claim() {
    let fx = fixture(Case {
        version: NEXT.trim_start_matches('v'),
        // This release's section says the truth; the PREVIOUS one says
        // something else entirely. Only the first must be read.
        changelog: "# Changelog\n\n## [0.0.2] - 2026-01-04\n\nOne PR across one ticket.\n\n**A behaviour.** Something shipped.\n\n## [0.0.1] - 2026-01-02\n\nNine PRs across nine tickets.\n",
        ..Case::default()
    });
    run(&fx, AN_HOUR).assert_green("the previous release's count is about the previous release");
}

/// Before a cut is in flight the arm does not run at all.
///
/// `Case::default()` leaves Cargo.toml at the FLOOR's version, which is the
/// state between releases. A stale count left in a past section must not turn
/// the gate red when nothing is being cut.
#[test]
fn between_releases_a_stale_count_is_not_this_gates_business() {
    let fx = fixture(Case {
        changelog: Box::leak(changelog("Ninety PRs across ninety tickets.").into_boxed_str()),
        ..Case::default()
    });
    run(&fx, AN_HOUR).assert_green("no cut is in flight, so no release section is being written");
}
