//! Falsification: `docs/specifications/forjar-state-generation-ownership.md`
//! (PMAT-162, v10 REVIEWED — nine three-lane review rounds) locks the
//! document's SHAPE: the rule numbering, the falsifier table, the mutation
//! catalog, the interface names, the review record and the data example. A
//! future edit that silently drops a rule, a mutation, or a keyword the
//! implementation (deferred to 1.27) will depend on is caught here before it
//! merges. This suite exercises no forjar code — only the spec's text.

use std::fs;
use std::path::PathBuf;

fn spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("docs/specifications/forjar-state-generation-ownership.md")
}

fn spec_text() -> String {
    fs::read_to_string(spec_path()).unwrap_or_else(|e| {
        panic!("docs/specifications/forjar-state-generation-ownership.md is unreadable: {e}")
    })
}

/// (1) the file exists and its status line names v10 and REVIEWED.
#[test]
fn status_line_names_v10_and_reviewed() {
    let text = spec_text();
    let status_line = text
        .lines()
        .find(|l| l.trim_start().starts_with("Status:"))
        .unwrap_or_else(|| panic!("no 'Status:' line found in the spec"));
    assert!(
        status_line.contains("v10"),
        "status line does not name v10: {status_line:?}"
    );
    assert!(
        status_line.contains("REVIEWED"),
        "status line does not say REVIEWED: {status_line:?}"
    );
}

/// (2) R1 through R9 appear as `R<n> **` bold rule headings, in order, in §2.
#[test]
fn rules_r1_through_r9_appear_as_bold_headings_in_order() {
    let text = spec_text();
    let mut last_pos = 0usize;
    for n in 1..=9 {
        let needle = format!("R{n} **");
        let pos = text
            .find(&needle)
            .unwrap_or_else(|| panic!("rule heading `{needle}` not found in §2"));
        assert!(
            pos > last_pos,
            "rule heading `{needle}` is out of order (found at byte {pos}, expected after byte {last_pos})"
        );
        last_pos = pos;
    }
}

/// (3) §9's falsifier table has a `| R<n> |` row for every rule R1-R9.
#[test]
fn falsifier_table_has_a_row_for_every_rule_r1_through_r9() {
    let text = spec_text();
    for n in 1..=9 {
        let needle = format!("| R{n} |");
        assert!(
            text.contains(&needle),
            "§9 falsifier table has no `{needle}` row for rule R{n}"
        );
    }
}

/// (4) §9's mutation list names a mutation (`→ R<n> RED`) for every rule R1-R9.
#[test]
fn mutation_list_names_a_mutation_for_every_rule_r1_through_r9() {
    let text = spec_text();
    for n in 1..=9 {
        let needle = format!("R{n} RED");
        assert!(
            text.contains(&needle),
            "§9 mutation list has no `→ {needle}` entry for rule R{n}"
        );
    }
}

/// (5) §5 names `restore_decision` with a `RestoreVerb` parameter and `RestoreScope`.
#[test]
fn interfaces_name_restore_decision_with_restore_verb_and_restore_scope() {
    let text = spec_text();
    assert!(
        text.contains("restore_decision"),
        "§5 does not name restore_decision"
    );
    assert!(text.contains("RestoreVerb"), "§5 does not name RestoreVerb");
    assert!(
        text.contains("RestoreScope"),
        "§5 does not name RestoreScope"
    );
}

/// (6) §11 records a `v9 review` line containing `3/3 PASS`.
#[test]
fn review_record_shows_v9_review_three_of_three_pass() {
    let text = spec_text();
    let line = text
        .lines()
        .find(|l| l.contains("v9 review"))
        .unwrap_or_else(|| panic!("no 'v9 review' line found in §11"));
    assert!(
        line.contains("3/3 PASS"),
        "v9 review line does not contain 3/3 PASS: {line:?}"
    );
}

/// (7) §4's example carries `parent:` and `restores:` keys.
#[test]
fn section_4_example_carries_parent_and_restores_keys() {
    let text = spec_text();
    assert!(text.contains("parent:"), "§4 example has no `parent:` key");
    assert!(
        text.contains("restores:"),
        "§4 example has no `restores:` key"
    );
}

/// (8) the document contains no `TODO`, `TBD` or `owner.yaml` (the withdrawn design).
#[test]
fn the_document_has_no_todo_tbd_or_the_withdrawn_owner_yaml_design() {
    let text = spec_text();
    assert!(!text.contains("TODO"), "the spec contains a TODO marker");
    assert!(!text.contains("TBD"), "the spec contains a TBD marker");
    assert!(
        !text.contains("owner.yaml"),
        "the spec still names the withdrawn owner.yaml design"
    );
}
