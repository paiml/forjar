//! U1/U2 benchmark tests — validate aspirational performance targets.

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

    /// U1: FTS5 query should complete in <50ms for a 3-machine, 40-resource dataset.
    #[test]
    fn query_latency_under_50ms() {
        let (conn, _f) = temp_db();
        let dir = TempDir::new().unwrap();

        // Create 3 machines with ~13 resources each (40 total)
        for (i, machine) in ["web1", "db1", "cache1"].iter().enumerate() {
            let mdir = dir.path().join(machine);
            std::fs::create_dir(&mdir).unwrap();

            let mut resources = String::new();
            for j in 0..13 {
                resources.push_str(&format!(
                    "  res-{i}-{j}:\n    type: {}\n    status: converged\n    \
                     applied_at: 2026-03-06\n    duration_seconds: 0.1\n    hash: blake3:aa{i}{j}\n    \
                     details:\n      path: /etc/app/config-{i}-{j}.yaml\n",
                    if j % 3 == 0 { "package" } else { "file" }
                ));
            }

            let lock = format!(
                "schema: '1.0'\nmachine: {machine}\nhostname: {machine}.local\n\
                 generated_at: 2026-03-06T12:00:00Z\nresources:\n{resources}"
            );
            std::fs::write(mdir.join("state.lock.yaml"), lock).unwrap();
            std::fs::write(mdir.join("events.jsonl"), "").unwrap();
        }

        ingest_state_dir(&conn, dir.path()).unwrap();

        let start = std::time::Instant::now();
        let results = fts5_search(&conn, "config", 50).unwrap();
        let elapsed = start.elapsed();

        assert!(!results.is_empty(), "should find results matching 'config'");
        assert!(
            elapsed.as_millis() < 50,
            "FTS5 query took {}ms (target: <50ms)",
            elapsed.as_millis()
        );
    }

    /// U2: state.db should be <1MB for a 3-machine stack.
    #[test]
    fn state_db_size_under_1mb() {
        let f = NamedTempFile::new().unwrap();
        let conn = open_state_db(f.path()).unwrap();
        let dir = TempDir::new().unwrap();

        for machine in ["web1", "db1", "cache1"] {
            let mdir = dir.path().join(machine);
            std::fs::create_dir(&mdir).unwrap();

            let mut resources = String::new();
            for j in 0..20 {
                resources.push_str(&format!(
                    "  {machine}-res-{j}:\n    type: file\n    status: converged\n    \
                     applied_at: 2026-03-06\n    duration_seconds: 0.5\n    hash: blake3:bb{j}\n    \
                     details:\n      path: /etc/{machine}/config-{j}.conf\n      \
                     content_hash: blake3:cc{j}\n      live_hash: blake3:cc{j}\n",
                ));
            }

            let lock = format!(
                "schema: '1.0'\nmachine: {machine}\nhostname: {machine}.local\n\
                 generated_at: 2026-03-06T12:00:00Z\nresources:\n{resources}"
            );
            std::fs::write(mdir.join("state.lock.yaml"), lock).unwrap();

            // 100 events per machine
            let mut events = String::new();
            for j in 0..100 {
                events.push_str(&format!(
                    "{{\"ts\":\"2026-03-06T12:{j:02}:00Z\",\"event\":\"resource_converged\",\
                     \"machine\":\"{machine}\",\"resource\":\"{machine}-res-{}\",\"run_id\":\"r-{j}\",\
                     \"duration_seconds\":0.{j}}}\n",
                    j % 20
                ));
            }
            std::fs::write(mdir.join("events.jsonl"), events).unwrap();
        }

        ingest_state_dir(&conn, dir.path()).unwrap();
        drop(conn);

        let size = std::fs::metadata(f.path()).unwrap().len();
        assert!(
            size < 1_048_576,
            "state.db is {} bytes (target: <1MB)",
            size
        );
    }
}
