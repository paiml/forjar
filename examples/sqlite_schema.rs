//! Demonstrates FJ-2001 SQLite schema types: config, DDL, ingest cursor, query enrichments.

use forjar::core::types::{IngestCursor, IngestResult, QueryEnrichments, SchemaV1, SqliteConfig};

fn main() {
    // SQLite configuration
    println!("=== SQLite Config ===");
    let config = SqliteConfig::default();
    println!("  DB path: {}", config.db_path());
    println!("  WAL mode: {}", config.wal_mode);
    println!("  FTS5: {}", config.fts5);
    println!("  Cache size: {} pages", config.cache_size);
    println!("  Pragmas:");
    for p in config.pragma_statements() {
        println!("    {p}");
    }

    let custom = SqliteConfig {
        state_dir: "/opt/forjar/state".into(),
        wal_mode: true,
        ..Default::default()
    };
    println!("  Custom DB path: {}", custom.db_path());

    // Schema DDL
    println!("\n=== Schema V1 (version {}) ===", SchemaV1::VERSION);
    let ddl = SchemaV1::all_ddl();
    println!("  Total DDL statements: {}", ddl.len());
    for (i, stmt) in ddl.iter().enumerate() {
        let first_line = stmt.trim().lines().next().unwrap_or("(empty)");
        println!("  [{i}] {first_line}");
    }

    // Indexes
    println!("\n=== Indexes ===");
    for idx in SchemaV1::INDEXES {
        println!("  {idx}");
    }

    // Ingest cursor
    println!("\n=== Ingest Cursor ===");
    let mut cursor = IngestCursor::default();
    println!("  intel gen 1 ingested? {}", cursor.is_ingested("intel", 1));
    cursor.mark_ingested("intel", 3, 42);
    cursor.mark_ingested("jetson", 1, 15);
    println!("  After marking intel=3, jetson=1:");
    println!("    intel gen 1? {}", cursor.is_ingested("intel", 1));
    println!("    intel gen 3? {}", cursor.is_ingested("intel", 3));
    println!("    intel gen 4? {}", cursor.is_ingested("intel", 4));
    println!("    jetson gen 1? {}", cursor.is_ingested("jetson", 1));
    println!("    Total ingested: {}", cursor.total_ingested);

    // Ingest result
    println!("\n=== Ingest Result ===");
    let result = IngestResult {
        resources_upserted: 120,
        generations_ingested: 8,
        run_logs_ingested: 45,
        duration_secs: 1.23,
        machines: vec!["intel".into(), "jetson".into(), "rpi4".into()],
    };
    println!("  {result}");

    // Query enrichments
    println!("\n=== Query Enrichments ===");
    let empty = QueryEnrichments::default();
    println!("  Default any_enabled: {}", empty.any_enabled());

    let enriched = QueryEnrichments {
        history: true,
        drift: true,
        timing: true,
        git_history: true,
        ..Default::default()
    };
    println!(
        "  With history+drift+timing+git: {}",
        enriched.any_enabled()
    );
}
