//! Additional coverage for query.rs — health_pct, query_history, drift with data.

use super::db;
use super::query::*;

fn setup_db() -> rusqlite::Connection {
    let conn = db::open_state_db(std::path::Path::new(":memory:")).unwrap();
    conn.execute(
        "INSERT INTO machines (name, first_seen, last_seen) VALUES (?1, ?2, ?3)",
        ["m1", "2026-01-01", "2026-03-08"],
    )
    .unwrap();
    conn
}

fn insert_resource(
    conn: &rusqlite::Connection,
    resource_id: &str,
    status: &str,
    content_hash: &str,
    live_hash: &str,
) {
    conn.execute(
        "INSERT INTO generations (generation_num, run_id, config_hash, created_at) \
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            rand_gen(conn),
            format!("run-{resource_id}"),
            "hash",
            "2026-03-08"
        ],
    )
    .unwrap();
    let gen_id: i64 = conn
        .query_row("SELECT MAX(id) FROM generations", [], |r| r.get(0))
        .unwrap();
    conn.execute(
        "INSERT INTO resources (machine_id, generation_id, resource_id, resource_type, status, \
         content_hash, live_hash, applied_at) \
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            gen_id,
            resource_id,
            "file",
            status,
            content_hash,
            live_hash,
            "2026-03-08"
        ],
    )
    .unwrap();
}

fn rand_gen(conn: &rusqlite::Connection) -> i64 {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM generations", [], |r| r.get(0))
        .unwrap();
    count + 1
}

// ── HealthSummary::health_pct ───────────────────────────────────────

#[test]
fn health_pct_all_converged() {
    let summary = HealthSummary {
        machines: vec![],
        total_resources: 10,
        total_converged: 10,
        total_drifted: 0,
        total_failed: 0,
    };
    assert!((summary.health_pct() - 100.0).abs() < 0.01);
}

#[test]
fn health_pct_half_converged() {
    let summary = HealthSummary {
        machines: vec![],
        total_resources: 100,
        total_converged: 50,
        total_drifted: 30,
        total_failed: 20,
    };
    assert!((summary.health_pct() - 50.0).abs() < 0.01);
}

#[test]
fn health_pct_empty() {
    let summary = HealthSummary {
        machines: vec![],
        total_resources: 0,
        total_converged: 0,
        total_drifted: 0,
        total_failed: 0,
    };
    assert!((summary.health_pct() - 100.0).abs() < 0.01);
}

// ── query_health with data ──────────────────────────────────────────

#[test]
fn health_empty_db() {
    let conn = setup_db();
    let health = query_health(&conn).unwrap();
    assert!(health.machines.is_empty());
    assert_eq!(health.total_resources, 0);
}

#[test]
fn health_with_resources() {
    let conn = setup_db();
    insert_resource(&conn, "nginx", "converged", "abc", "abc");
    insert_resource(&conn, "app", "failed", "def", "def");
    let health = query_health(&conn).unwrap();
    assert_eq!(health.total_resources, 2);
    assert_eq!(health.total_converged, 1);
    assert_eq!(health.total_failed, 1);
}

// ── query_history ───────────────────────────────────────────────────

#[test]
fn history_empty() {
    let conn = setup_db();
    let history = query_history(&conn, "nonexistent").unwrap();
    assert!(history.is_empty());
}

#[test]
fn history_with_events() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp, duration_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            "run-1",
            "nginx",
            "m1",
            "converged",
            "2026-03-08T10:00:00",
            500
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp, duration_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            "run-2",
            "nginx",
            "m1",
            "converged",
            "2026-03-08T11:00:00",
            300
        ],
    )
    .unwrap();
    let history = query_history(&conn, "nginx").unwrap();
    assert_eq!(history.len(), 2);
    // Most recent first
    assert_eq!(history[0].run_id, "run-2");
}

// ── query_drift with actual drift ───────────────────────────────────

#[test]
fn drift_none() {
    let conn = setup_db();
    insert_resource(&conn, "nginx", "converged", "abc", "abc");
    let drift = query_drift(&conn).unwrap();
    assert!(drift.is_empty());
}

#[test]
fn drift_detected() {
    let conn = setup_db();
    insert_resource(
        &conn,
        "nginx",
        "converged",
        "expected_hash",
        "actual_different_hash",
    );
    let drift = query_drift(&conn).unwrap();
    assert_eq!(drift.len(), 1);
    assert_eq!(drift[0].resource_id, "nginx");
    assert_eq!(drift[0].content_hash, "expected_hash");
    assert_eq!(drift[0].live_hash, "actual_different_hash");
}

// ── query_churn ─────────────────────────────────────────────────────

#[test]
fn churn_empty() {
    let conn = setup_db();
    let churn = query_churn(&conn).unwrap();
    assert!(churn.is_empty());
}

#[test]
fn churn_with_multiple_events() {
    let conn = setup_db();
    for i in 0..5 {
        conn.execute(
            "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                format!("run-{i}"),
                "nginx",
                "m1",
                "resource_converged",
                format!("2026-03-0{i}T10:00:00")
            ],
        )
        .unwrap();
    }
    let churn = query_churn(&conn).unwrap();
    assert_eq!(churn.len(), 1);
    assert_eq!(churn[0].resource_id, "nginx");
    assert_eq!(churn[0].event_count, 5);
    assert_eq!(churn[0].distinct_runs, 5);
}

// ── query_events with filters ───────────────────────────────────────

#[test]
fn events_with_since_and_run() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["run-a", "r1", "m1", "converged", "2026-03-01T10:00:00"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["run-b", "r2", "m1", "converged", "2026-03-08T10:00:00"],
    )
    .unwrap();

    // Filter by since
    let events = query_events(&conn, Some("2026-03-05"), None, 50).unwrap();
    assert_eq!(events.len(), 1);

    // Filter by run
    let events = query_events(&conn, None, Some("run-a"), 50).unwrap();
    assert_eq!(events.len(), 1);

    // Both filters
    let events = query_events(&conn, Some("2026-03-05"), Some("run-b"), 50).unwrap();
    assert_eq!(events.len(), 1);
}

// ── query_failures ──────────────────────────────────────────────────

#[test]
fn failures_empty() {
    let conn = setup_db();
    let failures = query_failures(&conn, None, 50).unwrap();
    assert!(failures.is_empty());
}

#[test]
fn failures_with_data() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp, exit_code, stderr_tail) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["run-1", "pkg", "m1", "resource_failed", "2026-03-08T12:00:00", 1, "not found"],
    )
    .unwrap();
    let failures = query_failures(&conn, None, 50).unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].exit_code, Some(1));
}

#[test]
fn failures_with_since_filter() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            "run-1",
            "pkg",
            "m1",
            "resource_failed",
            "2026-03-01T12:00:00"
        ],
    )
    .unwrap();
    let failures = query_failures(&conn, Some("2026-03-05"), 50).unwrap();
    assert!(failures.is_empty());
}
