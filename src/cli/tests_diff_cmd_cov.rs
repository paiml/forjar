//! Coverage tests for diff_cmd.rs — cmd_diff, cmd_env_diff, cmd_env.

use std::path::Path;

fn write_machine_lock(state_dir: &Path, machine: &str, yaml: &str) {
    let dir = state_dir.join(machine);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("state.lock.yaml"), yaml).unwrap();
}

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

const LOCK_A: &str = r#"schema: "1"
machine: web
hostname: web
generated_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
blake3_version: "1.5"
resources:
  nginx:
    type: file
    status: converged
    hash: abc123
  app:
    type: file
    status: converged
    hash: def456
"#;

const LOCK_B: &str = r#"schema: "1"
machine: web
hostname: web
generated_at: "2026-03-08T11:00:00Z"
generator: "forjar 1.0.0"
blake3_version: "1.5"
resources:
  nginx:
    type: file
    status: converged
    hash: abc123
  app:
    type: file
    status: failed
    hash: ghi789
  redis:
    type: package
    status: converged
    hash: jkl012
"#;

// ── cmd_diff text mode ─────────────────────────────────────────────

#[test]
fn diff_text_with_changes() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    write_machine_lock(from.path(), "web", LOCK_A);
    write_machine_lock(to.path(), "web", LOCK_B);
    let result = super::diff_cmd::cmd_diff(from.path(), to.path(), None, None, false);
    assert!(result.is_ok());
}

#[test]
fn diff_json_with_changes() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    write_machine_lock(from.path(), "web", LOCK_A);
    write_machine_lock(to.path(), "web", LOCK_B);
    let result = super::diff_cmd::cmd_diff(from.path(), to.path(), None, None, true);
    assert!(result.is_ok());
}

#[test]
fn diff_no_changes() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    write_machine_lock(from.path(), "web", LOCK_A);
    write_machine_lock(to.path(), "web", LOCK_A);
    let result = super::diff_cmd::cmd_diff(from.path(), to.path(), None, None, false);
    assert!(result.is_ok());
}

#[test]
fn diff_machine_filter_match() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    write_machine_lock(from.path(), "web", LOCK_A);
    write_machine_lock(to.path(), "web", LOCK_B);
    let result =
        super::diff_cmd::cmd_diff(from.path(), to.path(), Some("web"), None, false);
    assert!(result.is_ok());
}

#[test]
fn diff_machine_filter_no_match() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    write_machine_lock(from.path(), "web", LOCK_A);
    write_machine_lock(to.path(), "web", LOCK_B);
    let result =
        super::diff_cmd::cmd_diff(from.path(), to.path(), Some("nonexistent"), None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no machines found"));
}

#[test]
fn diff_resource_filter() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    write_machine_lock(from.path(), "web", LOCK_A);
    write_machine_lock(to.path(), "web", LOCK_B);
    let result = super::diff_cmd::cmd_diff(
        from.path(),
        to.path(),
        None,
        Some("app"),
        false,
    );
    assert!(result.is_ok());
}

// ── cmd_diff with removed resources ────────────────────────────────

#[test]
fn diff_removed_resource() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    write_machine_lock(from.path(), "web", LOCK_B); // has redis
    write_machine_lock(to.path(), "web", LOCK_A); // no redis
    let result = super::diff_cmd::cmd_diff(from.path(), to.path(), None, None, false);
    assert!(result.is_ok());
}

#[test]
fn diff_removed_resource_json() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    write_machine_lock(from.path(), "web", LOCK_B);
    write_machine_lock(to.path(), "web", LOCK_A);
    let result = super::diff_cmd::cmd_diff(from.path(), to.path(), None, None, true);
    assert!(result.is_ok());
}

// ── cmd_diff from_only / to_only machines ──────────────────────────

#[test]
fn diff_from_has_extra_machine() {
    let from = tempfile::tempdir().unwrap();
    let to = tempfile::tempdir().unwrap();
    write_machine_lock(from.path(), "web", LOCK_A);
    write_machine_lock(from.path(), "db", LOCK_A);
    write_machine_lock(to.path(), "web", LOCK_A);
    let result = super::diff_cmd::cmd_diff(from.path(), to.path(), None, None, false);
    assert!(result.is_ok());
}

// ── cmd_env_diff ───────────────────────────────────────────────────

#[test]
fn env_diff_missing_first() {
    let state_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(state_dir.path().join("prod")).unwrap();
    let result =
        super::diff_cmd::cmd_env_diff("staging", "prod", state_dir.path(), false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn env_diff_missing_second() {
    let state_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(state_dir.path().join("staging")).unwrap();
    let result =
        super::diff_cmd::cmd_env_diff("staging", "prod", state_dir.path(), false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn env_diff_identical() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(&state_dir.path().join("staging"), "web", LOCK_A);
    write_machine_lock(&state_dir.path().join("prod"), "web", LOCK_A);
    let result =
        super::diff_cmd::cmd_env_diff("staging", "prod", state_dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn env_diff_with_drifted() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(&state_dir.path().join("staging"), "web", LOCK_A);
    write_machine_lock(&state_dir.path().join("prod"), "web", LOCK_B);
    let result =
        super::diff_cmd::cmd_env_diff("staging", "prod", state_dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn env_diff_json() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(&state_dir.path().join("staging"), "web", LOCK_A);
    write_machine_lock(&state_dir.path().join("prod"), "web", LOCK_B);
    let result =
        super::diff_cmd::cmd_env_diff("staging", "prod", state_dir.path(), true);
    assert!(result.is_ok());
}

// ── cmd_env ────────────────────────────────────────────────────────

#[test]
fn env_text_with_config() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: env-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/env-test.txt
    content: hello
"#,
    );
    let result = super::diff_cmd::cmd_env(&file, false);
    assert!(result.is_ok());
}

#[test]
fn env_json_with_config() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: env-json
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources: {}
"#,
    );
    let result = super::diff_cmd::cmd_env(&file, true);
    assert!(result.is_ok());
}

#[test]
fn env_text_missing_config() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("nonexistent.yaml");
    let result = super::diff_cmd::cmd_env(&file, false);
    assert!(result.is_ok());
}

#[test]
fn env_json_missing_config() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("nonexistent.yaml");
    let result = super::diff_cmd::cmd_env(&file, true);
    assert!(result.is_ok());
}

#[test]
fn env_text_invalid_config() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(&file, "invalid: yaml: [broken").unwrap();
    let result = super::diff_cmd::cmd_env(&file, false);
    assert!(result.is_ok());
}
