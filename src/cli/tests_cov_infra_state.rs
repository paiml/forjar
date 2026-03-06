//! Coverage tests for cli/infra.rs state commands and cli/history.rs.

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
    hash: abc123def456
    applied_at: "2025-01-01T00:00:00Z"
    duration_seconds: 2.5
  config:
    type: file
    status: converged
    hash: ghi789jkl012
    applied_at: "2025-01-01T00:01:00Z"
    duration_seconds: 0.5
"#;

const LOCK_DB1: &str = r#"schema: "1"
machine: db1
hostname: db1
generated_at: "2025-01-01T00:00:00Z"
generator: forjar-test
blake3_version: "1.0"
resources:
  postgres:
    type: package
    status: converged
    hash: mno345pqr678
    applied_at: "2025-01-01T00:02:00Z"
    duration_seconds: 1.0
"#;

fn setup_state(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("web1")).unwrap();
    std::fs::write(dir.join("web1/state.lock.yaml"), LOCK_WEB1).unwrap();
    std::fs::create_dir_all(dir.join("db1")).unwrap();
    std::fs::write(dir.join("db1/state.lock.yaml"), LOCK_DB1).unwrap();
}

fn setup_events(dir: &std::path::Path) {
    let event_converged = r#"{"ts":"2025-01-01T00:00:00Z","event":"resource_converged","machine":"web1","resource":"nginx","duration_seconds":2.5,"hash":"blake3:abc123"}"#;
    let event_failed = r#"{"ts":"2025-01-01T00:01:00Z","event":"resource_failed","machine":"web1","resource":"config","error":"exit 1"}"#;
    let events = format!("{event_converged}\n{event_failed}\n");
    std::fs::write(dir.join("web1/events.jsonl"), &events).unwrap();
    let db_event = r#"{"ts":"2025-01-01T00:02:00Z","event":"resource_converged","machine":"db1","resource":"postgres","duration_seconds":1.0,"hash":"blake3:mno345"}"#;
    std::fs::write(dir.join("db1/events.jsonl"), format!("{db_event}\n")).unwrap();
}

// ── cmd_state_list ──

#[test]
fn state_list_no_dir() {
    let d = tempfile::tempdir().unwrap();
    let r = super::infra::cmd_state_list(&d.path().join("nonexistent"), None, false);
    assert!(r.is_ok());
}

#[test]
fn state_list_no_dir_json() {
    let d = tempfile::tempdir().unwrap();
    let r = super::infra::cmd_state_list(&d.path().join("nonexistent"), None, true);
    assert!(r.is_ok());
}

#[test]
fn state_list_empty() {
    let d = tempfile::tempdir().unwrap();
    let r = super::infra::cmd_state_list(d.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn state_list_with_data() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_list(d.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn state_list_with_data_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_list(d.path(), None, true);
    assert!(r.is_ok());
}

#[test]
fn state_list_machine_filter() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_list(d.path(), Some("web1"), false);
    assert!(r.is_ok());
}

#[test]
fn state_list_machine_filter_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_list(d.path(), Some("db1"), true);
    assert!(r.is_ok());
}

#[test]
fn state_list_machine_filter_no_match() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_list(d.path(), Some("zzz"), false);
    assert!(r.is_ok());
}

// ── cmd_state_mv ──

#[test]
fn state_mv_same_id() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_mv(d.path(), "nginx", "nginx", None);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("same"));
}

#[test]
fn state_mv_no_dir() {
    let d = tempfile::tempdir().unwrap();
    let r = super::infra::cmd_state_mv(&d.path().join("nonexistent"), "a", "b", None);
    assert!(r.is_err());
}

#[test]
fn state_mv_success() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_mv(d.path(), "nginx", "nginx-new", Some("web1"));
    assert!(r.is_ok());
}

#[test]
fn state_mv_not_found() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_mv(d.path(), "nonexistent", "new", None);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("not found"));
}

#[test]
fn state_mv_target_exists() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_mv(d.path(), "nginx", "config", Some("web1"));
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("already exists"));
}

// ── cmd_state_rm ──

#[test]
fn state_rm_no_dir() {
    let d = tempfile::tempdir().unwrap();
    let r = super::infra::cmd_state_rm(&d.path().join("nonexistent"), "nginx", None, false);
    assert!(r.is_err());
}

#[test]
fn state_rm_not_found() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_rm(d.path(), "nonexistent", None, false);
    assert!(r.is_err());
}

#[test]
fn state_rm_without_force_no_deps() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    // nginx has no dependents, so force=false still removes it
    let r = super::infra::cmd_state_rm(d.path(), "nginx", Some("web1"), false);
    assert!(r.is_ok());
}

#[test]
fn state_rm_with_force() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_rm(d.path(), "nginx", Some("web1"), true);
    assert!(r.is_ok());
}

#[test]
fn state_rm_machine_filter() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::infra::cmd_state_rm(d.path(), "postgres", Some("db1"), true);
    assert!(r.is_ok());
}

// ── cmd_history ──

#[test]
fn history_no_state() {
    let d = tempfile::tempdir().unwrap();
    let r = super::history::cmd_history(d.path(), None, 10, false, None);
    assert!(r.is_ok());
}

#[test]
fn history_no_state_json() {
    let d = tempfile::tempdir().unwrap();
    let r = super::history::cmd_history(d.path(), None, 10, true, None);
    assert!(r.is_ok());
}

#[test]
fn history_with_events() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    setup_events(d.path());
    let r = super::history::cmd_history(d.path(), None, 10, false, None);
    assert!(r.is_ok());
}

#[test]
fn history_with_events_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    setup_events(d.path());
    let r = super::history::cmd_history(d.path(), None, 10, true, None);
    assert!(r.is_ok());
}

#[test]
fn history_machine_filter() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    setup_events(d.path());
    let r = super::history::cmd_history(d.path(), Some("web1"), 10, false, None);
    assert!(r.is_ok());
}

#[test]
fn history_limit() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    setup_events(d.path());
    let r = super::history::cmd_history(d.path(), None, 1, false, None);
    assert!(r.is_ok());
}

#[test]
fn history_since() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    setup_events(d.path());
    // since expects a duration like "1h", "7d"
    let r = super::history::cmd_history(d.path(), None, 10, false, Some("7d"));
    assert!(r.is_ok());
}

#[test]
fn history_since_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    setup_events(d.path());
    let r = super::history::cmd_history(d.path(), None, 10, true, Some("24h"));
    assert!(r.is_ok());
}

// ── cmd_history_resource ──

#[test]
fn history_resource_no_state() {
    let d = tempfile::tempdir().unwrap();
    let r = super::history::cmd_history_resource(d.path(), "nginx", 10, false);
    assert!(r.is_ok());
}

#[test]
fn history_resource_with_events() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    setup_events(d.path());
    let r = super::history::cmd_history_resource(d.path(), "nginx", 10, false);
    assert!(r.is_ok());
}

#[test]
fn history_resource_with_events_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    setup_events(d.path());
    let r = super::history::cmd_history_resource(d.path(), "nginx", 10, true);
    assert!(r.is_ok());
}

#[test]
fn history_resource_not_found() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    setup_events(d.path());
    let r = super::history::cmd_history_resource(d.path(), "nonexistent", 10, false);
    assert!(r.is_ok());
}
