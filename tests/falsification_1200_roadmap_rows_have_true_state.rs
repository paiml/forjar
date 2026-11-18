//! PMAT-1200 (#1200): the post-merge defect rows (PMAT-1048, 1049, 1114,
//! 1115, 1155) carry their true state. Each row must not be `planned`, and
//! must name its acceptance criteria; a `completed` row among them must too.
//!
//! Reads `docs/roadmaps/roadmap.yaml` only: no network, no cargo subprocess.

use std::path::Path;

const ROWS: [&str; 5] = [
    "PMAT-1048",
    "PMAT-1049",
    "PMAT-1114",
    "PMAT-1115",
    "PMAT-1155",
];

fn load() -> serde_yaml_ng::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/roadmaps/roadmap.yaml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("roadmap.yaml does not parse: {e}"))
}

fn row<'a>(doc: &'a serde_yaml_ng::Value, id: &str) -> &'a serde_yaml_ng::Value {
    doc["roadmap"]
        .as_sequence()
        .expect("roadmap: is a list")
        .iter()
        .find(|r| r["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("{id}: row missing from roadmap.yaml"))
}

fn criteria(r: &serde_yaml_ng::Value) -> usize {
    r["acceptance_criteria"].as_sequence().map_or(0, |s| {
        s.iter()
            .filter(|c| c.as_str().is_some_and(|t| !t.trim().is_empty()))
            .count()
    })
}

#[test]
fn roadmap_yaml_parses_and_lists_the_five_rows() {
    let doc = load();
    for id in ROWS {
        let _ = row(&doc, id);
    }
}

#[test]
fn no_defect_row_is_planned_or_without_acceptance_criteria() {
    let doc = load();
    for id in ROWS {
        let r = row(&doc, id);
        let status = r["status"].as_str().unwrap_or("");
        assert!(
            !status.is_empty() && status != "planned",
            "{id}: status is {status:?}; set it to its true state"
        );
        assert!(criteria(r) > 0, "{id}: acceptance_criteria is empty");
    }
}

#[test]
fn a_completed_defect_row_names_what_was_done() {
    let doc = load();
    for id in ROWS {
        let r = row(&doc, id);
        if r["status"].as_str() == Some("completed") {
            assert!(
                criteria(r) > 0,
                "{id}: completed with no acceptance_criteria"
            );
        }
    }
}
