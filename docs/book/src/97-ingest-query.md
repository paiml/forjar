# State Ingest Pipeline & Query Engine

Falsification coverage for FJ-2001 (state database ingest and query functions).

## Ingest Pipeline (FJ-2001)

Parses machine state directories into SQLite for sub-second queries:

```rust
use forjar::core::store::db;
use forjar::core::store::ingest::ingest_state_dir;

let conn = db::open_state_db(db_path)?;
let result = ingest_state_dir(&conn, &state_dir)?;
println!("{result}"); // "Ingested 3 machines, 42 resources, 128 events"
```

### Directory Layout

```
state/
├── web-01/
│   ├── state.lock.yaml     # resource status, hashes, details
│   ├── events.jsonl         # run events (converged, failed, etc.)
│   └── destroy-log.jsonl    # resource destruction records
├── web-02/
│   └── ...
└── generations/
    └── gen-001.yaml         # generation metadata
```

### Incremental Ingest (F3)

Uses cursor tracking to avoid redundant work:

- Lock file hash comparison: skip re-ingest if unchanged
- Event offset tracking: resume from last ingested line
- Cursor stored per-machine in `ingest_cursor` table

## Query Engine

### Health Summary

```rust
let health = query_health(&conn)?;
println!("{:.1}%", health.health_pct());  // 95.0%
```

### Drift Detection

```rust
let drift = query_drift(&conn)?;
for d in &drift {
    println!("{} on {}: {} → {}", d.resource_id, d.machine,
        d.content_hash, d.live_hash);
}
```

### Event Filtering

```rust
let events = query_events(&conn, Some("2026-03-08T12:00:00Z"), Some("run-001"), 50)?;
let failures = query_failures(&conn, None, 10)?;
let churn = query_churn(&conn)?;
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_ingest_query.rs` | 23 | ~300 |
