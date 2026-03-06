//! Coverage tests for cli/destroy.rs — cleanup_succeeded_entries, write_destroy_log_entry.

use crate::core::types;
use std::collections::HashMap;

const LOCK_WEB1: &str = r#"schema: "1"
machine: web1
hostname: web1
generated_at: "2025-01-01T00:00:00Z"
generator: forjar-test
blake3_version: "1.0"
resources:
  nginx:
    type: package
    status: converged
    hash: abc123
    applied_at: "2025-01-01T00:00:00Z"
    duration_seconds: 2.5
  config:
    type: file
    status: converged
    hash: def456
    applied_at: "2025-01-01T00:01:00Z"
    duration_seconds: 0.5
"#;

fn setup_state(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("web1")).unwrap();
    std::fs::write(dir.join("web1/state.lock.yaml"), LOCK_WEB1).unwrap();
}

// ── cleanup_succeeded_entries ──

#[test]
fn cleanup_empty() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let succeeded = HashMap::new();
    super::destroy::cleanup_succeeded_entries(d.path(), &succeeded);
    // Lock should be unchanged
    let content = std::fs::read_to_string(d.path().join("web1/state.lock.yaml")).unwrap();
    assert!(content.contains("nginx"));
}

#[test]
fn cleanup_one_resource() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let mut succeeded = HashMap::new();
    succeeded.insert("web1".to_string(), vec!["nginx".to_string()]);
    super::destroy::cleanup_succeeded_entries(d.path(), &succeeded);
    let content = std::fs::read_to_string(d.path().join("web1/state.lock.yaml")).unwrap();
    assert!(!content.contains("nginx"));
    assert!(content.contains("config"));
}

#[test]
fn cleanup_all_resources_removes_file() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let mut succeeded = HashMap::new();
    succeeded.insert(
        "web1".to_string(),
        vec!["nginx".to_string(), "config".to_string()],
    );
    super::destroy::cleanup_succeeded_entries(d.path(), &succeeded);
    assert!(!d.path().join("web1/state.lock.yaml").exists());
}

#[test]
fn cleanup_missing_machine_is_noop() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let mut succeeded = HashMap::new();
    succeeded.insert("nonexistent".to_string(), vec!["nginx".to_string()]);
    super::destroy::cleanup_succeeded_entries(d.path(), &succeeded);
    // web1 lock still intact
    assert!(d.path().join("web1/state.lock.yaml").exists());
}

// ── write_destroy_log_entry ──

fn make_resource() -> types::Resource {
    serde_yaml_ng::from_str("type: package").unwrap()
}

fn make_resource_with_content() -> types::Resource {
    serde_yaml_ng::from_str(
        "type: file\npath: /etc/app.conf\ncontent: \"hello world\"",
    )
    .unwrap()
}

#[test]
fn write_destroy_log_basic() {
    let d = tempfile::tempdir().unwrap();
    let log_path = d.path().join("destroy-log.jsonl");
    let resource = make_resource();
    let locks = HashMap::new();
    super::destroy::write_destroy_log_entry(&log_path, "nginx", &resource, "web1", &locks);
    assert!(log_path.exists());
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("nginx"));
    assert!(content.contains("web1"));
}

#[test]
fn write_destroy_log_with_hash() {
    let d = tempfile::tempdir().unwrap();
    let log_path = d.path().join("destroy-log.jsonl");
    let resource = make_resource();
    let lock: types::StateLock = serde_yaml_ng::from_str(LOCK_WEB1).unwrap();
    let mut locks = HashMap::new();
    locks.insert("web1".to_string(), lock);
    super::destroy::write_destroy_log_entry(&log_path, "nginx", &resource, "web1", &locks);
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("abc123"));
}

#[test]
fn write_destroy_log_with_content() {
    let d = tempfile::tempdir().unwrap();
    let log_path = d.path().join("destroy-log.jsonl");
    let resource = make_resource_with_content();
    let locks = HashMap::new();
    super::destroy::write_destroy_log_entry(&log_path, "app-conf", &resource, "web1", &locks);
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("app-conf"));
    // reliable_recreate should be true for resources with content
    assert!(content.contains("\"reliable_recreate\":true"));
}

#[test]
fn write_destroy_log_appends() {
    let d = tempfile::tempdir().unwrap();
    let log_path = d.path().join("destroy-log.jsonl");
    let resource = make_resource();
    let locks = HashMap::new();
    super::destroy::write_destroy_log_entry(&log_path, "nginx", &resource, "web1", &locks);
    super::destroy::write_destroy_log_entry(&log_path, "mysql", &resource, "db1", &locks);
    let content = std::fs::read_to_string(&log_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
}

// ── compute_rollback_changes (additional coverage) ──

#[test]
fn rollback_changes_added_resource() {
    let prev: types::ForjarConfig = serde_yaml_ng::from_str(
        "version: '1.0'\nname: t\nmachines: {}\nresources:\n  nginx:\n    type: package\n  mysql:\n    type: package\n",
    ).unwrap();
    let cur: types::ForjarConfig = serde_yaml_ng::from_str(
        "version: '1.0'\nname: t\nmachines: {}\nresources:\n  nginx:\n    type: package\n",
    ).unwrap();
    let changes = super::destroy::compute_rollback_changes(&prev, &cur, 1);
    assert!(changes.iter().any(|c| c.contains("mysql") && c.contains("re-added")));
}

#[test]
fn rollback_changes_removed_resource() {
    let prev: types::ForjarConfig = serde_yaml_ng::from_str(
        "version: '1.0'\nname: t\nmachines: {}\nresources:\n  nginx:\n    type: package\n",
    ).unwrap();
    let cur: types::ForjarConfig = serde_yaml_ng::from_str(
        "version: '1.0'\nname: t\nmachines: {}\nresources:\n  nginx:\n    type: package\n  redis:\n    type: package\n",
    ).unwrap();
    let changes = super::destroy::compute_rollback_changes(&prev, &cur, 1);
    assert!(changes.iter().any(|c| c.contains("redis") && c.contains("remain")));
}

#[test]
fn rollback_changes_modified_resource() {
    let prev: types::ForjarConfig = serde_yaml_ng::from_str(
        "version: '1.0'\nname: t\nmachines: {}\nresources:\n  nginx:\n    type: package\n    version: '1.0'\n",
    ).unwrap();
    let cur: types::ForjarConfig = serde_yaml_ng::from_str(
        "version: '1.0'\nname: t\nmachines: {}\nresources:\n  nginx:\n    type: package\n    version: '2.0'\n",
    ).unwrap();
    let changes = super::destroy::compute_rollback_changes(&prev, &cur, 2);
    assert!(changes.iter().any(|c| c.contains("nginx") && c.contains("modified")));
}
