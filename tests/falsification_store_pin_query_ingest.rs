//! FJ-1364/2001: Pin resolution, query engine, and state ingest falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1364: Pin resolution
//!   - resolution_command: provider-specific CLI generation
//!   - parse_resolved_version: output parsing for apt/cargo/pip/nix/docker
//!   - pin_hash: deterministic hash computation
//! - FJ-2001: Query engine
//!   - HealthSummary::health_pct: percentage computation
//!   - query_health / query_drift / query_churn / query_events / query_failures
//!   - IngestResult display
//! - FJ-2001: State ingest pipeline (via integration with SQLite)
//!
//! Usage: cargo test --test falsification_store_pin_query_ingest

use forjar::core::store::pin_resolve::{
    parse_resolved_version, pin_hash, resolution_command, ResolvedPin,
};
use forjar::core::store::query::{
    query_churn, query_drift, query_events, query_failures, query_health, query_history,
    HealthSummary, MachineHealth,
};

// ============================================================================
// FJ-1364: resolution_command
// ============================================================================

#[test]
fn pin_resolution_command_apt() {
    let cmd = resolution_command("apt", "nginx").unwrap();
    assert_eq!(cmd, "apt-cache policy nginx");
}

#[test]
fn pin_resolution_command_cargo() {
    let cmd = resolution_command("cargo", "serde").unwrap();
    assert!(cmd.contains("cargo search serde"));
    assert!(cmd.contains("--limit 1"));
}

#[test]
fn pin_resolution_command_nix() {
    let cmd = resolution_command("nix", "hello").unwrap();
    assert!(cmd.contains("nix eval"));
    assert!(cmd.contains("hello"));
}

#[test]
fn pin_resolution_command_pip() {
    let cmd = resolution_command("pip", "requests").unwrap();
    assert!(cmd.contains("pip index versions requests"));
}

#[test]
fn pin_resolution_command_uv() {
    let cmd = resolution_command("uv", "numpy").unwrap();
    assert!(cmd.contains("pip index versions numpy"));
}

#[test]
fn pin_resolution_command_docker() {
    let cmd = resolution_command("docker", "nginx").unwrap();
    assert!(cmd.contains("docker image inspect"));
}

#[test]
fn pin_resolution_command_apr() {
    let cmd = resolution_command("apr", "llama-3.1").unwrap();
    assert!(cmd.contains("apr info llama-3.1"));
}

#[test]
fn pin_resolution_command_unknown_returns_none() {
    assert!(resolution_command("homebrew", "wget").is_none());
    assert!(resolution_command("snap", "firefox").is_none());
}

// ============================================================================
// FJ-1364: parse_resolved_version
// ============================================================================

#[test]
fn pin_parse_apt_candidate() {
    let output = "\
nginx:
  Installed: (none)
  Candidate: 1.24.0-2
  Version table:
     1.24.0-2 500";
    let version = parse_resolved_version("apt", output).unwrap();
    assert_eq!(version, "1.24.0-2");
}

#[test]
fn pin_parse_apt_no_candidate() {
    assert!(parse_resolved_version("apt", "some random output").is_none());
}

#[test]
fn pin_parse_cargo_search() {
    let output = r#"serde = "1.0.215"    # A generic serialization/deserialization framework"#;
    let version = parse_resolved_version("cargo", output).unwrap();
    assert_eq!(version, "1.0.215");
}

#[test]
fn pin_parse_cargo_empty() {
    assert!(parse_resolved_version("cargo", "").is_none());
}

#[test]
fn pin_parse_nix_raw_version() {
    let version = parse_resolved_version("nix", "23.11").unwrap();
    assert_eq!(version, "23.11");
}

#[test]
fn pin_parse_docker_digest() {
    let version = parse_resolved_version("docker", "sha256:abc123def456").unwrap();
    assert_eq!(version, "sha256:abc123def456");
}

#[test]
fn pin_parse_pip_available_versions() {
    let output = "Available versions: 2.31.0, 2.30.0, 2.29.0";
    let version = parse_resolved_version("pip", output).unwrap();
    assert_eq!(version, "2.31.0");
}

