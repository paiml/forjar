//! Coverage tests for cli/query_format.rs (FJ-2001/2004).

use crate::core::store::db::{self, FtsResult};

fn setup_db() -> rusqlite::Connection {
    let conn = db::open_state_db(std::path::Path::new(":memory:")).unwrap();
    // Insert generation first (FK)
    conn.execute(
        "INSERT INTO generations (generation_num, run_id, config_hash, created_at) VALUES (1, 'run-1', 'hash-1', '2026-03-06')",
        [],
    ).unwrap();
    // Insert machine
    conn.execute(
        "INSERT INTO machines (name, first_seen, last_seen) VALUES ('test-machine', '2026-03-06', '2026-03-06')",
        [],
    ).unwrap();
    // Insert resources (machine_id=1, generation_id=1)
    conn.execute(
        "INSERT INTO resources (resource_id, machine_id, generation_id, resource_type, status, path, duration_secs, reversibility, applied_at) \
         VALUES ('nginx-pkg', 1, 1, 'package', 'converged', '/usr/bin/nginx', 1.5, 'reversible', '2026-03-06')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO resources (resource_id, machine_id, generation_id, resource_type, status, path, duration_secs, reversibility, applied_at) \
         VALUES ('app-config', 1, 1, 'file', 'converged', '/etc/app/config.yaml', 0.3, 'reversible', '2026-03-06')",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO resources (resource_id, machine_id, generation_id, resource_type, status, path, duration_secs, reversibility, applied_at) \
         VALUES ('svc-nginx', 1, 1, 'service', 'converged', NULL, 2.1, 'irreversible', '2026-03-06')",
        [],
    ).unwrap();
    // Insert events
    conn.execute(
        "INSERT INTO events (resource_id, machine, event_type, run_id, timestamp, duration_ms) \
         VALUES ('nginx-pkg', 'test-machine', 'apply', 'run-001', '2026-03-06T10:00:00Z', 1500)",
        [],
    ).unwrap();
    conn.execute(
        "INSERT INTO events (resource_id, machine, event_type, run_id, timestamp, duration_ms) \
         VALUES ('nginx-pkg', 'test-machine', 'drift-check', 'run-002', '2026-03-06T11:00:00Z', 200)",
        [],
    ).unwrap();
    // Rebuild FTS
    conn.execute("INSERT INTO resources_fts(resources_fts) VALUES('rebuild')", []).unwrap();
    conn
}

fn sample_results() -> Vec<FtsResult> {
    vec![
        FtsResult {
            resource_id: "nginx-pkg".into(),
            resource_type: "package".into(),
            status: "converged".into(),
            path: Some("/usr/bin/nginx".into()),
            rank: -1.5,
        },
        FtsResult {
            resource_id: "app-config".into(),
            resource_type: "file".into(),
            status: "converged".into(),
            path: Some("/etc/app/config.yaml".into()),
            rank: -2.0,
        },
        FtsResult {
            resource_id: "svc-nginx".into(),
            resource_type: "service".into(),
            status: "converged".into(),
            path: None,
            rank: -3.0,
        },
    ]
}

#[test]
fn timing_stats_with_data() {
    let conn = setup_db();
    let results = sample_results();
    let r = super::query_format::print_timing_stats(&conn, &results);
    assert!(r.is_ok());
}

#[test]
fn timing_stats_empty_results() {
    let conn = setup_db();
    let r = super::query_format::print_timing_stats(&conn, &[]);
    assert!(r.is_ok());
}

#[test]
fn timing_stats_single_result() {
    let conn = setup_db();
    let results = vec![sample_results()[0].clone()];
    let r = super::query_format::print_timing_stats(&conn, &results);
    assert!(r.is_ok());
}

#[test]
fn history_with_events() {
    let conn = setup_db();
    let results = sample_results();
    let r = super::query_format::print_history(&conn, &results);
    assert!(r.is_ok());
}

#[test]
fn history_empty_results() {
    let conn = setup_db();
    let r = super::query_format::print_history(&conn, &[]);
    assert!(r.is_ok());
}

#[test]
fn reversibility_with_data() {
    let conn = setup_db();
    let results = sample_results();
    let r = super::query_format::print_reversibility(&conn, &results);
    assert!(r.is_ok());
}

#[test]
fn reversibility_empty_results() {
    let conn = setup_db();
    let r = super::query_format::print_reversibility(&conn, &[]);
    assert!(r.is_ok());
}

#[test]
fn json_output_no_history() {
    let conn = setup_db();
    let results = sample_results();
    super::query_format::print_json(&conn, "nginx", &results, false);
}

#[test]
fn json_output_with_history() {
    let conn = setup_db();
    let results = sample_results();
    super::query_format::print_json(&conn, "nginx", &results, true);
}

#[test]
fn json_output_empty() {
    let conn = setup_db();
    super::query_format::print_json(&conn, "missing", &[], false);
}

#[test]
fn csv_output() {
    let results = sample_results();
    super::query_format::print_csv(&results);
}

#[test]
fn csv_output_empty() {
    super::query_format::print_csv(&[]);
}

#[test]
fn csv_output_none_path() {
    let results = vec![FtsResult {
        resource_id: "svc".into(),
        resource_type: "service".into(),
        status: "converged".into(),
        path: None,
        rank: -1.0,
    }];
    super::query_format::print_csv(&results);
}

#[test]
fn git_history_in_repo() {
    let results = sample_results();
    let r = super::query_format::print_git_history("forjar", &results);
    assert!(r.is_ok());
}

#[test]
fn git_history_empty_query() {
    let r = super::query_format::print_git_history("zzz_no_match_zzz", &[]);
    assert!(r.is_ok());
}

#[test]
fn print_sql_basic() {
    super::query_format::print_sql("nginx", None);
}

#[test]
fn print_sql_with_type() {
    super::query_format::print_sql("nginx", Some("package"));
}

#[test]
fn git_log_entry_serde() {
    let entry = super::query_format::GitLogEntry {
        hash: "abc123".into(),
        message: "fix thing".into(),
        files: vec!["src/main.rs".into()],
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("abc123"));
    assert!(json.contains("fix thing"));
}
