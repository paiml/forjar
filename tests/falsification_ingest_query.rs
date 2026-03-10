//! FJ-2001: State ingest pipeline and query engine.
//! Usage: cargo test --test falsification_ingest_query

use forjar::core::store::db;
use forjar::core::store::ingest::ingest_state_dir;
use forjar::core::store::query::{
    query_churn, query_drift, query_events, query_failures, query_health, HealthSummary,
};
use std::path::Path;

// ── helpers ──

fn setup_state_dir(base: &Path) -> std::path::PathBuf {
    let state_dir = base.join("state");
    let machine_dir = state_dir.join("web-01");
    std::fs::create_dir_all(&machine_dir).unwrap();

    // state.lock.yaml
    std::fs::write(
        machine_dir.join("state.lock.yaml"),
        r#"hostname: web-01.prod
generated_at: "2026-03-08T12:00:00Z"
resources:
  nginx-pkg:
    type: package
    status: converged
    applied_at: "2026-03-08T12:00:00Z"
    hash: blake3:aabbcc
    duration_seconds: 2.5
    details:
      content_hash: blake3:aabbcc
      live_hash: blake3:aabbcc
  app-config:
    type: file
    status: converged
    applied_at: "2026-03-08T12:01:00Z"
    hash: blake3:ddeeff
    duration_seconds: 0.3
    details:
      path: /etc/app/config.yaml
      content_hash: blake3:ddeeff
      live_hash: blake3:ddeeff
      content_preview: "port: 8080\nworkers: 4"
"#,
    )
    .unwrap();

    // events.jsonl
    std::fs::write(
        machine_dir.join("events.jsonl"),
        r#"{"run_id":"run-001","event":"resource_converged","resource":"nginx-pkg","ts":"2026-03-08T12:00:00Z","duration_seconds":2.5}
{"run_id":"run-001","event":"resource_converged","resource":"app-config","ts":"2026-03-08T12:01:00Z","duration_seconds":0.3}
{"run_id":"run-002","event":"resource_failed","resource":"bad-svc","ts":"2026-03-08T13:00:00Z","duration_seconds":0.0}
"#,
    )
    .unwrap();

    state_dir
}

fn setup_multi_machine_state(base: &Path) -> std::path::PathBuf {
    let state_dir = base.join("state");

    for (name, status) in [("app-01", "converged"), ("app-02", "drifted")] {
        let dir = state_dir.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("state.lock.yaml"),
            format!(
                r#"hostname: {name}.prod
generated_at: "2026-03-08T10:00:00Z"
resources:
  svc:
    type: service
    status: {status}
    applied_at: "2026-03-08T10:00:00Z"
    hash: blake3:111222
    details:
      content_hash: blake3:111222
      live_hash: {}
"#,
                if status == "drifted" {
                    "blake3:333444"
                } else {
                    "blake3:111222"
                }
            ),
        )
        .unwrap();
    }

    state_dir
}

// ── FJ-2001: ingest_state_dir ──

#[test]
fn ingest_single_machine() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();

    let result = ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(result.machines, 1);
    assert_eq!(result.resources, 2);
    assert_eq!(result.events, 3);
}

#[test]
fn ingest_display() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    let result = ingest_state_dir(&conn, &state_dir).unwrap();
    let display = format!("{result}");
    assert!(display.contains("1 machines"));
    assert!(display.contains("2 resources"));
    assert!(display.contains("3 events"));
}

#[test]
fn ingest_multi_machine() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_multi_machine_state(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();

    let result = ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(result.machines, 2);
    assert_eq!(result.resources, 2);
}

#[test]
fn ingest_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = tmp.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();

    let result = ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(result.machines, 0);
    assert_eq!(result.resources, 0);
    assert_eq!(result.events, 0);
}

#[test]
fn ingest_skips_non_dir_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = tmp.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    std::fs::write(state_dir.join("stray-file.txt"), "ignore me").unwrap();
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();

    let result = ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(result.machines, 0);
}

#[test]
fn ingest_skips_dir_without_lockfile() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = tmp.path().join("state");
    let empty_machine = state_dir.join("no-lock");
    std::fs::create_dir_all(&empty_machine).unwrap();
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();

    let result = ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(result.machines, 0);
}

// ── FJ-2001: incremental ingest (F3 cursor) ──

#[test]
fn ingest_incremental_skips_unchanged_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();

    // First ingest
    let r1 = ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(r1.resources, 2);

    // Second ingest — same lock file, should skip resources
    let r2 = ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(r2.resources, 0); // lock hash unchanged → skip
    assert_eq!(r2.events, 0); // events offset caught up
}

