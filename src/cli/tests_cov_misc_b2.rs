//! Coverage tests for cli/dispatch_misc_b.rs — contracts, logs, oci-pack, query functions.

use super::dispatch_misc_b::*;
use super::query_format::{cmd_query_churn, cmd_query_drift};
use crate::core::store::db::{self, FtsResult};

fn setup_state_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("web1")).unwrap();
    std::fs::write(
        dir.path().join("web1/state.lock.yaml"),
        "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 2.5\n",
    ).unwrap();
    dir
}

fn setup_db() -> rusqlite::Connection {
    let conn = db::open_state_db(std::path::Path::new(":memory:")).unwrap();
    conn.execute(
        "INSERT INTO generations (generation_num, run_id, config_hash, created_at) VALUES (1, 'run-1', 'hash-1', '2026-03-06')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO machines (name, first_seen, last_seen) VALUES ('web1', '2026-03-06', '2026-03-06')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO resources (resource_id, machine_id, generation_id, resource_type, status, path, duration_secs, reversibility, applied_at) \
         VALUES ('nginx-pkg', 1, 1, 'package', 'converged', '/usr/bin/nginx', 1.5, 'reversible', '2026-03-06')",
        [],
    ).unwrap();
    conn.execute("INSERT INTO resources_fts(resources_fts) VALUES('rebuild')", []).unwrap();
    conn
}

fn sample_fts() -> Vec<FtsResult> {
    vec![FtsResult {
        resource_id: "nginx-pkg".into(),
        resource_type: "package".into(),
        status: "converged".into(),
        path: Some("/usr/bin/nginx".into()),
        rank: -1.5,
    }]
}

// ── cmd_contracts (now in contracts.rs) ──

fn write_contracts_config() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("forjar.yaml"), CONTRACTS_YAML).unwrap();
    dir
}

const CONTRACTS_YAML: &str = r#"
version: "1.0"
name: contracts-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  web-pkg:
    type: package
    machine: m
    provider: apt
    packages: [nginx]
  web-svc:
    type: service
    machine: m
    name: nginx
    depends_on: [web-pkg]
"#;

#[test]
fn contracts_coverage_text_real() {
    let dir = write_contracts_config();
    let file = dir.path().join("forjar.yaml");
    let r = super::contracts::cmd_contracts(true, &file, false);
    assert!(r.is_ok());
}

#[test]
fn contracts_coverage_json_real() {
    let dir = write_contracts_config();
    let file = dir.path().join("forjar.yaml");
    let r = super::contracts::cmd_contracts(true, &file, true);
    assert!(r.is_ok());
}

#[test]
fn contracts_no_coverage_flag_still_works() {
    let dir = write_contracts_config();
    let file = dir.path().join("forjar.yaml");
    let r = super::contracts::cmd_contracts(false, &file, false);
    assert!(r.is_ok());
}

#[test]
fn contracts_empty_config() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("f.yaml");
    std::fs::write(&file, "").unwrap();
    let r = super::contracts::cmd_contracts(true, &file, false);
    assert!(r.is_ok());
}

#[test]
fn contracts_json_empty_config() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("f.yaml");
    std::fs::write(&file, "").unwrap();
    let r = super::contracts::cmd_contracts(true, &file, true);
    assert!(r.is_ok());
}

#[test]
fn contracts_with_task_resource_l1() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  setup:\n    type: task\n    machine: m\n    command: echo setup\n";
    std::fs::write(dir.path().join("forjar.yaml"), yaml).unwrap();
    let file = dir.path().join("forjar.yaml");
    let r = super::contracts::cmd_contracts(true, &file, false);
    assert!(r.is_ok());
}

#[test]
fn contracts_without_detail_flag() {
    let dir = write_contracts_config();
    let file = dir.path().join("forjar.yaml");
    let r = super::contracts::cmd_contracts(false, &file, false);
    assert!(r.is_ok());
}

// ── cmd_logs (delegated to cli/logs.rs) ──

