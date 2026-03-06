//! Tests for FJ-2001 SQLite state database.

#[cfg(test)]
mod tests {
    use crate::core::store::db::*;
    use tempfile::NamedTempFile;

    fn temp_db() -> (rusqlite::Connection, NamedTempFile) {
        let f = NamedTempFile::new().unwrap();
        let conn = open_state_db(f.path()).unwrap();
        (conn, f)
    }

    #[test]
    fn open_creates_tables() {
        let (conn, _f) = temp_db();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(tables.contains(&"machines".to_string()));
        assert!(tables.contains(&"generations".to_string()));
        assert!(tables.contains(&"resources".to_string()));
        assert!(tables.contains(&"events".to_string()));
        assert!(tables.contains(&"run_logs".to_string()));
        assert!(tables.contains(&"destroy_log".to_string()));
        assert!(tables.contains(&"drift_findings".to_string()));
        assert!(tables.contains(&"ingest_cursor".to_string()));
    }

    #[test]
    fn wal_mode_enabled() {
        let (conn, _f) = temp_db();
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
    }

    #[test]
    fn schema_version_roundtrip() {
        let (conn, _f) = temp_db();
        set_schema_version(&conn, SCHEMA_VERSION).unwrap();
        let v = schema_version(&conn).unwrap();
        assert_eq!(v, SCHEMA_VERSION);
    }

    #[test]
    fn insert_and_query_machine() {
        let (conn, _f) = temp_db();
        conn.execute(
            "INSERT INTO machines (name, hostname, transport, first_seen, last_seen) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["intel", "intel.local", "ssh", "2026-01-01", "2026-03-06"],
        )
        .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM machines", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn insert_generation() {
        let (conn, _f) = temp_db();
        conn.execute(
            "INSERT INTO generations (generation_num, run_id, config_hash, created_at) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![1, "r-abc123", "blake3:deadbeef", "2026-03-06T12:00:00Z"],
        )
        .unwrap();
        let gen: i64 = conn
            .query_row(
                "SELECT generation_num FROM generations WHERE run_id = ?1",
                ["r-abc123"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(gen, 1);
    }

    #[test]
    fn fts5_search_empty_db() {
        let (conn, _f) = temp_db();
        let results = fts5_search(&conn, "bash", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn fts5_search_with_data() {
        let (conn, _f) = temp_db();
        // Insert a machine
        conn.execute(
            "INSERT INTO machines (name, hostname, transport, first_seen, last_seen) \
             VALUES ('m1', 'host', 'local', '2026-01-01', '2026-03-06')",
            [],
        )
        .unwrap();
        // Insert a generation
        conn.execute(
            "INSERT INTO generations (generation_num, run_id, config_hash, created_at) \
             VALUES (1, 'r-1', 'hash1', '2026-03-06')",
            [],
        )
        .unwrap();
        // Insert a resource
        conn.execute(
            "INSERT INTO resources (resource_id, machine_id, generation_id, resource_type, \
             status, applied_at, path) VALUES ('bash-aliases', 1, 1, 'file', 'converged', \
             '2026-03-06', '/home/user/.bash_aliases')",
            [],
        )
        .unwrap();
        // Rebuild FTS from content table
        conn.execute(
            "INSERT INTO resources_fts(resources_fts) VALUES('rebuild')",
            [],
        )
        .unwrap();
        let results = fts5_search(&conn, "bash", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resource_id, "bash-aliases");
        assert_eq!(results[0].resource_type, "file");
    }

    #[test]
    fn open_idempotent() {
        let f = NamedTempFile::new().unwrap();
        let _conn1 = open_state_db(f.path()).unwrap();
        // Opening again should not fail (IF NOT EXISTS)
        let _conn2 = open_state_db(f.path()).unwrap();
    }

    #[test]
    fn indexes_exist() {
        let (conn, _f) = temp_db();
        let indexes: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(indexes.contains(&"idx_resources_machine".to_string()));
        assert!(indexes.contains(&"idx_resources_gen".to_string()));
        assert!(indexes.contains(&"idx_resources_status".to_string()));
        assert!(indexes.contains(&"idx_events_run".to_string()));
        assert!(indexes.contains(&"idx_destroy_machine".to_string()));
        assert!(indexes.contains(&"idx_drift_machine".to_string()));
    }

    #[test]
    fn list_all_resources_empty() {
        let (conn, _f) = temp_db();
        let results = list_all_resources(&conn, 50).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn list_all_resources_returns_all() {
        let (conn, _f) = temp_db();
        conn.execute(
            "INSERT INTO machines (name, hostname, transport, first_seen, last_seen) \
             VALUES ('m1', 'host', 'local', '2026-01-01', '2026-03-06')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO generations (generation_num, run_id, config_hash, created_at) \
             VALUES (1, 'r-1', 'hash1', '2026-03-06')",
            [],
        ).unwrap();
        for (id, rtype) in [("nginx-cfg", "file"), ("bash-pkg", "package")] {
            conn.execute(
                "INSERT INTO resources (resource_id, machine_id, generation_id, resource_type, \
                 status, applied_at, path) VALUES (?1, 1, 1, ?2, 'converged', '2026-03-06', '/tmp')",
                rusqlite::params![id, rtype],
            ).unwrap();
        }
        let results = list_all_resources(&conn, 50).unwrap();
        assert_eq!(results.len(), 2);
        // Ordered alphabetically by resource_id
        assert_eq!(results[0].resource_id, "bash-pkg");
        assert_eq!(results[1].resource_id, "nginx-cfg");
        assert_eq!(results[0].rank, 0.0);
    }
}
