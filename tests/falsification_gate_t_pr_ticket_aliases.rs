//! forjar#1235: gate T and a PR that names no `PMAT-<n>` id at all.
//!
//! PR #944 named its work by GitHub issue (`GH-667`, `forjar#669`), which the
//! gate's id rule does not read, so no `alias:<id>` label can be declared for
//! it: an alias maps a STRAY id, and #944 names none. The ledger therefore
//! carries `pr_ticket_aliases`, a PR number mapped to the row that owns it.
//! The map is read ONLY for a PR that names no id; it never overrides a stray
//! and it never names a row the registry lacks.

#[path = "release_goal_fixture/mod.rs"]
mod fx;
use fx::*;

const ALIAS_TO_SHIPPED: &str = "pr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-901\n";

#[test]
fn a_pr_naming_no_ticket_is_red_without_a_declared_alias() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PR #10 names no roadmap ticket");
    r.assert_says("#10");
    r.assert_says("name no roadmap ticket");
}

#[test]
fn a_declared_alias_to_a_row_tickets_the_pr_and_is_green() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        aliases: ALIAS_TO_SHIPPED,
        ..Case::default()
    });
    run(&fx, AN_HOUR).assert_green("PR #10 is mapped to PMAT-901, a row labelled release:v0.0.1");
}

#[test]
fn an_alias_to_a_row_the_registry_lacks_is_still_red() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        aliases: "pr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-777\n",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PMAT-777 is no row, so PR #10 stays unticketed");
    r.assert_says("name no roadmap ticket");
}

#[test]
fn an_alias_never_overrides_a_stray_id_the_pr_names() {
    let fx = fixture(Case {
        shipped_branch: "PMAT-999-misnamed",
        aliases: ALIAS_TO_SHIPPED,
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("the branch names PMAT-999, a stray; the alias map does not paper over it");
    r.assert_says("PMAT-999");
}

#[test]
fn a_malformed_alias_entry_is_a_ledger_shape_error() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        aliases: "pr_ticket_aliases:\n  - pr: ten\n    ticket: GH-669\n",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("the alias entry is not {pr: number, ticket: PMAT-n}");
    r.assert_says("pr_ticket_aliases");
}