#[test]
fn logs_gc_mode() {
    let dir = tempfile::tempdir().unwrap();
    let r = super::logs::cmd_logs_gc(dir.path(), false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_follow_mode() {
    let dir = tempfile::tempdir().unwrap();
    let r = super::logs::cmd_logs_follow(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn logs_default_all() {
    let dir = tempfile::tempdir().unwrap();
    let r = super::logs::cmd_logs(dir.path(), None, None, None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_with_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    let r = super::logs::cmd_logs(dir.path(), Some("web1"), None, None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_with_run_filter() {
    let dir = tempfile::tempdir().unwrap();
    let r = super::logs::cmd_logs(dir.path(), None, Some("run-001"), None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_failures_only() {
    let dir = tempfile::tempdir().unwrap();
    let r = super::logs::cmd_logs(dir.path(), Some("web1"), Some("run-001"), None, true, false, false, false);
    assert!(r.is_ok());
}

// ── cmd_logs JSON mode ──

#[test]
fn logs_json_default() {
    let dir = tempfile::tempdir().unwrap();
    let r = super::logs::cmd_logs(dir.path(), None, None, None, false, false, false, true);
    assert!(r.is_ok());
}

#[test]
fn logs_json_gc() {
    let dir = tempfile::tempdir().unwrap();
    let r = super::logs::cmd_logs_gc(dir.path(), false, false, true);
    assert!(r.is_ok());
}

#[test]
fn logs_json_follow() {
    let dir = tempfile::tempdir().unwrap();
    let r = super::logs::cmd_logs_follow(dir.path(), true);
    assert!(r.is_ok());
}

#[test]
fn logs_json_with_filters() {
    let dir = tempfile::tempdir().unwrap();
    let r = super::logs::cmd_logs(dir.path(), Some("web1"), Some("run-001"), None, true, false, false, true);
    assert!(r.is_ok());
}

// ── cmd_oci_pack ──

#[test]
fn oci_pack_text() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let output = out.path().join("image.tar");
    let r = cmd_oci_pack(src.path(), "v1.0", &output, false);
    assert!(r.is_ok());
}

#[test]
fn oci_pack_json() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let output = out.path().join("image.tar");
    let r = cmd_oci_pack(src.path(), "latest", &output, true);
    assert!(r.is_ok());
}

#[test]
fn oci_pack_missing_dir() {
    let out = tempfile::tempdir().unwrap();
    let output = out.path().join("image.tar");
    let r = cmd_oci_pack(std::path::Path::new("/nonexistent/dir"), "v1", &output, false);
    assert!(r.is_err());
}

// ── open_state_conn ──

#[test]
fn open_state_conn_valid_dir() {
    let dir = setup_state_dir();
    let conn = open_state_conn(dir.path());
    assert!(conn.is_ok());
}

#[test]
fn open_state_conn_missing_dir() {
    let conn = open_state_conn(std::path::Path::new("/nonexistent/state"));
    assert!(conn.is_ok()); // falls back to :memory:
}

// ── print_table_results ──

#[test]
fn table_results_empty() {
    let conn = setup_db();
    let r = print_table_results("nothing", &conn, &[], false, false, false);
    assert!(r.is_ok());
}

#[test]
fn table_results_with_data() {
    let conn = setup_db();
    let results = sample_fts();
    let r = print_table_results("nginx", &conn, &results, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn table_results_with_history() {
    let conn = setup_db();
    let results = sample_fts();
    let r = print_table_results("nginx", &conn, &results, true, false, false);
    assert!(r.is_ok());
}

#[test]
fn table_results_with_timing() {
    let conn = setup_db();
    let results = sample_fts();
    let r = print_table_results("nginx", &conn, &results, false, true, false);
    assert!(r.is_ok());
}

#[test]
fn table_results_with_reversibility() {
    let conn = setup_db();
    let results = sample_fts();
    let r = print_table_results("nginx", &conn, &results, false, false, true);
    assert!(r.is_ok());
}

#[test]
fn table_results_all_flags() {
    let conn = setup_db();
    let results = sample_fts();
    let r = print_table_results("nginx", &conn, &results, true, true, true);
    assert!(r.is_ok());
}

// ── cmd_query_health ──

#[test]
fn query_health_text_empty() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_query_health(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn query_health_json_empty() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_query_health(dir.path(), true);
    assert!(r.is_ok());
}

#[test]
fn query_health_text_with_state() {
    let dir = setup_state_dir();
    let r = cmd_query_health(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn query_health_json_with_state() {
    let dir = setup_state_dir();
    let r = cmd_query_health(dir.path(), true);
    assert!(r.is_ok());
}

// ── cmd_query_drift ──

#[test]
fn query_drift_text_empty() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_query_drift(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn query_drift_json_empty() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_query_drift(dir.path(), true);
    assert!(r.is_ok());
}

// ── cmd_query_churn ──

#[test]
fn query_churn_text_empty() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_query_churn(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn query_churn_json_empty() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_query_churn(dir.path(), true);
    assert!(r.is_ok());
}

// ── which_runtime ──

#[test]
fn which_runtime_sh() {
    // sh should exist on any unix system
    assert!(which_runtime("sh"));
}

#[test]
fn which_runtime_nonexistent() {
    assert!(!which_runtime("zzz_no_such_binary_zzz"));
}
