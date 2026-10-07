//! PMAT-607 phase 2 (forjar#607): a booked release row is never edited; the
//! tickets it missed are APPENDED as an amendment, and gate T reads both.
//!
//! v1.33.0 was booked by `release-goal.sh cut` while T3 still read a bare `#N`
//! as no ticket, so its row omits the two tickets #643 and #646 were filed
//! from, and the cut moved PMAT-642's `release:v1.33.0` to `release:v1.34.0`.
//! Once the rule reads `#N` through the row filed from it, the measured set is
//! larger than the declared one and gate T is red at T2, with no tool that can
//! say so on the record without rewriting a booked row.
//!
//! `release-goal.sh amend TAG` appends a record (tag, at, tickets, prs) to an
//! `amendments:` block above `releases:`, labels each ticket `release:TAG` and
//! takes back the `release:<following>` the cut moved it to. An amendment only
//! adds: lib/releases.sh refuses one that names an unbooked tag, re-declares a
//! ticket, names one twice, cites a PR the tag does not declare, or is dated
//! before the record above it.

#[path = "release_goal_fixture/mod.rs"]
mod fx;
use fx::*;

/// The world after v0.0.1 was booked under the old rule: PR #10 is titled
/// "fixes #77", PMAT-901 was filed from #77, the row declares no ticket and
/// the cut moved PMAT-901's goal label to the next release.
fn booked_under_the_old_rule() -> Case {
    Case {
        shipped_branch: "fix-the-shipped-work",
        shipped_title: "fixes #77",
        github_issues: vec![(SHIPPED, 77)],
        declared_tickets: "[]",
        rows: vec![
            (SHIPPED, vec!["release:v0.0.2"]),
            (OPEN, vec!["release:v0.0.2"]),
        ],
        ..Case::default()
    }
}

/// Everything from `releases:` down: the booked rows, which amend never touches.
fn booked(ledger: &str) -> &str {
    &ledger[ledger.find("\nreleases:\n").expect("a releases: block")..]
}

#[test]
fn a_booked_row_that_missed_a_ticket_is_red_until_amend_appends_it() {
    let fx = fixture(booked_under_the_old_rule());
    let r = run(&fx, AN_HOUR);
    r.assert_red("the window names PMAT-901 and the row declares no ticket");
    r.assert_says("the ledger declares tickets [] and the PRs of its window name [PMAT-901]");

    let before = read(&fx, "docs/roadmaps/releases.yaml");
    let t = tool(&fx, AN_HOUR, &["amend", FLOOR]);
    assert_eq!(t.code, 0, "amend must succeed:\n{}", t.text);
    t.assert_says("amended v0.0.1: +[PMAT-901] named by PR(s) [10]");
    t.assert_says("labelled PMAT-901 release:v0.0.1");
    t.assert_says("removed  PMAT-901 release:v0.0.2");

    let after = read(&fx, "docs/roadmaps/releases.yaml");
    assert_eq!(booked(&before), booked(&after), "the booked row was edited");
    let block = after.split("\nreleases:\n").next().expect("the head");
    assert!(
        block.contains("amendments:\n  - tag: v0.0.1\n    at: ")
            && block.ends_with("    tickets: [PMAT-901]\n    prs: [10]"),
        "the record, above releases:, carrying tag, at, tickets and prs:\n{after}"
    );
    let roadmap = read(&fx, "docs/roadmaps/roadmap.yaml");
    let p901 = roadmap.split("- id: PMAT-901").nth(1).expect("the row");
    let p901 = p901.split("- id: ").next().expect("one row");
    assert!(
        p901.contains("  - release:v0.0.1\n") && !p901.contains("release:v0.0.2"),
        "the label the cut moved forward came back:\n{p901}"
    );

    git(&fx.root, &["add", "-A"]);
    git(&fx.root, &["commit", "-qm", "amend v0.0.1"]);
    run(&fx, AN_HOUR).assert_green("the row plus its amendment is the measured set");

    let again = tool(&fx, AN_HOUR, &["amend", FLOOR]);
    assert_eq!(again.code, 2, "{}", again.text);
    again
        .assert_says("already declares every ticket its window names [PMAT-901]: nothing to amend");
}

