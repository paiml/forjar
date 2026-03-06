//! FJ-2001: SQLite schema benchmark — validates performance targets.
//!
//! Demonstrates that the SQLite query engine meets aspirational targets:
//! - U1: FTS5 query < 50ms for 3 machines
//! - U2: state.db < 1MB for 3 machines
//!
//! ```bash
//! cargo run --example schema_benchmark
//! ```

use forjar::core::store::db;
use forjar::core::store::ingest;

fn main() {
    println!("=== SQLite Schema Benchmark (v{}) ===\n", db::SCHEMA_VERSION);

    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("bench.db");
    let state_dir = dir.path().join("state");

    // Build synthetic 3-machine state with ~50 resources total
    for (i, machine) in ["web-prod", "db-primary", "cache-hot"].iter().enumerate() {
        let mdir = state_dir.join(machine);
        std::fs::create_dir_all(&mdir).expect("mkdir");

        let mut resources = String::new();
        let count = if i == 0 { 25 } else { 13 };
        for j in 0..count {
            let rtype = match j % 4 {
                0 => "package",
                1 => "file",
                2 => "service",
                _ => "cron",
            };
            resources.push_str(&format!(
                "  {machine}-r{j}:\n    type: {rtype}\n    status: converged\n    \
                 applied_at: 2026-03-06T12:{j:02}:00Z\n    duration_seconds: 0.{j}\n    \
                 hash: blake3:bench{i}{j:04}\n    details:\n      \
                 path: /etc/{machine}/config-{j}.conf\n",
            ));
        }
        let lock = format!(
            "schema: '1.0'\nmachine: {machine}\nhostname: {machine}.internal\n\
             generated_at: 2026-03-06T12:00:00Z\nresources:\n{resources}"
        );
        std::fs::write(mdir.join("state.lock.yaml"), lock).expect("write lock");

        // 50 events per machine
        let mut events = String::new();
        for j in 0..50 {
            events.push_str(&format!(
                "{{\"ts\":\"2026-03-06T12:{j:02}:00Z\",\"event\":\"resource_converged\",\
                 \"machine\":\"{machine}\",\"resource\":\"{machine}-r{}\",\
                 \"run_id\":\"r-bench-{j}\",\"duration_seconds\":0.{j}}}\n",
                j % count
            ));
        }
        std::fs::write(mdir.join("events.jsonl"), events).expect("write events");
    }

    // Ingest
    let conn = db::open_state_db(&db_path).expect("open");
    let t0 = std::time::Instant::now();
    let result = ingest::ingest_state_dir(&conn, &state_dir).expect("ingest");
    let ingest_ms = t0.elapsed().as_millis();
    println!("Ingest: {result} ({ingest_ms}ms)");

    // U1: Query latency
    println!("\n--- U1: FTS5 Query Latency ---");
    for query in ["config", "package", "cache", "service"] {
        let t1 = std::time::Instant::now();
        let results = db::fts5_search(&conn, query, 50).expect("search");
        let query_us = t1.elapsed().as_micros();
        let pass = if query_us < 50_000 { "PASS" } else { "FAIL" };
        println!("  [{pass}] \"{query}\": {} results in {query_us}us (target: <50ms)", results.len());
    }

    // U2: DB size
    drop(conn);
    let size = std::fs::metadata(&db_path).expect("stat").len();
    let pass = if size < 1_048_576 { "PASS" } else { "FAIL" };
    println!("\n--- U2: Database Size ---");
    println!("  [{pass}] state.db = {} bytes ({:.1} KB, target: <1MB)", size, size as f64 / 1024.0);

    // Health check
    let conn = db::open_state_db(&db_path).expect("reopen");
    let health = ingest::query_health(&conn).expect("health");
    println!("\n--- Health ---");
    println!("  Machines: {}, Resources: {}, Health: {:.0}%",
        health.machines.len(), health.total_resources, health.health_pct());
}
