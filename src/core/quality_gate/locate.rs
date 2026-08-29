//! Recover the YAML line a resource is declared on.
//!
//! `ForjarConfig` is deserialised by serde_yaml_ng into typed structs; spans
//! are discarded, so there is no line number anywhere in the parsed model. The
//! only recovery is re-scanning the raw file — and with `includes:` the
//! resource may not be in the addressed file at all. This module therefore
//! answers `None` freely, and a SARIF result with no `region` is the correct
//! output for a resource whose line cannot be established. An invented line is
//! worse than a missing one: a reviewer follows it to the wrong place.

use super::GateFinding;
use std::collections::HashMap;

/// Indent width of a line, in bytes of leading space.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Whether a line can hold a mapping key (not blank, not a comment).
fn is_key_line(trimmed: &str) -> bool {
    !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('-')
}

/// 1-based line of `<indent><resource_id>:` under the top-level `resources:`.
///
/// Only keys at the FIRST child indent are considered, so an `app-config:` that
/// appears nested inside another resource's block scalar cannot be mistaken for
/// a resource declaration.
pub fn resource_line(yaml_text: &str, resource_id: &str) -> Option<usize> {
    let lines: Vec<&str> = yaml_text.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_end() == "resources:" && indent_of(l) == 0)?;

    let mut child_indent: Option<usize> = None;
    for (offset, line) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = line.trim();
        if !is_key_line(trimmed) {
            continue;
        }
        let indent = indent_of(line);
        if indent == 0 {
            return None; // left the `resources:` block without a match
        }
        let expected = *child_indent.get_or_insert(indent);
        if indent == expected && trimmed.starts_with(&format!("{resource_id}:")) {
            return Some(offset + 1);
        }
    }
    None
}

/// Fill in `yaml_line` on every finding that names a resource in this file.
///
/// One lookup per distinct resource id, not one per finding: a config with a
/// hundred findings on one resource would otherwise rescan the file a hundred
/// times.
pub fn annotate(yaml_text: &str, findings: &mut [GateFinding]) {
    let mut cache: HashMap<String, Option<usize>> = HashMap::new();
    for f in findings.iter_mut() {
        if f.resource_id.is_empty() {
            continue;
        }
        let line = cache
            .entry(f.resource_id.clone())
            .or_insert_with(|| resource_line(yaml_text, &f.resource_id));
        f.yaml_line = *line;
    }
}