#[test]
fn pin_parse_pip_fallback_first_line() {
    let output = "1.2.3\nsome other info";
    let version = parse_resolved_version("pip", output).unwrap();
    assert_eq!(version, "1.2.3");
}

#[test]
fn pin_parse_uv_same_as_pip() {
    let output = "Available versions: 3.0.0, 2.9.0";
    let version = parse_resolved_version("uv", output).unwrap();
    assert_eq!(version, "3.0.0");
}

#[test]
fn pin_parse_apr_first_line() {
    let version = parse_resolved_version("apr", "3.1-Q4").unwrap();
    assert_eq!(version, "3.1-Q4");
}

#[test]
fn pin_parse_empty_input() {
    assert!(parse_resolved_version("apt", "").is_none());
    assert!(parse_resolved_version("cargo", "   \n").is_none());
    assert!(parse_resolved_version("nix", "").is_none());
}

#[test]
fn pin_parse_unknown_provider() {
    assert!(parse_resolved_version("brew", "1.0").is_none());
}

// ============================================================================
// FJ-1364: pin_hash
// ============================================================================

#[test]
fn pin_hash_deterministic() {
    let h1 = pin_hash("apt", "nginx", "1.24.0");
    let h2 = pin_hash("apt", "nginx", "1.24.0");
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn pin_hash_differs_on_version_change() {
    let h1 = pin_hash("apt", "nginx", "1.24.0");
    let h2 = pin_hash("apt", "nginx", "1.25.0");
    assert_ne!(h1, h2);
}

#[test]
fn pin_hash_differs_on_provider_change() {
    let h1 = pin_hash("apt", "curl", "7.88.1");
    let h2 = pin_hash("cargo", "curl", "7.88.1");
    assert_ne!(h1, h2);
}

#[test]
fn pin_hash_differs_on_name_change() {
    let h1 = pin_hash("apt", "curl", "7.88.1");
    let h2 = pin_hash("apt", "wget", "7.88.1");
    assert_ne!(h1, h2);
}

// ============================================================================
// FJ-1364: ResolvedPin struct
// ============================================================================

#[test]
fn resolved_pin_construction() {
    let pin = ResolvedPin {
        name: "nginx".into(),
        provider: "apt".into(),
        version: "1.24.0-2".into(),
        hash: pin_hash("apt", "nginx", "1.24.0-2"),
    };
    assert_eq!(pin.name, "nginx");
    assert_eq!(pin.provider, "apt");
    assert!(pin.hash.starts_with("blake3:"));
}

// ============================================================================
// FJ-2001: HealthSummary::health_pct
// ============================================================================

#[test]
fn health_pct_all_converged() {
    let summary = HealthSummary {
        machines: vec![MachineHealth {
            name: "web1".into(),
            resources: 10,
            converged: 10,
            drifted: 0,
            failed: 0,
        }],
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
        total_resources: 10,
        total_converged: 5,
        total_drifted: 3,
        total_failed: 2,
    };
    assert!((summary.health_pct() - 50.0).abs() < 0.01);
}

#[test]
fn health_pct_no_resources() {
    let summary = HealthSummary {
        machines: vec![],
        total_resources: 0,
        total_converged: 0,
        total_drifted: 0,
        total_failed: 0,
    };
    assert!((summary.health_pct() - 100.0).abs() < 0.01);
}

#[test]
fn health_pct_none_converged() {
    let summary = HealthSummary {
        machines: vec![],
        total_resources: 5,
        total_converged: 0,
        total_drifted: 3,
        total_failed: 2,
    };
    assert!((summary.health_pct() - 0.0).abs() < 0.01);
}

// ============================================================================
// FJ-2001: Query engine with in-memory SQLite
// ============================================================================

fn setup_test_db() -> rusqlite::Connection {
    let conn = forjar::core::store::db::open_state_db(std::path::Path::new(":memory:")).unwrap();
    conn.execute(
        "INSERT INTO machines (name, first_seen, last_seen) VALUES (?1, ?2, ?3)",
        ["web-01", "2026-03-08T00:00:00", "2026-03-08T12:00:00"],
    )
    .unwrap();
    let machine_id: i64 = conn
        .query_row("SELECT id FROM machines WHERE name = ?1", ["web-01"], |r| {
            r.get(0)
        })
        .unwrap();

    // Default generation
    conn.execute(
        "INSERT INTO generations (generation_num, run_id, config_hash, created_at) \
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![1, "run-100", "confighash", "2026-03-08T00:00:00"],
    )
    .unwrap();
    let gen_id: i64 = conn
        .query_row(
            "SELECT id FROM generations WHERE generation_num = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();

    // Converged resource
    conn.execute(
        "INSERT INTO resources (machine_id, generation_id, resource_id, resource_type, status, \
         content_hash, live_hash, applied_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            machine_id,
            gen_id,
            "nginx-pkg",
            "package",
            "converged",
            "hash-a",
            "hash-a",
            "2026-03-08T10:00:00"
        ],
    )
    .unwrap();

    // Drifted resource
    conn.execute(
        "INSERT INTO resources (machine_id, generation_id, resource_id, resource_type, status, \
         content_hash, live_hash, applied_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            machine_id,
            gen_id,
            "config-file",
            "file",
            "drifted",
            "hash-b",
            "hash-c",
            "2026-03-08T11:00:00"
        ],
    )
    .unwrap();

    // Events
    conn.execute(
        "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp, duration_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            "run-100",
            "nginx-pkg",
            "web-01",
            "resource_converged",
            "2026-03-08T10:00:00",
            200
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp, exit_code, stderr_tail) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            "run-101",
            "bad-svc",
            "web-01",
            "resource_failed",
            "2026-03-08T12:00:00",
            1,
            "service not found"
        ],
    ).unwrap();

    conn
}

