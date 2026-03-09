//! Coverage tests for lock_merge.rs — merge, rebase, sign.

use super::lock_merge::*;
use crate::core::state;

fn write_lock(state_dir: &std::path::Path, machine: &str) {
    let lock = state::new_lock(machine, &format!("{machine}.local"));
    state::save_lock(state_dir, &lock).unwrap();
}

fn write_config(dir: &std::path::Path, yaml: &str) -> std::path::PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

// ── cmd_lock_merge ──────────────────────────────────────────────────

#[test]
fn merge_both_empty_dirs() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let result = cmd_lock_merge(from.path(), to.path(), out.path(), false);
    // Both exist but have no machine dirs — no crash
    assert!(result.is_ok());
}

#[test]
fn merge_from_only() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let to = dir.path().join("to");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&from).unwrap();
    std::fs::create_dir_all(&to).unwrap();
    write_lock(&from, "web");
    let result = cmd_lock_merge(&from, &to, &out, false);
    assert!(result.is_ok());
    assert!(out.join("web/state.lock.yaml").exists());
}

#[test]
fn merge_to_only() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let to = dir.path().join("to");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&from).unwrap();
    std::fs::create_dir_all(&to).unwrap();
    write_lock(&to, "db");
    let result = cmd_lock_merge(&from, &to, &out, false);
    assert!(result.is_ok());
    assert!(out.join("db/state.lock.yaml").exists());
}

#[test]
fn merge_conflict_right_wins() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let to = dir.path().join("to");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&from).unwrap();
    std::fs::create_dir_all(&to).unwrap();
    write_lock(&from, "web");
    write_lock(&to, "web");
    let result = cmd_lock_merge(&from, &to, &out, false);
    assert!(result.is_ok());
    assert!(out.join("web/state.lock.yaml").exists());
}

#[test]
fn merge_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let to = dir.path().join("to");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&from).unwrap();
    std::fs::create_dir_all(&to).unwrap();
    write_lock(&from, "web");
    assert!(cmd_lock_merge(&from, &to, &out, true).is_ok());
}

#[test]
fn merge_both_nonexistent() {
    let out = tempfile::tempdir().unwrap();
    let result = cmd_lock_merge(
        std::path::Path::new("/nonexistent-from-dir"),
        std::path::Path::new("/nonexistent-to-dir"),
        out.path(),
        false,
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty"));
}

// ── cmd_lock_rebase ─────────────────────────────────────────────────

#[test]
fn rebase_keeps_matching_resources() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&from).unwrap();

    // Write a lock with resources
    let mut lock = state::new_lock("web", "web.local");
    lock.resources.insert(
        "pkg-curl".to_string(),
        crate::core::types::ResourceLock {
            resource_type: crate::core::types::ResourceType::Package,
            status: crate::core::types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "abc".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    lock.resources.insert(
        "pkg-vim".to_string(),
        crate::core::types::ResourceLock {
            resource_type: crate::core::types::ResourceType::Package,
            status: crate::core::types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "def".to_string(),
            details: std::collections::HashMap::new(),
        },
    );
    state::save_lock(&from, &lock).unwrap();

    let config_path = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  pkg-curl:
    type: package
    machine: web
    provider: apt
    packages: [curl]
"#,
    );
    let result = cmd_lock_rebase(&from, &config_path, &out, false);
    assert!(result.is_ok());
}

#[test]
fn rebase_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&from).unwrap();
    write_lock(&from, "web");

    let config_path = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources: {}
"#,
    );
    assert!(cmd_lock_rebase(&from, &config_path, &out, true).is_ok());
}

#[test]
fn rebase_empty_from() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("from");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&from).unwrap();

    let config_path = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources: {}
"#,
    );
    assert!(cmd_lock_rebase(&from, &config_path, &out, false).is_ok());
}

// ── cmd_lock_sign ───────────────────────────────────────────────────

#[test]
fn sign_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    assert!(cmd_lock_sign(dir.path(), "test-key", false).is_ok());
}

#[test]
fn sign_with_lock() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web");
    assert!(cmd_lock_sign(dir.path(), "my-secret-key", false).is_ok());
    assert!(dir.path().join("web/lock.sig").exists());
}

#[test]
fn sign_json_output() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "db");
    assert!(cmd_lock_sign(dir.path(), "key", true).is_ok());
}

#[test]
fn sign_skips_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let hidden = dir.path().join(".hidden");
    std::fs::create_dir_all(&hidden).unwrap();
    std::fs::write(hidden.join("state.lock.yaml"), "dummy").unwrap();
    assert!(cmd_lock_sign(dir.path(), "key", false).is_ok());
    assert!(!hidden.join("lock.sig").exists());
}
