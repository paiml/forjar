//! Coverage tests for destroy.rs — cmd_destroy without --yes, cleanup_state_files via cmd_destroy.

use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

// ── cmd_destroy without --yes → immediate error ────────────────────

#[test]
fn destroy_no_yes_flag() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: destroy-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/forjar-destroy-test.txt
    content: hello
"#,
    );
    let result = super::destroy::cmd_destroy(&file, &state_dir, None, false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--yes"));
}

#[test]
fn destroy_no_yes_verbose() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n",
    );
    let result = super::destroy::cmd_destroy(&file, &state_dir, None, false, true);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--yes"));
}

// ── cmd_destroy with machine filter and no --yes ───────────────────

#[test]
fn destroy_machine_filter_no_yes() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n",
    );
    let result =
        super::destroy::cmd_destroy(&file, &state_dir, Some("web"), false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("--yes"));
}

// ── cmd_rollback via compute_rollback_changes directly ─────────────

#[test]
fn rollback_changes_all_same() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /tmp/x
    content: same
"#;
    let config = crate::core::parser::parse_config(yaml).unwrap();
    let changes = super::destroy::compute_rollback_changes(&config, &config, 1);
    assert!(changes.is_empty());
}

#[test]
fn rollback_changes_large_revision() {
    let prev: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(
        "version: '1.0'\nname: t\nmachines: {}\nresources:\n  a:\n    type: package\n",
    )
    .unwrap();
    let cur: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(
        "version: '1.0'\nname: t\nmachines: {}\nresources: {}\n",
    )
    .unwrap();
    let changes = super::destroy::compute_rollback_changes(&prev, &cur, 100);
    assert!(changes[0].contains("HEAD~100"));
}

// ── cleanup_succeeded_entries edge: invalid YAML in lock file ──────

#[test]
fn cleanup_succeeded_invalid_yaml() {
    let d = tempfile::tempdir().unwrap();
    let machine_dir = d.path().join("web1");
    std::fs::create_dir_all(&machine_dir).unwrap();
    std::fs::write(machine_dir.join("state.lock.yaml"), "invalid: [yaml: broken").unwrap();
    let mut succeeded = std::collections::HashMap::new();
    succeeded.insert("web1".to_string(), vec!["nginx".to_string()]);
    // Should not panic on invalid YAML — just skips
    super::destroy::cleanup_succeeded_entries(d.path(), &succeeded);
    // File still exists (wasn't modified)
    assert!(machine_dir.join("state.lock.yaml").exists());
}

// ── write_destroy_log_entry with no matching lock resource ─────────

#[test]
fn write_destroy_log_no_matching_resource() {
    let d = tempfile::tempdir().unwrap();
    let log_path = d.path().join("destroy-log.jsonl");
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: package").unwrap();
    let lock: crate::core::types::StateLock = serde_yaml_ng::from_str(
        "schema: '1'\nmachine: web\nhostname: h\ngenerated_at: t\ngenerator: g\nblake3_version: b\nresources:\n  other:\n    type: file\n    status: converged\n    hash: xyz\n",
    )
    .unwrap();
    let mut locks = std::collections::HashMap::new();
    locks.insert("web".to_string(), lock);
    // Resource "nginx" doesn't exist in lock → pre_hash will be empty
    super::destroy::write_destroy_log_entry(&log_path, "nginx", &resource, "web", &locks);
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("nginx"));
    assert!(content.contains("\"pre_hash\":\"\""));
}

#[test]
fn write_destroy_log_no_matching_machine() {
    let d = tempfile::tempdir().unwrap();
    let log_path = d.path().join("destroy-log.jsonl");
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: package").unwrap();
    let locks = std::collections::HashMap::new();
    // Machine "db" not in locks → pre_hash will be empty
    super::destroy::write_destroy_log_entry(&log_path, "pg", &resource, "db", &locks);
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("pg"));
}
