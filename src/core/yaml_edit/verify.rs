//! Proving that a text edit changed exactly what it meant to change.
//!
//! The scanner in the parent module edits bytes. Bytes have no semantics, so
//! "the splice landed where I intended" is not something the splice can assert
//! about itself. This module answers it from the outside: parse the document
//! BEFORE and AFTER as `serde_yaml_ng::Value`, collect every path whose value
//! differs, and hand the set back. A caller compares it against the set of
//! paths it intended to change and discards the whole edit on any mismatch.
//!
//! That is why a wrong anchor cannot ship a corrupted config: an anchor that
//! landed on the wrong node adds a path the caller did not intend, and an
//! anchor that landed nowhere leaves one missing.

use serde_yaml_ng::Value;
use std::collections::BTreeSet;

/// Every path at which `a` and `b` differ.
///
/// A path is a `Vec<String>` rather than a dotted string so a key containing a
/// `.` cannot alias a nested path.
pub fn changed_paths(a: &Value, b: &Value) -> BTreeSet<Vec<String>> {
    let mut out = BTreeSet::new();
    walk(&mut Vec::new(), a, b, &mut out);
    out
}

fn walk(prefix: &mut Vec<String>, a: &Value, b: &Value, out: &mut BTreeSet<Vec<String>>) {
    match (a, b) {
        (Value::Mapping(x), Value::Mapping(y)) => walk_map(prefix, x, y, out),
        (Value::Sequence(x), Value::Sequence(y)) if x.len() == y.len() => {
            for (i, (ea, eb)) in x.iter().zip(y.iter()).enumerate() {
                prefix.push(i.to_string());
                walk(prefix, ea, eb, out);
                prefix.pop();
            }
        }
        _ => {
            if a != b {
                out.insert(prefix.clone());
            }
        }
    }
}

fn walk_map(
    prefix: &mut Vec<String>,
    x: &serde_yaml_ng::Mapping,
    y: &serde_yaml_ng::Mapping,
    out: &mut BTreeSet<Vec<String>>,
) {
    let mut keys: Vec<&Value> = Vec::new();
    for k in x.keys().chain(y.keys()) {
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    for k in keys {
        prefix.push(key_label(k));
        match (x.get(k), y.get(k)) {
            (Some(va), Some(vb)) => walk(prefix, va, vb, out),
            _ => {
                out.insert(prefix.clone());
            }
        }
        prefix.pop();
    }
}

/// A stable label for a mapping key of any YAML shape.
fn key_label(key: &Value) -> String {
    match key {
        Value::String(s) => s.clone(),
        other => serde_yaml_ng::to_string(other)
            .unwrap_or_default()
            .trim_end()
            .to_string(),
    }
}

/// Parse both documents and report the paths that differ.
///
/// A document that no longer parses is itself a failure: an edit that produced
/// invalid YAML must never be written back.
pub fn changed_paths_of_text(before: &str, after: &str) -> Result<BTreeSet<Vec<String>>, String> {
    let a: Value = serde_yaml_ng::from_str(before)
        .map_err(|e| format!("the document did not parse before the edit: {e}"))?;
    let b: Value = serde_yaml_ng::from_str(after)
        .map_err(|e| format!("the edit produced YAML that does not parse: {e}"))?;
    Ok(changed_paths(&a, &b))
}
