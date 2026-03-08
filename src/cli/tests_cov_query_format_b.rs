//! Coverage tests for query_format.rs — resolve_since, cmd_query_*, print_table_results.

use crate::core::store::db;

fn setup_db_with_state(dir: &std::path::Path) -> rusqlite::Connection {
    let conn = db::open_state_db(dir.join("state.db").as_path()).unwrap();
    conn.execute(
        "INSERT INTO generations (generation_num, run_id, config_hash, created_at) VALUES (1, 'r1', 'h1', '2026-01-01')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO machines (name, first_seen, last_seen) VALUES ('m1', '2026-01-01', '2026-01-01')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO resources (resource_id, machine_id, generation_id, resource_type, status, applied_at) \
         VALUES ('pkg-a', 1, 1, 'package', 'converged', '2026-01-01')",
        [],
    ).unwrap();
    conn
}

fn setup_in_memory_db() -> rusqlite::Connection {
    let conn = db::open_state_db(std::path::Path::new(":memory:")).unwrap();
    conn.execute(
        "INSERT INTO generations (generation_num, run_id, config_hash, created_at) VALUES (1, 'run-1', 'hash-1', '2026-03-06')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO machines (name, first_seen, last_seen) VALUES ('test-machine', '2026-03-06', '2026-03-06')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO resources (resource_id, machine_id, generation_id, resource_type, status, path, duration_secs, reversibility, applied_at) \
         VALUES ('nginx-pkg', 1, 1, 'package', 'converged', '/usr/bin/nginx', 1.5, 'reversible', '2026-03-06')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO events (resource_id, machine, event_type, run_id, timestamp, duration_ms) \
         VALUES ('nginx-pkg', 'test-machine', 'apply', 'run-001', '2026-03-06T10:00:00Z', 1500)",
        [],
    ).unwrap();
    conn.execute("INSERT INTO resources_fts(resources_fts) VALUES('rebuild')", []).unwrap();
    conn
}

fn sample_results() -> Vec<db::FtsResult> {
    vec![db::FtsResult {
        resource_id: "nginx-pkg".into(),
        resource_type: "package".into(),
        status: "converged".into(),
        path: Some("/usr/bin/nginx".into()),
        rank: -1.5,
    }]
}

// --- resolve_since and time helpers ---

#[test]
fn resolve_since_hours() {
    let result = super::query_format::resolve_since("2h");
    assert!(result.contains("T"), "expected ISO format, got: {result}");
    assert!(result.len() >= 19);
}

#[test]
fn resolve_since_days() {
    let result = super::query_format::resolve_since("7d");
    assert!(result.contains("T"));
}

#[test]
fn resolve_since_minutes() {
    let result = super::query_format::resolve_since("30m");
    assert!(result.contains("T"));
}

#[test]
fn resolve_since_iso_passthrough() {
    let result = super::query_format::resolve_since("2026-03-01T00:00:00Z");
    assert_eq!(result, "2026-03-01T00:00:00Z");
}

#[test]
fn resolve_since_invalid_number() {
    let result = super::query_format::resolve_since("abch");
    assert_eq!(result, "abch");
}

#[test]
fn resolve_since_invalid_days() {
    let result = super::query_format::resolve_since("xyzd");
    assert_eq!(result, "xyzd");
}

#[test]
fn resolve_since_invalid_minutes() {
    let result = super::query_format::resolve_since("foom");
    assert_eq!(result, "foom");
}

#[test]
fn epoch_days_to_ymd_epoch() {
    let (y, m, d) = super::query_format::epoch_days_to_ymd(0);
    assert_eq!((y, m, d), (1970, 1, 1));
}

#[test]
fn epoch_days_to_ymd_known_date() {
    let days = (2026 - 1970) * 365 + 14 + 31 + 28 + 8 - 1;
    let (y, m, _d) = super::query_format::epoch_days_to_ymd(days as i64);
    assert_eq!(y, 2026);
    assert_eq!(m, 3);
}

#[test]
fn chrono_now_minus_seconds_format() {
    let ts = super::query_format::chrono_now_minus_seconds(0);
    assert!(ts.contains("T"));
    assert!(ts.len() >= 19);
}

#[test]
fn chrono_now_minus_seconds_one_day() {
    let ts = super::query_format::chrono_now_minus_seconds(86400);
    assert!(ts.contains("T"));
}

// --- print_table_results ---

