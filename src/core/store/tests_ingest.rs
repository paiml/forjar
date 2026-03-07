//! Tests for FJ-2001 ingest pipeline.

#[cfg(test)]
mod tests {
    use crate::core::store::db::*;
    use crate::core::store::ingest::*;
    use tempfile::{NamedTempFile, TempDir};

    fn temp_db() -> (rusqlite::Connection, NamedTempFile) {
        let f = NamedTempFile::new().unwrap();
        let conn = open_state_db(f.path()).unwrap();
        (conn, f)
    }

    fn create_state_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        let machine_dir = dir.path().join("test-machine");
        std::fs::create_dir(&machine_dir).unwrap();

        let lock = r#"
schema: '1.0'
machine: test-machine
hostname: test.local
generated_at: 2026-03-06T12:00:00Z
generator: forjar 1.0.0
resources:
  bash-aliases:
    type: file
    status: converged
    applied_at: 2026-03-06T12:00:00Z
    duration_seconds: 0.54
    hash: blake3:deadbeef
    details:
      path: /home/user/.bash_aliases
      content_hash: blake3:aabbccdd
      live_hash: blake3:eeff0011
  cargo-tools:
    type: package
    status: converged
    applied_at: 2026-03-06T12:01:00Z
    duration_seconds: 0.85
    hash: blake3:cafebabe
    details:
      live_hash: blake3:12345678
"#;
        std::fs::write(machine_dir.join("state.lock.yaml"), lock).unwrap();

