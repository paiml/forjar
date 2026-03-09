//! Coverage tests for undo_helpers.rs — cmd_undo_destroy dry-run paths.

use super::undo_helpers::*;
use crate::core::types;

fn make_log_entry(resource_id: &str, machine: &str, reliable: bool) -> types::DestroyLogEntry {
    types::DestroyLogEntry {
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        machine: machine.to_string(),
        resource_id: resource_id.to_string(),
        resource_type: "file".to_string(),
        pre_hash: "blake3:abc123".to_string(),
        generation: 1,
        config_fragment: Some(
            "type: file\nmachine: m\npath: /tmp/test\ncontent: hello\n".to_string(),
        ),
        reliable_recreate: reliable,
    }
}

// ── cmd_undo_destroy dry_run ────────────────────────────────────────

#[test]
fn undo_destroy_dry_run_reliable() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();
    let entry = make_log_entry("f1", "m1", true);
    let line = entry.to_jsonl().unwrap();
    std::fs::write(state_dir.join("destroy-log.jsonl"), &line).unwrap();

    let result = cmd_undo_destroy(state_dir, None, false, true);
    assert!(result.is_ok());
}

#[test]
fn undo_destroy_dry_run_unreliable_no_force() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();
    let entry = make_log_entry("f1", "m1", false);
    let line = entry.to_jsonl().unwrap();
    std::fs::write(state_dir.join("destroy-log.jsonl"), &line).unwrap();

    let result = cmd_undo_destroy(state_dir, None, false, true);
    assert!(result.is_ok());
}

#[test]
fn undo_destroy_dry_run_force() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();
    let entry = make_log_entry("f1", "m1", false);
    let line = entry.to_jsonl().unwrap();
    std::fs::write(state_dir.join("destroy-log.jsonl"), &line).unwrap();

    let result = cmd_undo_destroy(state_dir, None, true, true);
    assert!(result.is_ok());
}

#[test]
fn undo_destroy_dry_run_mixed_entries() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();
    let e1 = make_log_entry("f1", "m1", true);
    let e2 = make_log_entry("f2", "m1", false);
    let lines = format!("{}\n{}", e1.to_jsonl().unwrap(), e2.to_jsonl().unwrap());
    std::fs::write(state_dir.join("destroy-log.jsonl"), &lines).unwrap();

    let result = cmd_undo_destroy(state_dir, None, false, true);
    assert!(result.is_ok());
}

#[test]
fn undo_destroy_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();
    let e1 = make_log_entry("f1", "m1", true);
    let e2 = make_log_entry("f2", "m2", true);
    let lines = format!("{}\n{}", e1.to_jsonl().unwrap(), e2.to_jsonl().unwrap());
    std::fs::write(state_dir.join("destroy-log.jsonl"), &lines).unwrap();

    // Filter to m1 only — dry run
    let result = cmd_undo_destroy(state_dir, Some("m1"), false, true);
    assert!(result.is_ok());
}

#[test]
fn undo_destroy_machine_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path();
    let e1 = make_log_entry("f1", "m1", true);
    let line = e1.to_jsonl().unwrap();
    std::fs::write(state_dir.join("destroy-log.jsonl"), &line).unwrap();

    let result = cmd_undo_destroy(state_dir, Some("nonexistent"), false, true);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no matching entries"));
}

#[test]
fn undo_destroy_no_log_file() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_undo_destroy(dir.path(), None, false, true);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no destroy-log.jsonl"));
}

#[test]
fn undo_destroy_empty_log() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("destroy-log.jsonl"), "").unwrap();
    let result = cmd_undo_destroy(dir.path(), None, false, true);
    assert!(result.is_err());
}

// ── DestroyLogEntry serde ───────────────────────────────────────────

#[test]
fn destroy_log_entry_roundtrip() {
    let entry = make_log_entry("web-config", "prod-1", true);
    let json = entry.to_jsonl().unwrap();
    let parsed = types::DestroyLogEntry::from_jsonl(&json).unwrap();
    assert_eq!(parsed.resource_id, "web-config");
    assert_eq!(parsed.machine, "prod-1");
    assert!(parsed.reliable_recreate);
    assert!(parsed.config_fragment.is_some());
}

#[test]
fn destroy_log_entry_no_fragment() {
    let entry = types::DestroyLogEntry {
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        machine: "m".to_string(),
        resource_id: "r".to_string(),
        resource_type: "package".to_string(),
        pre_hash: "blake3:def".to_string(),
        generation: 2,
        config_fragment: None,
        reliable_recreate: false,
    };
    let json = entry.to_jsonl().unwrap();
    let parsed = types::DestroyLogEntry::from_jsonl(&json).unwrap();
    assert!(parsed.config_fragment.is_none());
    assert!(!parsed.reliable_recreate);
}

#[test]
fn destroy_log_entry_invalid_json() {
    let result = types::DestroyLogEntry::from_jsonl("not json");
    assert!(result.is_err());
}