#[test]
fn print_table_results_with_data() {
    let conn = setup_in_memory_db();
    let results = sample_results();
    let r = super::query_format::print_table_results("nginx", &conn, &results, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn print_table_results_empty() {
    let conn = setup_in_memory_db();
    let r = super::query_format::print_table_results("missing", &conn, &[], false, false, false);
    assert!(r.is_ok());
}

#[test]
fn print_table_results_with_history() {
    let conn = setup_in_memory_db();
    let results = sample_results();
    let r = super::query_format::print_table_results("nginx", &conn, &results, true, false, false);
    assert!(r.is_ok());
}

#[test]
fn print_table_results_with_timing() {
    let conn = setup_in_memory_db();
    let results = sample_results();
    let r = super::query_format::print_table_results("nginx", &conn, &results, false, true, false);
    assert!(r.is_ok());
}

#[test]
fn print_table_results_with_reversibility() {
    let conn = setup_in_memory_db();
    let results = sample_results();
    let r = super::query_format::print_table_results("nginx", &conn, &results, false, false, true);
    assert!(r.is_ok());
}

#[test]
fn print_table_results_all_flags() {
    let conn = setup_in_memory_db();
    let results = sample_results();
    let r = super::query_format::print_table_results("nginx", &conn, &results, true, true, true);
    assert!(r.is_ok());
}

// --- cmd_query_health ---

#[test]
fn cmd_query_health_text() {
    let dir = tempfile::tempdir().unwrap();
    let conn = setup_db_with_state(dir.path());
    drop(conn);
    let r = super::query_format::cmd_query_health(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_health_json() {
    let dir = tempfile::tempdir().unwrap();
    let conn = setup_db_with_state(dir.path());
    drop(conn);
    let r = super::query_format::cmd_query_health(dir.path(), true);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_health_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    let r = super::query_format::cmd_query_health(dir.path(), false);
    assert!(r.is_ok());
}

// --- cmd_query_drift ---

#[test]
fn cmd_query_drift_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    let r = super::query_format::cmd_query_drift(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_drift_json_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    let r = super::query_format::cmd_query_drift(dir.path(), true);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_drift_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let conn = setup_db_with_state(dir.path());
    // Update existing resource to have differing hashes (triggers drift detection)
    conn.execute(
        "UPDATE resources SET content_hash = 'blake3:aaaaaaaaaaaaaaaaaaaaa', live_hash = 'blake3:bbbbbbbbbbbbbbbbbbbbb' WHERE resource_id = 'pkg-a'",
        [],
    ).unwrap();
    drop(conn);
    let r = super::query_format::cmd_query_drift(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_drift_json_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let conn = setup_db_with_state(dir.path());
    conn.execute(
        "UPDATE resources SET content_hash = 'blake3:aaaaaaaaaaaaaaaaaaaaa', live_hash = 'blake3:bbbbbbbbbbbbbbbbbbbbb' WHERE resource_id = 'pkg-a'",
        [],
    ).unwrap();
    drop(conn);
    let r = super::query_format::cmd_query_drift(dir.path(), true);
    assert!(r.is_ok());
}

// --- cmd_query_churn ---

#[test]
fn cmd_query_churn_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    let r = super::query_format::cmd_query_churn(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_churn_json_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    let r = super::query_format::cmd_query_churn(dir.path(), true);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_churn_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    conn.execute(
        "INSERT INTO events (resource_id, machine, event_type, run_id, timestamp, duration_ms) \
         VALUES ('pkg-a', 'm1', 'apply', 'run-1', '2026-01-01T10:00:00Z', 500)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO events (resource_id, machine, event_type, run_id, timestamp, duration_ms) \
         VALUES ('pkg-a', 'm1', 'apply', 'run-2', '2026-01-02T10:00:00Z', 300)",
        [],
    ).unwrap();
    drop(conn);
    let r = super::query_format::cmd_query_churn(dir.path(), false);
    assert!(r.is_ok());
}

// --- cmd_query_events ---

#[test]
fn cmd_query_events_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    let r = super::query_format::cmd_query_events(dir.path(), None, None, false);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_events_json_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    let r = super::query_format::cmd_query_events(dir.path(), None, None, true);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_events_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    conn.execute(
        "INSERT INTO events (resource_id, machine, event_type, run_id, timestamp, duration_ms) \
         VALUES ('pkg-a', 'm1', 'apply', 'run-001-test-abcdef-x', '2026-01-01T10:00:00Z', 1500)",
        [],
    ).unwrap();
    drop(conn);
    let r = super::query_format::cmd_query_events(dir.path(), None, None, false);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_events_with_since() {
    let dir = tempfile::tempdir().unwrap();
    let conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    conn.execute(
        "INSERT INTO events (resource_id, machine, event_type, run_id, timestamp, duration_ms) \
         VALUES ('pkg-a', 'm1', 'apply', 'run-001-test-abcdef-x', '2026-01-01T10:00:00Z', 1500)",
        [],
    ).unwrap();
    drop(conn);
    let r = super::query_format::cmd_query_events(dir.path(), Some("24h"), None, false);
    assert!(r.is_ok());
}

// --- cmd_query_failures ---

#[test]
fn cmd_query_failures_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    let r = super::query_format::cmd_query_failures(dir.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_failures_json_empty() {
    let dir = tempfile::tempdir().unwrap();
    let _conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    let r = super::query_format::cmd_query_failures(dir.path(), None, true);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_failures_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    conn.execute(
        "INSERT INTO events (resource_id, machine, event_type, run_id, timestamp, duration_ms, exit_code, stderr_tail) \
         VALUES ('pkg-a', 'm1', 'apply-failed', 'run-001-test-abcdef-x', '2026-01-01T10:00:00Z', 1500, 1, 'error: package not found\ndetail line')",
        [],
    ).unwrap();
    drop(conn);
    let r = super::query_format::cmd_query_failures(dir.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn cmd_query_failures_with_since() {
    let dir = tempfile::tempdir().unwrap();
    let _conn = db::open_state_db(dir.path().join("state.db").as_path()).unwrap();
    let r = super::query_format::cmd_query_failures(dir.path(), Some("7d"), false);
    assert!(r.is_ok());
}
