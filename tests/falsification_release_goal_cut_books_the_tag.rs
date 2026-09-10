//! PMAT-225 (forjar#506): the cut flow. After `git tag`, the release exists
//! and the ledger does not know it: gate T is red by name until
//! `scripts/release-goal.sh cut TAG --next NEXT` books it — the measured row,
//! the next goal due exactly cadence_days later, `release:TAG` on every ticket
//! the tag shipped, and any `release:TAG` a goal carried without shipping
//! moved to `release:NEXT` (the plan grill: a goal that missed the cut must
//! not block it). Then `sync` labels the open window and the gate is green.

#[path = "release_goal_fixture/mod.rs"]
mod fx;
use fx::*;

#[test]
fn a_tag_that_was_cut_and_never_declared_is_red_until_cut_books_it() {
    let fx = fixture(Case {
        pre_cut: true,
        rows: vec![
            (SHIPPED, vec!["release:v0.0.1"]),
            (OPEN, vec![]),
            ("PMAT-903", vec!["release:v0.0.1"]),
        ],
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("v0.0.1 exists and the ledger has no row for it");
    r.assert_says("has no row");
    r.assert_says("release-goal.sh cut");

    let t = tool(&fx, AN_HOUR, &["cut", FLOOR, "--next", NEXT]);
    assert_eq!(t.code, 0, "cut must succeed:\n{}", t.text);
    t.assert_says("declared v0.0.1");
    // PMAT-901 carried release:v0.0.1 as its GOAL before the cut; the cut
    // confirms it rather than adding it twice.
    t.assert_says("already  PMAT-901 release:v0.0.1");
    t.assert_says("removed  PMAT-903 release:v0.0.1");
    t.assert_says("labelled PMAT-903 release:v0.0.2");

    let ledger = read(&fx, "docs/roadmaps/releases.yaml");
    assert!(ledger.contains("  - tag: v0.0.1\n"), "the row:\n{ledger}");
    assert!(
        ledger.contains("    prs: [10]\n"),
        "the measured PRs:\n{ledger}"
    );
    assert!(
        ledger.contains("    tickets: [PMAT-901]\n"),
        "the measured tickets:\n{ledger}"
    );
    assert!(
        ledger.contains("    dogfood: docs/audits/dogfood-0.0.1-receipt.md\n"),
        "the receipt the release must carry:\n{ledger}"
    );
    let due = iso(fx.cut + 2 * 86400);
    assert!(
        ledger.ends_with(&format!("next:\n  tag: v0.0.2\n  due: {due}\n")),
        "the next goal, due exactly cadence_days after this cut:\n{ledger}"
    );
    assert!(
        !ledger.contains("releases: []"),
        "the empty list became a list:\n{ledger}"
    );

    let roadmap = read(&fx, "docs/roadmaps/roadmap.yaml");
    let p903 = roadmap.split("- id: PMAT-903").nth(1).expect("the row");
    assert!(
        p903.contains("  - release:v0.0.2\n") && !p903.contains("  - release:v0.0.1\n"),
        "the goal that missed the cut moved to the next one:\n{p903}"
    );

    // The open window: PMAT-902 merged after the tag and is unlabelled until sync.
    let s = tool(&fx, AN_HOUR, &["sync", "--check"]);
    assert_eq!(
        s.code, 1,
        "--check reports the missing label and edits nothing:\n{}",
        s.text
    );
    s.assert_says("MISSING  PMAT-902 release:v0.0.2");
    let s = tool(&fx, AN_HOUR, &["sync"]);
    assert_eq!(s.code, 0, "sync labels it:\n{}", s.text);
    s.assert_says("labelled PMAT-902 release:v0.0.2");

    // The gate reads HEAD: booked and committed, the goal is green.
    git(&fx.root, &["add", "-A"]);
    git(&fx.root, &["commit", "-qm", "book v0.0.1"]);
    let r = run(&fx, AN_HOUR);
    r.assert_green("the cut is booked, the labels agree, the next goal is declared");
    r.assert_says("1 of 1 PR(s) merged since v0.0.1 carry release:v0.0.2");

    // The status line reads the working tree and says so.
    let s = tool(&fx, AN_HOUR, &["show"]);
    assert_eq!(s.code, 0, "{}", s.text);
    s.assert_says("v0.0.2 ");
    s.assert_says("1h/48h left=47h");
    s.assert_says("1 merged, 1 tagged");
    s.assert_says("basis=docs/roadmaps/releases.yaml:L");
    s.assert_says("ledger=worktree");
}