        let events = r#"{"ts":"2026-03-06T12:00:00Z","event":"apply_started","machine":"test-machine","run_id":"r-test123"}
{"ts":"2026-03-06T12:00:00Z","event":"resource_converged","machine":"test-machine","resource":"bash-aliases","run_id":"r-test123","duration_seconds":0.54}
{"ts":"2026-03-06T12:01:00Z","event":"resource_converged","machine":"test-machine","resource":"cargo-tools","run_id":"r-test123","duration_seconds":0.85}
{"ts":"2026-03-06T12:01:01Z","event":"apply_completed","machine":"test-machine","run_id":"r-test123"}
"#;
        std::fs::write(machine_dir.join("events.jsonl"), events).unwrap();
        dir
    }

    #[test]
    fn ingest_creates_machine() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        let result = ingest_state_dir(&conn, state_dir.path()).unwrap();
        assert_eq!(result.machines, 1);

        let name: String = conn
            .query_row("SELECT name FROM machines WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "test-machine");
    }

    #[test]
    fn ingest_creates_resources() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        let result = ingest_state_dir(&conn, state_dir.path()).unwrap();
        assert_eq!(result.resources, 2);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM resources", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn ingest_creates_events() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        let result = ingest_state_dir(&conn, state_dir.path()).unwrap();
        assert_eq!(result.events, 4);
    }

    #[test]
    fn ingest_populates_fts() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        ingest_state_dir(&conn, state_dir.path()).unwrap();

        let results = fts5_search(&conn, "bash", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource_id, "bash-aliases");
        assert_eq!(results[0].resource_type, "file");
        assert_eq!(results[0].status, "converged");
        assert_eq!(results[0].path.as_deref(), Some("/home/user/.bash_aliases"));
    }

    #[test]
    fn ingest_fts_search_package() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        ingest_state_dir(&conn, state_dir.path()).unwrap();

        let results = fts5_search(&conn, "cargo", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource_id, "cargo-tools");
        assert_eq!(results[0].resource_type, "package");
    }

    #[test]
    fn ingest_resource_details() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        ingest_state_dir(&conn, state_dir.path()).unwrap();

        let (hash, duration): (Option<String>, f64) = conn
            .query_row(
                "SELECT state_hash, duration_secs FROM resources WHERE resource_id = 'bash-aliases'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(hash.as_deref(), Some("blake3:deadbeef"));
        assert!((duration - 0.54).abs() < 0.001);
    }

    #[test]
    fn ingest_idempotent() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        ingest_state_dir(&conn, state_dir.path()).unwrap();
        // F3: Second ingest skips unchanged lock files (cursor optimization)
        let result2 = ingest_state_dir(&conn, state_dir.path()).unwrap();
        assert_eq!(
            result2.resources, 0,
            "unchanged lock file should be skipped"
        );

        // DB still has all resources from first ingest
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM resources", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn ingest_cursor_incremental() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();

        // First ingest: all resources
        let r1 = ingest_state_dir(&conn, state_dir.path()).unwrap();
        assert_eq!(r1.resources, 2);

        // Second ingest: skipped (lock unchanged)
        let r2 = ingest_state_dir(&conn, state_dir.path()).unwrap();
        assert_eq!(r2.resources, 0, "unchanged lock should be skipped");

        // Modify lock file to trigger re-ingest
        let lock_path = state_dir
            .path()
            .join("test-machine")
            .join("state.lock.yaml");
        let mut content = std::fs::read_to_string(&lock_path).unwrap();
        content.push_str("\n  new-resource:\n    type: package\n    status: converged\n");
        std::fs::write(&lock_path, content).unwrap();

        // Third ingest: re-ingests modified lock file
        let r3 = ingest_state_dir(&conn, state_dir.path()).unwrap();
        assert!(r3.resources > 0, "modified lock should trigger re-ingest");
    }

    #[test]
    fn ingest_skips_non_directories() {
        let (conn, _f) = temp_db();
        let dir = TempDir::new().unwrap();
        // Create a regular file (not a machine dir)
        std::fs::write(dir.path().join("forjar.lock.yaml"), "schema: '1.0'").unwrap();
        let result = ingest_state_dir(&conn, dir.path()).unwrap();
        assert_eq!(result.machines, 0);
    }

    #[test]
    fn ingest_skips_dir_without_lock() {
        let (conn, _f) = temp_db();
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join("empty-machine")).unwrap();
        let result = ingest_state_dir(&conn, dir.path()).unwrap();
        assert_eq!(result.machines, 0);
    }

    #[test]
    fn health_summary_from_ingested_data() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        ingest_state_dir(&conn, state_dir.path()).unwrap();

        let health = query_health(&conn).unwrap();
        assert_eq!(health.machines.len(), 1);
        assert_eq!(health.total_resources, 2);
        assert_eq!(health.total_converged, 2);
        assert_eq!(health.total_drifted, 0);
        assert_eq!(health.total_failed, 0);
        assert!((health.health_pct() - 100.0).abs() < 0.01);
    }

    #[test]
    fn health_empty_db() {
        let (conn, _f) = temp_db();
        let health = query_health(&conn).unwrap();
        assert!(health.machines.is_empty());
        assert_eq!(health.total_resources, 0);
        assert!((health.health_pct() - 100.0).abs() < 0.01);
    }

    #[test]
    fn ingest_result_display() {
        let result = IngestResult {
            machines: 3,
            resources: 25,
            events: 100,
        };
        assert_eq!(
            result.to_string(),
            "Ingested 3 machines, 25 resources, 100 events"
        );
    }

    #[test]
    fn multi_machine_ingest() {
        let (conn, _f) = temp_db();
        let dir = TempDir::new().unwrap();

        for name in ["alpha", "beta"] {
            let mdir = dir.path().join(name);
            std::fs::create_dir(&mdir).unwrap();
            let lock = format!(
                "schema: '1.0'\nmachine: {name}\nhostname: {name}.local\n\
                 generated_at: 2026-03-06T12:00:00Z\nresources:\n  pkg-{name}:\n    \
                 type: package\n    status: converged\n    applied_at: 2026-03-06T12:00:00Z\n    \
                 duration_seconds: 0.1\n    hash: blake3:aabb\n"
            );
            std::fs::write(mdir.join("state.lock.yaml"), lock).unwrap();
            std::fs::write(mdir.join("events.jsonl"), "").unwrap();
        }

        let result = ingest_state_dir(&conn, dir.path()).unwrap();
        assert_eq!(result.machines, 2);
        assert_eq!(result.resources, 2);

        let health = query_health(&conn).unwrap();
        assert_eq!(health.machines.len(), 2);
    }

    #[test]
    fn query_history_returns_events() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        ingest_state_dir(&conn, state_dir.path()).unwrap();

        let history = query_history(&conn, "bash-aliases").unwrap();
        assert!(!history.is_empty());
        assert_eq!(history[0].event_type, "resource_converged");
    }

    #[test]
    fn query_history_empty_resource() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        ingest_state_dir(&conn, state_dir.path()).unwrap();

        let history = query_history(&conn, "nonexistent").unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn query_drift_detects_hash_mismatch() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        ingest_state_dir(&conn, state_dir.path()).unwrap();

        // bash-aliases has different content_hash and live_hash in our test fixture
        let drift = query_drift(&conn).unwrap();
        assert!(!drift.is_empty());
        let ba = drift.iter().find(|d| d.resource_id == "bash-aliases");
        assert!(ba.is_some());
    }

    #[test]
    fn query_churn_counts_events() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        ingest_state_dir(&conn, state_dir.path()).unwrap();

        let churn = query_churn(&conn).unwrap();
        assert!(!churn.is_empty());
        let ba = churn.iter().find(|c| c.resource_id == "bash-aliases");
        assert!(ba.is_some());
        assert!(ba.unwrap().event_count >= 1);
    }

    #[test]
    fn ingest_generations_from_dir() {
        let (conn, _f) = temp_db();
        let dir = TempDir::new().unwrap();

        // Create machine dir with lock file
        let mdir = dir.path().join("test-machine");
        std::fs::create_dir(&mdir).unwrap();
        std::fs::write(
            mdir.join("state.lock.yaml"),
            "schema: '1.0'\nmachine: test\nhostname: test\ngenerated_at: 2026-03-06\nresources:\n",
        )
        .unwrap();

        // Create generations directory
        let gens = dir.path().join("generations");
        std::fs::create_dir(&gens).unwrap();
        std::fs::write(
            gens.join("gen-1.yaml"),
            "\
generation: 1\nrun_id: r-gen1\nconfig_hash: blake3:aabb\n\
created_at: 2026-03-06T12:00:00Z\ngit_ref: abc123\naction: apply\n",
        )
        .unwrap();
        std::fs::write(
            gens.join("gen-2.yaml"),
            "\
generation: 2\nrun_id: r-gen2\nconfig_hash: blake3:ccdd\n\
created_at: 2026-03-06T13:00:00Z\naction: rollback\n",
        )
        .unwrap();

        ingest_state_dir(&conn, dir.path()).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM generations WHERE run_id != 'ingest'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn query_drift_empty_db() {
        let (conn, _f) = temp_db();
        let drift = query_drift(&conn).unwrap();
        assert!(drift.is_empty());
    }

    #[test]
    fn query_churn_empty_db() {
        let (conn, _f) = temp_db();
        let churn = query_churn(&conn).unwrap();
        assert!(churn.is_empty());
    }

    #[test]
    fn ingest_destroy_log() {
        let (conn, _f) = temp_db();
        let dir = TempDir::new().unwrap();

        let mdir = dir.path().join("test-machine");
        std::fs::create_dir(&mdir).unwrap();
        std::fs::write(
            mdir.join("state.lock.yaml"),
            "schema: '1.0'\nmachine: test-machine\nhostname: test\ngenerated_at: 2026-03-06\n\
             resources:\n  nginx:\n    type: package\n    status: converged\n    \
             applied_at: 2026-03-06\n    duration_seconds: 0.1\n    hash: blake3:aabb\n",
        )
        .unwrap();
        std::fs::write(mdir.join("events.jsonl"), "").unwrap();

        let destroy_log = r#"{"timestamp":"2026-03-06T12:00:00Z","machine":"test-machine","resource_id":"old-pkg","resource_type":"package","pre_hash":"abc123","generation":1,"reliable_recreate":true}
{"timestamp":"2026-03-06T12:01:00Z","machine":"test-machine","resource_id":"old-svc","resource_type":"service","pre_hash":"def456","generation":1,"reliable_recreate":false}
"#;
        std::fs::write(mdir.join("destroy-log.jsonl"), destroy_log).unwrap();

        ingest_state_dir(&conn, dir.path()).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM destroy_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let rid: String = conn
            .query_row(
                "SELECT resource_id FROM destroy_log ORDER BY id LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rid, "old-pkg");
    }

    #[test]
    fn ingest_destroy_log_empty() {
        let (conn, _f) = temp_db();
        let dir = TempDir::new().unwrap();

        let mdir = dir.path().join("test-machine");
        std::fs::create_dir(&mdir).unwrap();
        std::fs::write(
            mdir.join("state.lock.yaml"),
            "schema: '1.0'\nmachine: test-machine\nhostname: test\ngenerated_at: 2026-03-06\n\
             resources:\n  pkg:\n    type: package\n    status: converged\n    \
             applied_at: 2026-03-06\n    duration_seconds: 0.1\n    hash: blake3:cc\n",
        )
        .unwrap();
        std::fs::write(mdir.join("events.jsonl"), "").unwrap();
        std::fs::write(mdir.join("destroy-log.jsonl"), "\n").unwrap();

        ingest_state_dir(&conn, dir.path()).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM destroy_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn ingest_packages_column() {
        let (conn, _f) = temp_db();
        let state_dir = create_state_dir();
        ingest_state_dir(&conn, state_dir.path()).unwrap();

        let pkgs: Option<String> = conn
            .query_row(
                "SELECT packages FROM resources WHERE resource_id = 'cargo-tools'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pkgs.as_deref(), Some("cargo-tools"));

        // file type should have NULL packages
        let file_pkgs: Option<String> = conn
            .query_row(
                "SELECT packages FROM resources WHERE resource_id = 'bash-aliases'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(file_pkgs.is_none());
    }
}
