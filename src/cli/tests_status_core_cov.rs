//! Coverage tests for status_core.rs — cmd_status text/json/summary modes.

use std::path::Path;

fn write_global_lock(state_dir: &Path, yaml: &str) {
    std::fs::write(state_dir.join("forjar.lock.yaml"), yaml).unwrap();
}

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

const GLOBAL_LOCK: &str = r#"schema: "1"
name: myproject
last_apply: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
machines:
  web:
    resources: 2
    converged: 1
    failed: 1
    last_apply: "2026-03-08T10:00:00Z"
"#;

const MACHINE_LOCK_WEB: &str = r#"schema: "1"
machine: web
hostname: webhost
generated_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
blake3_version: "1.5"
resources:
  nginx-cfg:
    type: file
    status: converged
    hash: abc123
    applied_at: "2026-03-08T10:00:00Z"
    duration_seconds: 1.5
  app-cfg:
    type: file
    status: failed
    hash: def456
"#;

const MACHINE_LOCK_DB: &str = r#"schema: "1"
machine: db
hostname: dbhost
generated_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
blake3_version: "1.5"
resources:
  pg-cfg:
    type: file
    status: converged
    hash: ghi789
"#;

// ── empty state ─────────────────────────────────────────────────────

#[test]
fn status_empty_state_text() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::status_core::cmd_status(state_dir.path(), None, false, None, false);
    assert!(result.is_ok());
}

#[test]
fn status_empty_state_json() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::status_core::cmd_status(state_dir.path(), None, true, None, false);
    assert!(result.is_ok());
}

#[test]
fn status_empty_state_summary() {
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::status_core::cmd_status(state_dir.path(), None, false, None, true);
    assert!(result.is_ok());
}

// ── with machine lock ───────────────────────────────────────────────

#[test]
fn status_with_machine_text() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let result = super::status_core::cmd_status(state_dir.path(), None, false, None, false);
    assert!(result.is_ok());
}

#[test]
fn status_with_machine_json() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let result = super::status_core::cmd_status(state_dir.path(), None, true, None, false);
    assert!(result.is_ok());
}

#[test]
fn status_with_machine_summary() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let result = super::status_core::cmd_status(state_dir.path(), None, false, None, true);
    assert!(result.is_ok());
}

// ── with global lock ────────────────────────────────────────────────

#[test]
fn status_with_global_lock_text() {
    let state_dir = tempfile::tempdir().unwrap();
    write_global_lock(state_dir.path(), GLOBAL_LOCK);
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let result = super::status_core::cmd_status(state_dir.path(), None, false, None, false);
    assert!(result.is_ok());
}

#[test]
fn status_with_global_lock_json() {
    let state_dir = tempfile::tempdir().unwrap();
    write_global_lock(state_dir.path(), GLOBAL_LOCK);
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let result = super::status_core::cmd_status(state_dir.path(), None, true, None, false);
    assert!(result.is_ok());
}

#[test]
fn status_with_global_lock_summary() {
    let state_dir = tempfile::tempdir().unwrap();
    write_global_lock(state_dir.path(), GLOBAL_LOCK);
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let result = super::status_core::cmd_status(state_dir.path(), None, false, None, true);
    assert!(result.is_ok());
}

// ── machine filter ──────────────────────────────────────────────────

#[test]
fn status_machine_filter_match() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    write_machine_lock(state_dir.path(), "db", MACHINE_LOCK_DB);
    let result =
        super::status_core::cmd_status(state_dir.path(), Some("web"), false, None, false);
    assert!(result.is_ok());
}

#[test]
fn status_machine_filter_no_match() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let result = super::status_core::cmd_status(
        state_dir.path(),
        Some("nonexistent"),
        false,
        None,
        false,
    );
    assert!(result.is_ok());
}

// ── config enrichment ───────────────────────────────────────────────

const ENRICHMENT_CONFIG: &str = r#"
version: "1.0"
name: myproject
machines:
  web:
    hostname: webhost
    addr: 127.0.0.1
