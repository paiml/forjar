//! FJ-2001: SQLite ingest + FTS5 search — real state data demo.
//!
//! ```bash
//! cargo run --example sqlite_ingest
//! ```

use forjar::core::store::db;
use forjar::core::store::ingest;
use std::path::Path;

fn main() {
    println!("=== FJ-2001: SQLite Ingest Pipeline ===\n");

    // Open in-memory database
    let conn = db::open_state_db(Path::new(":memory:")).expect("open db");
    println!("Database opened (in-memory, WAL mode)");

    // Check if real state dir exists
    let state_dir = Path::new("state");
    if state_dir.is_dir() {
        let result = ingest::ingest_state_dir(&conn, state_dir).expect("ingest");
        println!("Ingest: {result}");

        // FTS5 search demo
        for query in ["bash", "package", "converged", "gitconfig"] {
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
    } else {
        println!("No state/ directory found — run `forjar apply` first");
        println!("Schema created with {} tables", 5);
    }

    // Schema version
    let v = db::schema_version(&conn).unwrap_or(0);
    println!("\nSchema version: {v}");
}
