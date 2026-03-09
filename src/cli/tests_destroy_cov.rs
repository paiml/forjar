//! Coverage tests for destroy.rs — compute_rollback_changes (pure function).

use super::destroy::*;
use crate::core::types;

fn minimal_config(resources: Vec<(&str, &str)>) -> types::ForjarConfig {
    let yaml = if resources.is_empty() {
        "version: '1.0'\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n".to_string()
    } else {
        let mut y = "version: '1.0'\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n".to_string();
        for (id, content) in resources {
            y.push_str(&format!(
                "  {id}:\n    type: file\n    machine: m\n    path: /tmp/{id}\n    content: \"{content}\"\n"
            ));
        }
        y
    };
    crate::core::parser::parse_config(&yaml).unwrap()
}

// ── compute_rollback_changes ────────────────────────────────────────

#[test]
fn rollback_no_changes() {
    let config = minimal_config(vec![("f1", "hello")]);
    let changes = compute_rollback_changes(&config, &config, 1);
    assert!(changes.is_empty());
}

#[test]
fn rollback_resource_removed_in_current() {
    let previous = minimal_config(vec![("f1", "hello"), ("f2", "world")]);
    let current = minimal_config(vec![("f1", "hello")]);
    let changes = compute_rollback_changes(&previous, &current, 2);
    assert_eq!(changes.len(), 1);
    assert!(changes[0].contains("f2"));
    assert!(changes[0].contains("re-added"));
    assert!(changes[0].contains("HEAD~2"));
}

#[test]
fn rollback_resource_added_in_current() {
    let previous = minimal_config(vec![("f1", "hello")]);
    let current = minimal_config(vec![("f1", "hello"), ("f2", "new")]);
    let changes = compute_rollback_changes(&previous, &current, 1);
    assert_eq!(changes.len(), 1);
    assert!(changes[0].contains("f2"));
    assert!(changes[0].contains("will remain"));
}

#[test]
fn rollback_resource_modified() {
    let previous = minimal_config(vec![("f1", "old-content")]);
    let current = minimal_config(vec![("f1", "new-content")]);
    let changes = compute_rollback_changes(&previous, &current, 3);
    assert_eq!(changes.len(), 1);
    assert!(changes[0].contains("f1"));
    assert!(changes[0].contains("modified"));
}

#[test]
fn rollback_mixed_changes() {
    let previous = minimal_config(vec![("f1", "same"), ("f2", "old"), ("f3", "removed")]);
    let current = minimal_config(vec![("f1", "same"), ("f2", "changed"), ("f4", "added")]);
    let changes = compute_rollback_changes(&previous, &current, 1);
    // f1: same → no change
    // f2: modified
    // f3: removed in current → will be re-added
    // f4: added in current → will remain
    assert_eq!(changes.len(), 3);
    let joined = changes.join("\n");
    assert!(joined.contains("f2") && joined.contains("modified"));
    assert!(joined.contains("f3") && joined.contains("re-added"));
    assert!(joined.contains("f4") && joined.contains("will remain"));
}

#[test]
fn rollback_both_empty() {
    let previous = minimal_config(vec![]);
    let current = minimal_config(vec![]);
    let changes = compute_rollback_changes(&previous, &current, 1);
    assert!(changes.is_empty());
}

#[test]
fn rollback_revision_number_in_output() {
    let previous = minimal_config(vec![("gone", "x")]);
    let current = minimal_config(vec![]);
    let changes = compute_rollback_changes(&previous, &current, 5);
    assert!(changes[0].contains("HEAD~5"));
}
