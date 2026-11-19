//! PMAT-1200: the roadmap must say what shipped, and say it in text the YAML
//! parser keeps.
//!
//! * (a) a row whose status is `completed` never has empty acceptance criteria
//!   (scoped to the rows this ticket names, see `NAMED`);
//! * (b) the rows PMAT-1048, 1049, 1114, 1115, 1155 and 1200 are in a shipped
//!   state (`completed`; PMAT-1155 stays `inprogress` while its issue is open)
//!   with non-empty criteria;
//! * (c) no acceptance item is an unquoted plain scalar containing ` #`: YAML
//!   reads the rest of the line as a comment and the item is silently cut short.
//!
//! Reads `docs/roadmaps/roadmap.yaml` only: no network, no subprocess.

use serde_yaml_ng::Value;
use std::path::Path;

/// `(id, statuses the row may be in)`.
const NAMED: &[(&str, &[&str])] = &[
    ("PMAT-1048", &["completed"]),
    ("PMAT-1049", &["completed"]),
    ("PMAT-1114", &["completed"]),
    ("PMAT-1115", &["completed"]),
    ("PMAT-1155", &["completed", "inprogress"]),
    ("PMAT-1200", &["completed"]),
];

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// `(id, status, criteria)` for every row, from the real YAML parser.
fn rows(text: &str) -> Vec<(String, String, Vec<String>)> {
    let doc: Value = serde_yaml_ng::from_str(text).expect("roadmap.yaml parses");
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    doc.get("roadmap")
        .and_then(Value::as_sequence)
        .expect("roadmap: is a list")
        .iter()
        .map(|r| {
            let crit = r
                .get("acceptance_criteria")
                .and_then(Value::as_sequence)
                .map(|c| {
                    c.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            (s(r, "id"), s(r, "status"), crit)
        })
        .collect()
}

/// `(line number, item text)` of every unquoted plain-scalar acceptance item
/// whose raw text holds ` #` (the parser drops that text as a comment).
fn truncated_items(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_criteria = false;
    for (i, line) in text.lines().enumerate() {
        if line.starts_with("  acceptance_criteria:") {
            in_criteria = true;
        } else if in_criteria && !line.starts_with("  - ") {
            in_criteria = false;
        } else if in_criteria {
            let item = line["  - ".len()..].trim_start();
            let quoted = item.starts_with(['\'', '"', '|', '>', '[', '{']);
            if !quoted && item.contains(" #") {
                out.push((i + 1, item.chars().take(60).collect()));
            }
        }
    }
    out
}

#[test]
fn named_rows_are_shipped_with_criteria() {
    let all = rows(&read("docs/roadmaps/roadmap.yaml"));
    let mut bad = Vec::new();
    for (id, allowed) in NAMED {
        match all.iter().find(|(rid, _, _)| rid == id) {
            None => bad.push(format!("{id}: row missing")),
            Some((_, status, crit)) => {
                if !allowed.contains(&status.as_str()) {
                    bad.push(format!("{id}: status `{status}`, want one of {allowed:?}"));
                }
                if crit.is_empty() || crit.iter().any(|c| c.trim().is_empty()) {
                    bad.push(format!("{id}: acceptance criteria empty"));
                } else if *id != "PMAT-1200"
                    && !crit.iter().any(|c| c.contains(&format!("#{}", &id[5..])))
                {
                    bad.push(format!("{id}: criteria lost their `#<issue>` text"));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "roadmap rows not in their true state:\n{}",
        bad.join("\n")
    );
}

#[test]
fn completed_named_rows_never_have_empty_criteria() {
    let all = rows(&read("docs/roadmaps/roadmap.yaml"));
    let bad: Vec<_> = all
        .iter()
        .filter(|(id, st, c)| {
            st == "completed" && c.is_empty() && NAMED.iter().any(|(n, _)| n == id)
        })
        .map(|(id, _, _)| id.clone())
        .collect();
    assert!(
        bad.is_empty(),
        "completed rows with empty criteria: {bad:?}"
    );
}

#[test]
fn no_acceptance_item_hides_text_behind_an_unquoted_comment_marker() {
    let bad = truncated_items(&read("docs/roadmaps/roadmap.yaml"));
    let shown: Vec<String> = bad.iter().map(|(n, t)| format!("line {n}: {t}")).collect();
    assert!(
        bad.is_empty(),
        "unquoted acceptance items containing ` #` are cut short by YAML (quote them):\n{}",
        shown.join("\n")
    );
}

#[test]
fn the_comment_marker_check_catches_a_truncated_item() {
    let yaml = "  acceptance_criteria:\n  - Issue #1 closed\n  - 'Issue #2 closed'\n  - plain\n  phases: []\n";
    let hit = truncated_items(yaml);
    assert_eq!(hit.len(), 1);
    assert_eq!(hit[0].0, 2);
}