/// The next cut appends its row under `releases:`, never into `amendments:`,
/// and the ledger it leaves still loads.
#[test]
fn a_cut_after_an_amendment_books_below_it() {
    let fx = fixture(booked_under_the_old_rule());
    assert_eq!(tool(&fx, AN_HOUR, &["amend", FLOOR]).code, 0);
    git(&fx.root, &["add", "-A"]);
    git(&fx.root, &["commit", "-qm", "amend v0.0.1"]);
    git(&fx.root, &["tag", "-a", "-m", NEXT, NEXT]);
    git(&fx.root, &["push", "-q", "origin", "main", "--tags"]);

    let c = tool(&fx, AN_HOUR, &["cut", NEXT, "--next", "v0.0.3"]);
    assert_eq!(c.code, 0, "cut must succeed:\n{}", c.text);
    let ledger = read(&fx, "docs/roadmaps/releases.yaml");
    let (head, rows) = ledger.split_once("\nreleases:\n").expect("releases:");
    assert!(
        head.contains("\namendments:\n  - tag: v0.0.1\n"),
        "{ledger}"
    );
    assert!(
        rows.contains("  - tag: v0.0.2\n") && !head.contains("tag: v0.0.2"),
        "the new row is a release, not an amendment:\n{ledger}"
    );
    let w = tool(&fx, AN_HOUR, &["window", NEXT]);
    assert_eq!(w.code, 0, "the ledger the cut left must load:\n{}", w.text);
}

#[test]
fn amend_refuses_a_tag_with_no_row() {
    let fx = fixture(Case {
        pre_cut: true,
        ..Case::default()
    });
    let t = tool(&fx, AN_HOUR, &["amend", FLOOR]);
    assert_eq!(t.code, 2, "{}", t.text);
    t.assert_says("v0.0.1 has no row");
}

/// amend only adds: a declared ticket the window does not name stays red.
#[test]
fn amend_refuses_to_take_a_declared_ticket_back() {
    let fx = fixture(Case {
        declared_tickets: "[PMAT-901, PMAT-902]",
        ..Case::default()
    });
    let before = read(&fx, "docs/roadmaps/releases.yaml");
    let t = tool(&fx, AN_HOUR, &["amend", FLOOR]);
    assert_eq!(t.code, 2, "{}", t.text);
    t.assert_says("v0.0.1 declares PMAT-902 and no PR of its window names it");
    assert_eq!(before, read(&fx, "docs/roadmaps/releases.yaml"));
}

/// A record written by hand is read like one amend wrote: the row plus it is
/// the declared set, so this is green — and red under a gate that reads the
/// row alone.
#[test]
fn gate_t_reads_the_row_and_its_amendments() {
    let mut case = booked_under_the_old_rule();
    case.rows = vec![
        (SHIPPED, vec!["release:v0.0.1"]),
        (OPEN, vec!["release:v0.0.2"]),
    ];
    case.amendments = "amendments:\n  - tag: v0.0.1\n    at: 2026-01-01T00:00:00Z\n    tickets: [PMAT-901]\n    prs: [10]\n";
    run(&fixture(case), AN_HOUR).assert_green("PMAT-901 is declared by the amendment");
}

const AT: &str = "2026-01-01T00:00:00Z";

/// Every record that does more than add is refused by name.
#[test]
fn an_amendment_that_does_not_only_add_is_red() {
    let one = |tag: &str, at: &str, tickets: &str, prs: &str| {
        format!("  - tag: {tag}\n    at: {at}\n    tickets: {tickets}\n    prs: {prs}\n")
    };
    let cases: Vec<(String, &str, &str)> = vec![
        (
            one("v0.0.9", AT, "[PMAT-901]", "[10]"),
            "[]",
            "amendment 1 names v0.0.9, which has no row to amend",
        ),
        (
            one(FLOOR, AT, "[PMAT-901]", "[10]"),
            "[PMAT-901]",
            "amendment 1 adds PMAT-901 to v0.0.1, which already declares it",
        ),
        (
            one(FLOOR, AT, "[PMAT-901, PMAT-901]", "[10]"),
            "[]",
            "amendment 1 names a ticket twice",
        ),
        (
            one(FLOOR, AT, "[PMAT-901]", "[11]"),
            "[]",
            "amendment 1 cites #11, which is not a PR v0.0.1 declares",
        ),
        (
            one(FLOOR, "2026-01-02T00:00:00Z", "[PMAT-901]", "[10]")
                + &one(FLOOR, AT, "[PMAT-902]", "[10]"),
            "[]",
            "amendment 2 is dated 2026-01-01T00:00:00Z, before the one above it",
        ),
        (
            one(FLOOR, AT, "[PMAT-901]", "[10]") + &one(FLOOR, AT, "[PMAT-901]", "[10]"),
            "[]",
            "amendment 2 adds PMAT-901 to v0.0.1, which already declares it",
        ),
        (
            one(FLOOR, AT, "[]", "[10]"),
            "[]",
            "amendments[] (if any) with tag, at (UTC), tickets and prs (each non-empty)",
        ),
    ];
    for (records, declared, says) in cases {
        let mut case = booked_under_the_old_rule();
        case.declared_tickets = declared;
        let text: &'static str = Box::leak(format!("amendments:\n{records}").into_boxed_str());
        case.amendments = text;
        let r = run(&fixture(case), AN_HOUR);
        r.assert_red(says);
        r.assert_says(says);
    }
}
