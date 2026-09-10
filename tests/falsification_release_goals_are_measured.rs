//! PMAT-225 (forjar#506): gate T — every tagged release is declared, every
//! ticket names the tag that shipped it, and the next cut is on time.
//!
//! The gate joins a DECLARED side (`docs/roadmaps/releases.yaml` and the
//! `release:<tag>` labels on `docs/roadmaps/roadmap.yaml` rows) against a
//! MEASURED one (git's tags and their creation instants, the PRs a stubbed
//! `gh` reports merged, placed by ancestry, and a clock the test pins with
//! `DOGFOOD_NOW`). Every case below drives the REAL script over a temp
//! repository with a bare `origin`, changes ONE thing, and asserts the gate is
//! red for that reason by name — or green. A gate that exited 1 without its
//! `GATE T FAIL` line is a death, not a verdict, and is a failure here too.

#[path = "release_goal_fixture/mod.rs"]
mod fx;
use fx::*;

#[test]
fn a_declared_ledger_that_matches_git_and_github_is_green_and_counts() {
    let fx = fixture(Case::default());
    let r = run(&fx, AN_HOUR);
    r.assert_green("the ledger, the labels and the window agree");
    r.assert_says("1 tagged release(s) since v0.0.1");
    r.assert_says("1 of 1 PR(s) merged since v0.0.1 carry release:v0.0.2");
    r.assert_says("47h left");
}

#[test]
fn a_declared_window_that_disagrees_with_github_is_named_and_red() {
    let fx = fixture(Case {
        declared_prs: "[10, 12]",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("the ledger declares PR #12, which GitHub does not report in v0.0.1's window");
    r.assert_says("v0.0.1");
    r.assert_says("declared window is not the measured one");
}

#[test]
fn a_merged_ticket_without_the_next_release_label_is_named_and_red() {
    let fx = fixture(Case {
        rows: vec![(SHIPPED, vec!["release:v0.0.1"]), (OPEN, vec![])],
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PMAT-902 merged since v0.0.1 and carries no release:v0.0.2");
    r.assert_says(OPEN);
    r.assert_says("release:v0.0.2");
}

#[test]
fn a_shipped_ticket_without_its_release_label_is_named_and_red() {
    let fx = fixture(Case {
        rows: vec![(SHIPPED, vec![]), (OPEN, vec!["release:v0.0.2"])],
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PMAT-901 shipped in v0.0.1 and carries no release:v0.0.1");
    r.assert_says(SHIPPED);
    r.assert_says("release:v0.0.1");
}

#[test]
fn a_ticket_claiming_a_release_it_was_not_in_is_red() {
    let fx = fixture(Case {
        rows: vec![
            (SHIPPED, vec!["release:v0.0.1"]),
            (OPEN, vec!["release:v0.0.2"]),
            ("PMAT-903", vec!["release:v0.0.1"]),
        ],
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PMAT-903 claims v0.0.1 and no PR of that window names it");
    r.assert_says("PMAT-903");
    r.assert_says("fabricated");
}

#[test]
fn a_row_naming_a_tag_origin_does_not_carry_is_red() {
    let fx = fixture(Case {
        extra_row: "  - tag: v0.0.5\n    cut: 2026-01-01T00:00:00Z\n    prs: []\n    tickets: []\n",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("the ledger declares v0.0.5, a release that does not exist");
    r.assert_says("v0.0.5");
}

#[test]
fn a_declared_due_that_is_not_cut_plus_cadence_is_red() {
    let fx = fixture(Case {
        due_skew: 1,
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("next.due is one second off the derived instant");
    r.assert_says("derived one disagree");
}

#[test]
fn an_overdue_cut_is_red_until_the_version_is_bumped() {
    let fx = fixture(Case::default());
    let r = run(&fx, 2 * 86400 + 3 * AN_HOUR);
    r.assert_red("three hours past due with a PR merged and the version unbumped");
    r.assert_says("OVERDUE by 3h");

    let bumped = fixture(Case {
        version: "0.0.2",
        ..Case::default()
    });
    let r = run(&bumped, 2 * 86400 + 3 * AN_HOUR);
    r.assert_green("Cargo.toml is at the next version: the cut is in flight");
    r.assert_says("cut in flight");
}

#[test]
fn a_tree_at_a_version_no_goal_names_is_red() {
    let fx = fixture(Case {
        version: "0.0.7",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("0.0.7 is neither the newest tag nor the declared next");
    r.assert_says("0.0.7");
}

#[test]
fn a_release_at_the_dogfood_floor_without_its_receipt_is_red() {
    let fx = fixture(Case {
        receipts: false,
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("v0.0.1 is at the dogfood floor and its receipt is not at HEAD");
    r.assert_says("docs/audits/dogfood-0.0.1-receipt.md");

    let with = fixture(Case::default());
    with.assert_committed("docs/audits/dogfood-0.0.1-receipt.md");
    run(&with, AN_HOUR).assert_green("the receipt and the crux document are at HEAD");
}

#[test]
fn a_stray_id_in_a_tagged_window_is_red_until_a_row_declares_the_alias() {
    let fx = fixture(Case {
        shipped_branch: "PMAT-999-misnamed",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PR #10's branch names PMAT-999, which is no row and no declared alias");
    r.assert_says("PMAT-999");
    r.assert_says("alias");

    let declared = fixture(Case {
        shipped_branch: "PMAT-999-misnamed",
        rows: vec![
            (SHIPPED, vec!["release:v0.0.1", "alias:PMAT-999"]),
            (OPEN, vec!["release:v0.0.2"]),
        ],
        ..Case::default()
    });
    run(&declared, AN_HOUR).assert_green("PMAT-901 declares alias:PMAT-999 and carries the label");
}

#[test]
fn a_gh_that_cannot_answer_is_unmeasured_and_red() {
    let fx = fixture(Case::default());
    let r = run_at(&fx, AN_HOUR, "false");
    r.assert_red("gh exited 1: the window is unmeasured");
    r.assert_says("UNMEASURED");
}