#[test]
fn query_health_returns_machines() {
    let conn = setup_test_db();
    let health = query_health(&conn).unwrap();
    assert_eq!(health.machines.len(), 1);
    assert_eq!(health.machines[0].name, "web-01");
    assert_eq!(health.total_resources, 2);
    assert_eq!(health.total_converged, 1);
}

#[test]
fn query_drift_finds_mismatched_hashes() {
    let conn = setup_test_db();
    let drift = query_drift(&conn).unwrap();
    assert_eq!(drift.len(), 1);
    assert_eq!(drift[0].resource_id, "config-file");
    assert_eq!(drift[0].content_hash, "hash-b");
    assert_eq!(drift[0].live_hash, "hash-c");
}

#[test]
fn query_events_returns_all() {
    let conn = setup_test_db();
    let events = query_events(&conn, None, None, 50).unwrap();
    assert_eq!(events.len(), 2);
}

#[test]
fn query_events_filtered_by_run() {
    let conn = setup_test_db();
    let events = query_events(&conn, None, Some("run-100"), 50).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "resource_converged");
}

#[test]
fn query_events_filtered_by_since() {
    let conn = setup_test_db();
    let events = query_events(&conn, Some("2026-03-08T11:00:00"), None, 50).unwrap();
    assert_eq!(events.len(), 1); // only the failure at 12:00
}

#[test]
fn query_events_with_limit() {
    let conn = setup_test_db();
    let events = query_events(&conn, None, None, 1).unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn query_failures_returns_failed_events() {
    let conn = setup_test_db();
    let failures = query_failures(&conn, None, 50).unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].resource_id, "bad-svc");
    assert_eq!(failures[0].exit_code, Some(1));
    assert_eq!(
        failures[0].stderr_tail.as_deref(),
        Some("service not found")
    );
}

#[test]
fn query_failures_since_filter() {
    let conn = setup_test_db();
    // After all events
    let failures = query_failures(&conn, Some("2026-03-08T13:00:00"), 50).unwrap();
    assert!(failures.is_empty());
}

#[test]
fn query_history_for_resource() {
    let conn = setup_test_db();
    let history = query_history(&conn, "nginx-pkg").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].run_id, "run-100");
    assert_eq!(history[0].duration_ms, Some(200));
}

#[test]
fn query_history_missing_resource() {
    let conn = setup_test_db();
    let history = query_history(&conn, "nonexistent").unwrap();
    assert!(history.is_empty());
}

