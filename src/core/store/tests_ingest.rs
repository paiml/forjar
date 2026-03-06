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
        // Second ingest should replace, not duplicate
        let result2 = ingest_state_dir(&conn, state_dir.path()).unwrap();
        assert_eq!(result2.resources, 2);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM resources", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
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
        let result = IngestResult { machines: 3, resources: 25, events: 100 };
        assert_eq!(result.to_string(), "Ingested 3 machines, 25 resources, 100 events");
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
}
