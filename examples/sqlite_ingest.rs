//! FJ-2001: SQLite ingest + FTS5 search — real state data demo.
//!
//! ```bash
//! cargo run --example sqlite_ingest
//! ```

use forjar::core::store::db;
use forjar::core::store::ingest;
use std::path::Path;

fn main() {
    println!(
        "=== FJ-2001: SQLite Ingest Pipeline (Schema v{}) ===\n",
        db::SCHEMA_VERSION
    );

    // Open in-memory database
    let conn = db::open_state_db(Path::new(":memory:")).expect("open db");
    println!("Database opened (in-memory, WAL mode)");

    // Show schema tables
    show_schema_tables(&conn);

    // Check if real state dir exists
    let state_dir = Path::new("state");
    if state_dir.is_dir() {
        let result = ingest::ingest_state_dir(&conn, state_dir).expect("ingest");
        println!("\nIngest: {result}");

        // FTS5 search demo (porter tokenizer, no raw JSON)
        println!("\n=== FTS5 Search (porter tokenizer) ===");
        for query in ["bash", "package", "converged", "config"] {
            let results = db::fts5_search(&conn, query, 5).expect("search");
            println!("\nQuery \"{query}\": {} result(s)", results.len());
            for r in &results {
                println!(
                    "  {:20} {:10} {:10} {}",
                    r.resource_id,
                    r.resource_type,
                    r.status,
                    r.path.as_deref().unwrap_or("—")
                );
            }
        }

        // Health summary
        let health = ingest::query_health(&conn).expect("health");
        println!("\n=== Health Summary ===");
        println!(
            " {:10} {:>10} {:>10} {:>8} {:>8}",
            "MACHINE", "RESOURCES", "CONVERGED", "DRIFTED", "FAILED"
        );
        for m in &health.machines {
            println!(
                " {:10} {:>10} {:>10} {:>8} {:>8}",
                m.name, m.resources, m.converged, m.drifted, m.failed
            );
        }
        println!("\nStack health: {:.0}%", health.health_pct());

        // Destroy log query
        show_destroy_log(&conn);

        // Drift findings query
        show_drift_findings(&conn);
    } else {
        println!("No state/ directory found — run `forjar apply` first");
    }

    // Schema version
    let v = db::schema_version(&conn).unwrap_or(0);
    println!("\nSchema version: {v}");
}

fn show_schema_tables(conn: &rusqlite::Connection) {
    let mut stmt = conn
        .prepare("SELECT name, type FROM sqlite_master WHERE type IN ('table', 'index') ORDER BY type, name")
        .expect("prepare");
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query")
        .filter_map(|r| r.ok())
        .collect();

    let tables: Vec<_> = rows.iter().filter(|(_, t)| t == "table").collect();
    let indexes: Vec<_> = rows.iter().filter(|(_, t)| t == "index").collect();
    println!("\nSchema objects:");
    println!(
        "  Tables ({}): {}",
        tables.len(),
        tables
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "  Indexes ({}): {}",
        indexes.len(),
        indexes
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

fn show_destroy_log(conn: &rusqlite::Connection) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM destroy_log", [], |r| r.get(0))
        .unwrap_or(0);
    println!("\n=== Destroy Log: {count} entries ===");
    if count > 0 {
        let mut stmt = conn
            .prepare(
                "SELECT resource_id, resource_type, destroyed_at \
                 FROM destroy_log ORDER BY destroyed_at DESC LIMIT 5",
            )
            .expect("prepare");
        let rows: Vec<(String, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("query")
            .filter_map(|r| r.ok())
            .collect();
        for (rid, rtype, ts) in &rows {
            println!("  - {rid} ({rtype}) destroyed at {ts}");
        }
    }
}

fn show_drift_findings(conn: &rusqlite::Connection) {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM drift_findings", [], |r| r.get(0))
        .unwrap_or(0);
    println!("\n=== Drift Findings: {count} entries ===");
    if count > 0 {
        let mut stmt = conn
            .prepare(
                "SELECT resource_id, resource_type, detail \
                 FROM drift_findings WHERE resolved_at IS NULL LIMIT 5",
            )
            .expect("prepare");
        let rows: Vec<(String, String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("query")
            .filter_map(|r| r.ok())
            .collect();
        for (rid, rtype, detail) in &rows {
            println!("  ! {rid} ({rtype}): {detail}");
        }
    }
}
