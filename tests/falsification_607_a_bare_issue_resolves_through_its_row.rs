//! PMAT-607 (forjar#607): a PR that names no PMAT id may name its ticket as
//! the GitHub issue it was filed from, and only through a roadmap row.
//!
//! v1.33.0 shipped #643, titled "fixes #642", and #646, whose title says
//! `forjar#624` and whose body says `#624`. Neither names a `PMAT-<n>`, so
//! gate T's T3 was red on both, although the roadmap has a row for each issue
//! (`github_issue: 642` on PMAT-642, `github_issue: 624` on PMAT-624).
//!
//! The rule (`dogfood_pr_tickets` in scripts/dogfood/lib/window.sh, shared by
//! gates A and T): when a PR's branch, title and body name no PMAT id, the
//! FIRST bare `#N` in the title, then the body, resolves only through the row
//! whose `github_issue` is N. With no such row it stays red. The next `#N` is
//! never tried. Two rows filed from one issue resolve to neither.

#[path = "release_goal_fixture/mod.rs"]
mod fx;
use fx::*;
use std::process::Command;

/// What `dogfood_pr_tickets BRANCH TITLE BODY` resolves against `roadmap`,
/// printed as `TICKET=<id>|TICKETS=<ids>`, or the FAIL line it dies with.
fn tickets(roadmap: &str, branch: &str, title: &str, body: &str) -> (i32, String) {
    let fx = fixture(Case::default());
    write(&fx.root, "docs/roadmaps/roadmap.yaml", roadmap);
    let out = Command::new("bash")
        .args([
            "-c",
            "set -euo pipefail; fail() { echo \"FAIL $1\"; exit 1; }; \
             . scripts/dogfood/lib/window.sh; DOGFOOD_ROADMAP_REF=worktree; \
             dogfood_pr_tickets \"$1\" \"$2\" \"$3\"; \
             echo \"TICKET=${DOGFOOD_TICKET}|TICKETS=${DOGFOOD_TICKETS}\"",
            "tickets",
            branch,
            title,
            body,
        ])
        .current_dir(&fx.root)
        .output()
        .expect("bash must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text.trim().to_string())
}

/// A roadmap of rows `(id, github_issue)`; `None` writes `github_issue: null`.
fn rows(rows: &[(&str, Option<u32>)]) -> String {
    let mut text = String::from("roadmap_version: '1.0'\nroadmap:\n");
    for (id, issue) in rows {
        let issue = issue.map_or("null".to_string(), |n| n.to_string());
        text.push_str(&format!(
            "- id: {id}\n  github_issue: {issue}\n  title: fixture row\n  status: completed\n  labels: []\n"
        ));
    }
    text
}

const NONE: &str = "TICKET=|TICKETS=";

/// #643's shape: the title names the issue, and the row filed from it exists.
/// Under the old rule this was unticketed (red); the row makes it a ticket.
#[test]
fn a_bare_issue_in_the_title_resolves_through_the_row_filed_from_it() {
    let road = rows(&[("PMAT-642", Some(642)), ("PMAT-7", None)]);
    let (code, text) = tickets(&road, "fix/mount-owner", "fixes #642", "");
    assert_eq!(code, 0, "{text}");
    assert_eq!(text, "TICKET=PMAT-642|TICKETS=PMAT-642");
}

/// #646's shape: `forjar#624` in the title is not bare; the body's `#624` is.
#[test]
fn a_qualified_reference_is_not_bare_and_the_body_is_read_next() {
    let road = rows(&[("PMAT-624", Some(624))]);
    let (code, text) = tickets(&road, "fix/x", "fix: forjar#624 the thing", "Closes #624.");
    assert_eq!(code, 0, "{text}");
    assert_eq!(text, "TICKET=PMAT-624|TICKETS=PMAT-624");
}

/// No row was filed from the issue: the PR stays unticketed.
#[test]
fn an_issue_with_no_row_stays_red() {
    let road = rows(&[("PMAT-624", Some(624)), ("PMAT-642", None)]);
    let (code, text) = tickets(&road, "fix/x", "fixes #642", "");
    assert_eq!(code, 0, "{text}");
    assert_eq!(text, NONE, "a row id that equals N is not a declaration");
}

/// Only the first bare `#N` is read: a later one with a row is never tried,
/// as a stray id is never skipped for the next.
#[test]
fn the_next_issue_is_never_tried() {
    let road = rows(&[("PMAT-624", Some(624))]);
    let (code, text) = tickets(&road, "fix/x", "fixes #9", "and #624");
    assert_eq!(code, 0, "{text}");
    assert_eq!(text, NONE);
}

/// A reference glued to a word, a path or another reference is not bare.
#[test]
fn every_glued_form_is_not_bare() {
    let road = rows(&[("PMAT-5", Some(5))]);
    for title in [
        "forjar#5",
        "paiml/forjar#5",
        "PR-#5",
        "##5",
        "v1.#5",
        "x_#5",
    ] {
        let (code, text) = tickets(&road, "fix/x", title, "");
        assert_eq!(code, 0, "{title}: {text}");
        assert_eq!(text, NONE, "{title} is not a bare #5");
    }
    for title in ["#5", "(#5)", "fixes #5.", "[#5]"] {
        let (_, text) = tickets(&road, "fix/x", title, "");
        assert_eq!(text, "TICKET=PMAT-5|TICKETS=PMAT-5", "{title} is a bare #5");
    }
}

/// A PMAT id anywhere wins: the issue rule is read only when there is none.
#[test]
fn a_pmat_id_is_read_before_any_issue() {
    let road = rows(&[("PMAT-5", Some(5)), ("PMAT-6", None)]);
    let (_, text) = tickets(&road, "fix/x", "fixes #5", "PMAT-6");
    assert_eq!(text, "TICKET=PMAT-6|TICKETS=PMAT-6");
}

/// Two rows filed from one issue name two owners, which is none.
#[test]
fn two_rows_filed_from_one_issue_resolve_to_neither() {
    let road = rows(&[("PMAT-5", Some(5)), ("PMAT-6", Some(5))]);
    let (code, text) = tickets(&road, "fix/x", "fixes #5", "");
    assert_eq!(code, 1, "{text}");
    assert!(
        text.contains("FAIL more than one roadmap row declares github_issue: 5 (PMAT-5 PMAT-6)"),
        "{text}"
    );
}

/// Gate T end to end: PR #10 is titled "fixes #77" on a branch with no id.
/// Red while no row was filed from #77; green once PMAT-901 says it was.
#[test]
fn gate_t_is_red_with_no_row_and_green_with_the_row() {
    let bare = Case {
        shipped_branch: "fix-the-shipped-work",
        shipped_title: "fixes #77",
        ..Case::default()
    };
    let r = run(&fixture(bare), AN_HOUR);
    r.assert_red("PR #10 names only #77 and no row was filed from it");
    r.assert_says("#10");

    let filed = Case {
        shipped_branch: "fix-the-shipped-work",
        shipped_title: "fixes #77",
        github_issues: vec![(SHIPPED, 77)],
        ..Case::default()
    };
    run(&fixture(filed), AN_HOUR).assert_green("PMAT-901 says it was filed from #77");
}
