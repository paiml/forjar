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

const ALIAS_TO_SHIPPED: &str = "pr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-901\n    evidence: \"squash trailer Pmat-Ticket: PMAT-901\"\n";

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
        aliases: "pr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-777\n    evidence: \"squash trailer Pmat-Ticket: PMAT-777\"\n",
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

// The alias map is a frozen history, not an open door: a PR merged after the
// freeze point names its ticket or the gate stays red (FORJAR-T-HISTORY).
const FROZEN_AT_FLOOR: &str =
    "pr_ticket_aliases_frozen_at: v0.0.1\npr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-901\n    evidence: \"squash trailer Pmat-Ticket: PMAT-901\"\n";

#[test]
fn an_alias_for_a_pr_merged_inside_the_freeze_is_green() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        aliases: FROZEN_AT_FLOOR,
        ..Case::default()
    });
    run(&fx, AN_HOUR).assert_green("PR #10 is an ancestor of the freeze point v0.0.1");
}

#[test]
fn an_alias_for_a_pr_merged_after_the_freeze_is_still_red() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        aliases: "pr_ticket_aliases_frozen_at: v0.0.0\npr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-901\n    evidence: \"squash trailer Pmat-Ticket: PMAT-901\"\n",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("PR #10 merged after the freeze v0.0.0, so its alias is not read");
    r.assert_says("name no roadmap ticket");
}

#[test]
fn a_freeze_point_the_repository_cannot_resolve_is_unmeasured() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        aliases: "pr_ticket_aliases_frozen_at: v9.9.9\npr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-901\n    evidence: \"squash trailer Pmat-Ticket: PMAT-901\"\n",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("the freeze point resolves to no commit");
    r.assert_says("UNMEASURED");
}

#[test]
fn an_alias_entry_with_an_empty_evidence_is_a_ledger_shape_error() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        aliases: "pr_ticket_aliases:\n  - pr: 10\n    ticket: PMAT-901\n    evidence: \"\"\n",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("an alias that cites no evidence is not a mapping");
    r.assert_says("pr_ticket_aliases");
}

#[test]
fn an_alias_may_name_a_gh_row() {
    let fx = fixture(Case {
        shipped_branch: "the-shipped-work",
        rows: vec![
            ("GH-881", vec!["release:v0.0.1"]),
            (OPEN, vec!["release:v0.0.2"]),
        ],
        aliases: "pr_ticket_aliases:\n  - pr: 10\n    ticket: GH-881\n    evidence: \"squash trailer Pmat-Ticket: GH-881\"\n",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    // the ledger row still declares PMAT-901 for the floor window, so this is
    // red on the declared/measured disagreement and NOT on the alias shape.
    r.assert_red("the declared ticket set no longer matches the measured one");
    assert!(
        !r.text.contains("pr_ticket_aliases"),
        "a GH-<n> row is a legal alias target, not a shape error: {}",
        r.text
    );
}

/// The committed ledger: every alias cites its evidence, targets a roadmap row,
/// and the map carries the freeze point that closes it to later PRs.
#[test]
fn the_committed_alias_map_is_evidenced_row_backed_and_frozen() {
    let root = env!("CARGO_MANIFEST_DIR");
    let read = |rel: &str| {
        let text = std::fs::read_to_string(format!("{root}/{rel}")).expect(rel);
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text).expect(rel)
    };
    let ledger = read("docs/roadmaps/releases.yaml");
    let roadmap = read("docs/roadmaps/roadmap.yaml");
    let rows: Vec<&str> = roadmap["roadmap"]
        .as_sequence()
        .expect("roadmap rows")
        .iter()
        .filter_map(|r| r["id"].as_str())
        .collect();
    let frozen = ledger["pr_ticket_aliases_frozen_at"]
        .as_str()
        .expect("the alias map declares pr_ticket_aliases_frozen_at");
    assert!(
        frozen.len() == 40 && frozen.bytes().all(|b| b.is_ascii_hexdigit()),
        "the freeze point is a full commit sha, got {frozen:?}"
    );
    let aliases = ledger["pr_ticket_aliases"].as_sequence().expect("aliases");
    assert!(!aliases.is_empty(), "the alias map is empty");
    let mut seen = std::collections::BTreeSet::new();
    for a in aliases {
        let pr = a["pr"].as_u64().expect("pr is a number");
        let ticket = a["ticket"].as_str().expect("ticket is a string");
        assert!(seen.insert(pr), "PR #{pr} is mapped twice");
        assert!(
            rows.contains(&ticket),
            "PR #{pr}: {ticket} is no roadmap row"
        );
        let ev = a["evidence"].as_str().unwrap_or("");
        assert!(
            ev.contains(ticket) || ev.contains(ticket.trim_start_matches("PMAT-")),
            "PR #{pr}: the evidence {ev:?} does not name {ticket}"
        );
    }
}