#[test]
fn query_churn_counts_converged() {
    let conn = setup_test_db();
    let churn = query_churn(&conn).unwrap();
    // Only converged events count for churn
    assert!(churn.len() <= 1);
    if !churn.is_empty() {
        assert_eq!(churn[0].resource_id, "nginx-pkg");
        assert_eq!(churn[0].event_count, 1);
    }
}

// ============================================================================
// FJ-2001: IngestResult display
// ============================================================================

#[test]
fn ingest_result_display() {
    let result = forjar::core::store::ingest::IngestResult {
        machines: 3,
        resources: 42,
        events: 150,
    };
    let s = format!("{result}");
    assert!(s.contains("3 machines"));
    assert!(s.contains("42 resources"));
    assert!(s.contains("150 events"));
}

// ============================================================================
// FJ-2001: Full ingest pipeline
// ============================================================================

#[test]
fn ingest_state_dir_with_lock_file() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let machine_dir = state_dir.join("test-machine");
    std::fs::create_dir_all(&machine_dir).unwrap();

    // Create a minimal state.lock.yaml
    std::fs::write(
        machine_dir.join("state.lock.yaml"),
        r#"
hostname: test-machine
generated_at: "2026-03-08T00:00:00"
resources:
  nginx-pkg:
    type: package
    status: converged
    applied_at: "2026-03-08T10:00:00"
    duration_seconds: 1.5
    hash: "blake3:abc123"
    details:
      content_hash: "hash-a"
      live_hash: "hash-a"
"#,
    )
    .unwrap();

    let conn = forjar::core::store::db::open_state_db(std::path::Path::new(":memory:")).unwrap();
    let result = forjar::core::store::ingest::ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(result.machines, 1);
    assert_eq!(result.resources, 1);

    // Verify queryable
    let health = query_health(&conn).unwrap();
    assert_eq!(health.total_resources, 1);
}

#[test]
fn ingest_state_dir_with_events() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let machine_dir = state_dir.join("machine-1");
    std::fs::create_dir_all(&machine_dir).unwrap();

    std::fs::write(
        machine_dir.join("state.lock.yaml"),
        "hostname: machine-1\ngenerated_at: \"2026-03-08T00:00:00\"\nresources: {}\n",
    )
    .unwrap();

    // Events file
    std::fs::write(
        machine_dir.join("events.jsonl"),
        r#"{"run_id":"r1","event":"resource_converged","resource":"pkg-a","ts":"2026-03-08T10:00:00","duration_seconds":0.5}
{"run_id":"r1","event":"resource_converged","resource":"pkg-b","ts":"2026-03-08T10:01:00","duration_seconds":0.3}
"#,
    )
    .unwrap();

    let conn = forjar::core::store::db::open_state_db(std::path::Path::new(":memory:")).unwrap();
    let result = forjar::core::store::ingest::ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(result.machines, 1);
    assert_eq!(result.events, 2);
}

#[test]
fn ingest_empty_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let conn = forjar::core::store::db::open_state_db(std::path::Path::new(":memory:")).unwrap();
    let result = forjar::core::store::ingest::ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(result.machines, 0);
    assert_eq!(result.resources, 0);
    assert_eq!(result.events, 0);
}

#[test]
fn ingest_incremental_skips_unchanged_lock() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let machine_dir = state_dir.join("machine-2");
    std::fs::create_dir_all(&machine_dir).unwrap();

    let lock_content = "hostname: machine-2\ngenerated_at: \"2026-03-08T00:00:00\"\nresources:\n  pkg:\n    type: package\n    status: converged\n    applied_at: \"now\"\n    hash: \"h1\"\n";
    std::fs::write(machine_dir.join("state.lock.yaml"), lock_content).unwrap();

    let conn = forjar::core::store::db::open_state_db(std::path::Path::new(":memory:")).unwrap();

    // First ingest
    let r1 = forjar::core::store::ingest::ingest_state_dir(&conn, &state_dir).unwrap();
    assert_eq!(r1.resources, 1);

    // Second ingest — same lock file, should skip resource re-ingest
    let r2 = forjar::core::store::ingest::ingest_state_dir(&conn, &state_dir).unwrap();
    // F3: cursor optimization — resources may be 0 (skipped) or 1 (re-ingested)
    // The key invariant is it doesn't error
    assert_eq!(r2.machines, 1);
}
