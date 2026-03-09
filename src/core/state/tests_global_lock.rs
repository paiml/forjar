//! Coverage tests for core/state/mod.rs — global lock, outputs, apply reports.

use super::*;
use crate::core::types;

// ── new_global_lock ──────────────────────────────────────────────────

#[test]
fn new_global_lock_fields() {
    let lock = new_global_lock("test-project");
    assert_eq!(lock.schema, "1.0");
    assert_eq!(lock.name, "test-project");
    assert!(lock.machines.is_empty());
    assert!(lock.outputs.is_empty());
    assert!(lock.generator.contains("forjar"));
}

// ── save/load global lock roundtrip ──────────────────────────────────

#[test]
fn global_lock_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let lock = new_global_lock("roundtrip");
    save_global_lock(dir.path(), &lock).unwrap();
    let loaded = load_global_lock(dir.path()).unwrap();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.name, "roundtrip");
}

#[test]
fn load_global_lock_missing() {
    let dir = tempfile::tempdir().unwrap();
    let result = load_global_lock(dir.path()).unwrap();
    assert!(result.is_none());
}

// ── update_global_lock ───────────────────────────────────────────────

#[test]
fn update_global_lock_creates_new() {
    let dir = tempfile::tempdir().unwrap();
    let results = vec![("web".to_string(), 5, 4, 1), ("db".to_string(), 3, 3, 0)];
    update_global_lock(dir.path(), "test-config", &results).unwrap();
    let lock = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(lock.name, "test-config");
    assert_eq!(lock.machines.len(), 2);
    assert_eq!(lock.machines["web"].resources, 5);
    assert_eq!(lock.machines["web"].converged, 4);
    assert_eq!(lock.machines["web"].failed, 1);
    assert_eq!(lock.machines["db"].resources, 3);
}

#[test]
fn update_global_lock_updates_existing() {
    let dir = tempfile::tempdir().unwrap();
    let results1 = vec![("web".to_string(), 3, 2, 1)];
    update_global_lock(dir.path(), "proj", &results1).unwrap();

    let results2 = vec![("web".to_string(), 5, 5, 0)];
    update_global_lock(dir.path(), "proj", &results2).unwrap();

    let lock = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(lock.machines["web"].resources, 5);
    assert_eq!(lock.machines["web"].converged, 5);
}

// ── persist_outputs ──────────────────────────────────────────────────

#[test]
fn persist_outputs_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("api_url".to_string(), "https://api.example.com".to_string());
    outputs.insert("db_host".to_string(), "db.local".to_string());

    persist_outputs(dir.path(), "test", &outputs).unwrap();
    let lock = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(lock.outputs.len(), 2);
    assert_eq!(lock.outputs["api_url"], "https://api.example.com");
}

#[test]
fn persist_outputs_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let mut out1 = indexmap::IndexMap::new();
    out1.insert("key".to_string(), "value1".to_string());
    persist_outputs(dir.path(), "test", &out1).unwrap();

    let mut out2 = indexmap::IndexMap::new();
    out2.insert("key".to_string(), "value2".to_string());
    persist_outputs(dir.path(), "test", &out2).unwrap();

    let lock = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(lock.outputs["key"], "value2");
}

// ── new_lock ─────────────────────────────────────────────────────────

#[test]
fn new_lock_fields() {
    let lock = new_lock("web01", "web01.example.com");
    assert_eq!(lock.machine, "web01");
    assert_eq!(lock.hostname, "web01.example.com");
    assert_eq!(lock.schema, "1.0");
    assert!(lock.resources.is_empty());
    assert!(lock.generator.contains("forjar"));
}

// ── save/load apply report ───────────────────────────────────────────

#[test]
fn apply_report_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let report = types::ApplyResult {
        machine: "web01".to_string(),
        resources_converged: 4,
        resources_unchanged: 0,
        resources_failed: 1,
        total_duration: std::time::Duration::from_millis(2500),
        resource_reports: vec![],
    };
    save_apply_report(dir.path(), &report).unwrap();
    let loaded = load_apply_report(dir.path(), "web01").unwrap();
    assert!(loaded.is_some());
    let content = loaded.unwrap();
    assert!(content.contains("web01"));
}

#[test]
fn apply_report_missing() {
    let dir = tempfile::tempdir().unwrap();
    let result = load_apply_report(dir.path(), "nonexistent").unwrap();
    assert!(result.is_none());
}

// ── process lock ─────────────────────────────────────────────────────

#[test]
fn acquire_release_lock() {
    let dir = tempfile::tempdir().unwrap();
    acquire_process_lock(dir.path()).unwrap();
    assert!(process_lock_path(dir.path()).exists());
    release_process_lock(dir.path());
    assert!(!process_lock_path(dir.path()).exists());
}

#[test]
fn force_unlock_no_lock() {
    let dir = tempfile::tempdir().unwrap();
    assert!(force_unlock(dir.path()).is_ok());
}

#[test]
fn force_unlock_existing() {
    let dir = tempfile::tempdir().unwrap();
    acquire_process_lock(dir.path()).unwrap();
    force_unlock(dir.path()).unwrap();
    assert!(!process_lock_path(dir.path()).exists());
}

#[test]
fn parse_lock_pid_valid() {
    let content = "pid: 12345\nstarted_at: 2026-01-01T00:00:00Z\n";
    assert_eq!(parse_lock_pid(content), Some(12345));
}

#[test]
fn parse_lock_pid_missing() {
    let content = "started_at: 2026-01-01T00:00:00Z\n";
    assert_eq!(parse_lock_pid(content), None);
}

#[test]
fn parse_lock_pid_invalid() {
    let content = "pid: notanumber\n";
    assert_eq!(parse_lock_pid(content), None);
}

// ── global_lock_path / lock_file_path ────────────────────────────────

#[test]
fn global_lock_path_test() {
    let p = global_lock_path(std::path::Path::new("/var/state"));
    assert_eq!(p, std::path::PathBuf::from("/var/state/forjar.lock.yaml"));
}

#[test]
fn lock_file_path_test() {
    let p = lock_file_path(std::path::Path::new("/var/state"), "web01");
    assert_eq!(
        p,
        std::path::PathBuf::from("/var/state/web01/state.lock.yaml")
    );
}