#[test]
fn ingest_incremental_re_ingests_changed_lock() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();

    ingest_state_dir(&conn, &state_dir).unwrap();

    // Modify the lock file
    let lock_path = state_dir.join("web-01/state.lock.yaml");
    let mut content = std::fs::read_to_string(&lock_path).unwrap();
    content.push_str("  extra-pkg:\n    type: package\n    status: converged\n    applied_at: \"2026-03-08T14:00:00Z\"\n    hash: blake3:newone\n");
    std::fs::write(&lock_path, content).unwrap();

    let r2 = ingest_state_dir(&conn, &state_dir).unwrap();
    assert!(r2.resources > 0); // lock hash changed → re-ingest
}

// ── FJ-2001: query_health ──

#[test]
fn query_health_after_ingest() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let health = query_health(&conn).unwrap();
    assert_eq!(health.machines.len(), 1);
    assert_eq!(health.machines[0].name, "web-01");
    assert_eq!(health.total_resources, 2);
    assert_eq!(health.total_converged, 2);
    assert_eq!(health.total_drifted, 0);
}

#[test]
fn health_pct_all_converged() {
    let summary = HealthSummary {
        machines: vec![],
        total_resources: 10,
        total_converged: 10,
        total_drifted: 0,
        total_failed: 0,
    };
    assert!((summary.health_pct() - 100.0).abs() < 0.1);
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
    assert!((summary.health_pct() - 100.0).abs() < 0.1);
}

#[test]
fn health_pct_partial() {
    let summary = HealthSummary {
        machines: vec![],
        total_resources: 4,
        total_converged: 3,
        total_drifted: 1,
        total_failed: 0,
    };
    assert!((summary.health_pct() - 75.0).abs() < 0.1);
}

// ── FJ-2001: query_drift ──

#[test]
fn query_drift_finds_drifted() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_multi_machine_state(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let drift = query_drift(&conn).unwrap();
    assert_eq!(drift.len(), 1);
    assert_eq!(drift[0].machine, "app-02");
    assert_ne!(drift[0].content_hash, drift[0].live_hash);
}

#[test]
fn query_drift_none_when_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let drift = query_drift(&conn).unwrap();
    assert!(drift.is_empty());
}

// ── FJ-2001: query_events ──

#[test]
fn query_events_all() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let events = query_events(&conn, None, None, 100).unwrap();
    assert_eq!(events.len(), 3);
}

#[test]
fn query_events_by_run() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let events = query_events(&conn, None, Some("run-001"), 100).unwrap();
    assert_eq!(events.len(), 2);
}

#[test]
fn query_events_since() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let events = query_events(&conn, Some("2026-03-08T12:30:00Z"), None, 100).unwrap();
    assert_eq!(events.len(), 1); // only the failure after 12:30
}

#[test]
fn query_events_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let events = query_events(&conn, None, None, 1).unwrap();
    assert_eq!(events.len(), 1);
}

// ── FJ-2001: query_failures ──

#[test]
fn query_failures_finds_failed() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let failures = query_failures(&conn, None, 100).unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].resource_id, "bad-svc");
}

#[test]
fn query_failures_since_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let failures = query_failures(&conn, Some("2026-03-08T14:00:00Z"), 100).unwrap();
    assert!(failures.is_empty());
}

// ── FJ-2001: query_churn ──

#[test]
fn query_churn_counts_converged() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let churn = query_churn(&conn).unwrap();
    // 2 converged events in events.jsonl
    assert!(!churn.is_empty());
    assert!(churn.iter().any(|c| c.resource_id == "nginx-pkg"));
}

// ── FJ-2001: destroy log ingest ──

#[test]
fn ingest_destroy_log() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let destroy_path = state_dir.join("web-01/destroy-log.jsonl");
    std::fs::write(
        &destroy_path,
        r#"{"resource_id":"old-pkg","resource_type":"package","pre_hash":"blake3:old123","timestamp":"2026-03-08T15:00:00Z"}
"#,
    )
    .unwrap();

    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    let result = ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(result.machines, 1);

    // Verify destroy_log table has an entry
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM destroy_log", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

// ── FJ-2001: generations ingest ──

#[test]
fn ingest_generations_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let state_dir = setup_state_dir(tmp.path());
    let gens_dir = state_dir.join("generations");
    std::fs::create_dir_all(&gens_dir).unwrap();
    std::fs::write(
        gens_dir.join("gen-001.yaml"),
        r#"generation: 5
run_id: "apply-005"
config_hash: "blake3:cfgabc"
created_at: "2026-03-08T16:00:00Z"
git_ref: "main@abc123"
action: "apply"
"#,
    )
    .unwrap();

    let conn = db::open_state_db(Path::new(":memory:")).unwrap();
    ingest_state_dir(&conn, &state_dir).unwrap();

    let gen_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM generations WHERE run_id = 'apply-005'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(gen_count, 1);
}
