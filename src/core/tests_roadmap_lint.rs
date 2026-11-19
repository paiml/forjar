//! PMAT-1200: the roadmap says what shipped, in text the YAML parser keeps.
//! Runs in the `cargo test --lib` job; reads `docs/roadmaps/roadmap.yaml` only. Named rows
//! are shipped with criteria (every row of that id); no plain scalar holds a comment marker.
//!
//! Known gaps: quoted, block (`|` `>`) and flow (`[` `{`) scalars are not
//! inspected; a ` #` inside a block-scalar body would be reported (none today);
//! "completed with empty criteria" is enforced for the named rows only.

use serde_yaml_ng::Value;

/// `(id, states the row may be in)`. A row stays `inprogress` while its issue is open.
const NAMED: &[(&str, &[&str])] = &[
    ("PMAT-1048", &["completed"]),
    ("PMAT-1049", &["completed"]),
    ("PMAT-1114", &["completed"]),
    ("PMAT-1115", &["completed"]),
    ("PMAT-1155", &["completed", "inprogress"]),
    ("PMAT-1200", &["completed", "inprogress"]),
];

fn roadmap_text() -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/roadmaps/roadmap.yaml");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

/// Problems with the named rows in `text` (empty = fine).
fn named_row_problems(text: &str) -> Vec<String> {
    let doc: Value = serde_yaml_ng::from_str(text).expect("roadmap parses");
    let rows = doc["roadmap"].as_sequence().expect("roadmap: is a list");
    let mut bad = Vec::new();
    for (id, allowed) in NAMED {
        let mine: Vec<_> = rows
            .iter()
            .filter(|r| r["id"].as_str() == Some(id))
            .collect();
        if mine.is_empty() {
            bad.push(format!("{id}: row missing"));
        }
        for r in mine {
            let st = r["status"].as_str().unwrap_or("");
            if !allowed.contains(&st) {
                bad.push(format!("{id}: status `{st}`, want one of {allowed:?}"));
            }
            let crit = r["acceptance_criteria"]
                .as_sequence()
                .cloned()
                .unwrap_or_default();
            let text_of = |c: &Value| c.as_str().unwrap_or("").trim().to_string();
            if crit.is_empty() || crit.iter().any(|c| text_of(c).is_empty()) {
                bad.push(format!("{id}: acceptance criteria empty or blank"));
            } else if *id != "PMAT-1200"
                && !crit
                    .iter()
                    .any(|c| text_of(c).contains(&format!("#{}", &id[5..])))
            {
                bad.push(format!("{id}: criteria lost their `#<issue>` text"));
            }
        }
    }
    bad
}

/// `(line, text)` of plain scalars whose raw text YAML would cut at a comment.
fn comment_cut_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let mut rest = line.trim_start();
        while let Some(r) = rest.strip_prefix("- ") {
            rest = r.trim_start();
        }
        if let Some((k, v)) = rest.split_once(": ") {
            if !k.is_empty()
                && k.chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                rest = v.trim_start();
            }
        }
        let plain = !rest.starts_with(['\'', '"', '|', '>', '[', '{', '&', '*', '!']);
        if plain
            && (rest.starts_with('#') && line.trim_start().starts_with("- ")
                || rest.contains(" #")
                || rest.contains("\t#"))
        {
            out.push((i + 1, rest.chars().take(60).collect()));
        }
    }
    out
}

#[test]
fn named_rows_are_shipped_with_criteria() {
    let bad = named_row_problems(&roadmap_text());
    assert!(
        bad.is_empty(),
        "roadmap rows not in their true state:\n{}",
        bad.join("\n")
    );
}

#[test]
fn no_plain_scalar_hides_text_behind_a_comment_marker() {
    let bad = comment_cut_lines(&roadmap_text());
    let shown: Vec<_> = bad.iter().map(|(n, t)| format!("line {n}: {t}")).collect();
    assert!(
        bad.is_empty(),
        "plain scalars cut short by YAML (quote them):\n{}",
        shown.join("\n")
    );
}

fn rows(extra: &str) -> String {
    let mut s = String::from("roadmap:\n");
    for (id, _) in NAMED {
        let n = &id[5..];
        s += &format!(
            "- id: {id}\n  status: completed\n  acceptance_criteria:\n  - 'Issue #{n} done'\n"
        );
    }
    s + extra
}

#[test]
fn checker_accepts_a_clean_roadmap() {
    assert!(named_row_problems(&rows("")).is_empty());
    assert!(comment_cut_lines(&rows("")).is_empty());
}

#[test]
fn checker_rejects_bad_named_rows() {
    let dup = rows("- id: PMAT-1048\n  status: planned\n  acceptance_criteria:\n  - 'x #1048'\n");
    assert!(named_row_problems(&dup)
        .iter()
        .any(|p| p.contains("PMAT-1048: status `planned`")));
    let blank = rows("- id: PMAT-1049\n  status: completed\n  acceptance_criteria:\n  - '  '\n");
    assert!(named_row_problems(&blank)
        .iter()
        .any(|p| p.contains("empty or blank")));
    let null = rows("- id: PMAT-1114\n  status: completed\n  acceptance_criteria:\n  -\n");
    assert!(named_row_problems(&null)
        .iter()
        .any(|p| p.contains("empty or blank")));
    let empty = rows("- id: PMAT-1115\n  status: completed\n  acceptance_criteria: []\n");
    assert!(named_row_problems(&empty)
        .iter()
        .any(|p| p.contains("empty or blank")));
    let gone = rows("").replace("PMAT-1155", "PMAT-9999");
    assert!(named_row_problems(&gone)
        .iter()
        .any(|p| p.contains("PMAT-1155: row missing")));
}

#[test]
fn checker_catches_every_comment_layout() {
    for bad in [
        "  - Issue #1 closed",
        "  - Issue\t#1 closed",
        "      - Issue #1 closed",
        "- - Issue #1 closed",
        "  - #1 closed",
        "  title: a thing #1",
        "  notes: a\t#1",
    ] {
        assert_eq!(comment_cut_lines(bad).len(), 1, "not caught: {bad:?}");
    }
    for ok in [
        "  - 'Issue #1'",
        "  - \"Issue #1\"",
        "  - plain text",
        "  title: 'a #1'",
        "# a comment",
        "  - a#b",
    ] {
        assert!(comment_cut_lines(ok).is_empty(), "false positive: {ok:?}");
    }
}