resources:
  nginx-cfg:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    content: "worker_processes 1;"
    tags: [web, proxy]
    resource_group: webservers
  app-cfg:
    type: file
    machine: web
    path: /tmp/app.conf
    content: "key=val"
    tags: [app]
    depends_on: [nginx-cfg]
"#;

#[test]
fn status_with_config_enrichment_text() {
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_dir = tempfile::tempdir().unwrap();
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let config_file = write_config(cfg_dir.path(), ENRICHMENT_CONFIG);
    let result = super::status_core::cmd_status(
        state_dir.path(),
        None,
        false,
        Some(&config_file),
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn status_with_config_enrichment_json() {
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_dir = tempfile::tempdir().unwrap();
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let config_file = write_config(cfg_dir.path(), ENRICHMENT_CONFIG);
    let result = super::status_core::cmd_status(
        state_dir.path(),
        None,
        true,
        Some(&config_file),
        false,
    );
    assert!(result.is_ok());
}

// ── multi-machine ───────────────────────────────────────────────────

#[test]
fn status_multi_machine_text() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    write_machine_lock(state_dir.path(), "db", MACHINE_LOCK_DB);
    let result = super::status_core::cmd_status(state_dir.path(), None, false, None, false);
    assert!(result.is_ok());
}

#[test]
fn status_multi_machine_json_with_global() {
    let state_dir = tempfile::tempdir().unwrap();
    write_global_lock(state_dir.path(), GLOBAL_LOCK);
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    write_machine_lock(state_dir.path(), "db", MACHINE_LOCK_DB);
    let result = super::status_core::cmd_status(state_dir.path(), None, true, None, false);
    assert!(result.is_ok());
}

// ── resource with details ───────────────────────────────────────────

#[test]
fn status_resource_with_details_json() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(
        state_dir.path(),
        "web",
        r#"schema: "1"
machine: web
hostname: webhost
generated_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
blake3_version: "1.5"
resources:
  nginx:
    type: file
    status: drifted
    hash: abc123
    details:
      expected: "worker_processes 1;"
      actual: "worker_processes 4;"
"#,
    );
    let result = super::status_core::cmd_status(state_dir.path(), None, true, None, false);
    assert!(result.is_ok());
}

// ── all statuses in summary ─────────────────────────────────────────

#[test]
fn status_summary_all_statuses() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(
        state_dir.path(),
        "web",
        r#"schema: "1"
machine: web
hostname: webhost
generated_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
blake3_version: "1.5"
resources:
  a:
    type: file
    status: converged
    hash: a1
  b:
    type: file
    status: failed
    hash: b1
  c:
    type: file
    status: drifted
    hash: c1
  d:
    type: file
    status: unknown
    hash: d1
"#,
    );
    let result = super::status_core::cmd_status(state_dir.path(), None, false, None, true);
    assert!(result.is_ok());
}

// ── summary without global lock ─────────────────────────────────────

#[test]
fn status_summary_no_global() {
    let state_dir = tempfile::tempdir().unwrap();
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let result = super::status_core::cmd_status(state_dir.path(), None, false, None, true);
    assert!(result.is_ok());
}

// ── config with no matching resources ───────────────────────────────

#[test]
fn status_config_no_matching_resources() {
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_dir = tempfile::tempdir().unwrap();
    write_machine_lock(state_dir.path(), "web", MACHINE_LOCK_WEB);
    let config_file = write_config(
        cfg_dir.path(),
        r#"
version: "1.0"
name: other
machines:
  web:
    hostname: webhost
    addr: 127.0.0.1
resources:
  unrelated:
    type: file
    machine: web
    path: /tmp/x
    content: "x"
"#,
    );
    let result = super::status_core::cmd_status(
        state_dir.path(),
        None,
        false,
        Some(&config_file),
        false,
    );
    assert!(result.is_ok());
}
