//! FJ-2001: SQLite state database — schema, WAL, pragma tuning.
//!
//! Creates and manages `state.db` for sub-second queries across
//! machines, generations, resources, and events.

use rusqlite::{Connection, Result as SqlResult};

/// Schema version — bump when schema changes.
pub const SCHEMA_VERSION: u32 = 1;

/// SQL to create the state database schema.
const SCHEMA_SQL: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS machines (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    hostname    TEXT,
    transport   TEXT NOT NULL DEFAULT 'local',
    ssh_host    TEXT,
    ssh_user    TEXT,
    ssh_port    INTEGER DEFAULT 22,
    first_seen  TEXT NOT NULL,
    last_seen   TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active'
);

CREATE TABLE IF NOT EXISTS generations (
    id              INTEGER PRIMARY KEY,
    generation_num  INTEGER NOT NULL UNIQUE,
    run_id          TEXT NOT NULL,
    config_hash     TEXT NOT NULL,
    git_ref         TEXT,
    config_snapshot TEXT,
    operator        TEXT,
    created_at      TEXT NOT NULL,
    parent_gen      INTEGER REFERENCES generations(id),
    action          TEXT NOT NULL DEFAULT 'apply'
);

CREATE TABLE IF NOT EXISTS resources (
    id              INTEGER PRIMARY KEY,
    resource_id     TEXT NOT NULL,
    machine_id      INTEGER NOT NULL REFERENCES machines(id),
    generation_id   INTEGER NOT NULL REFERENCES generations(id),
    resource_type   TEXT NOT NULL,
    status          TEXT NOT NULL,
    state_hash      TEXT,
    content_hash    TEXT,
    live_hash       TEXT,
    applied_at      TEXT NOT NULL,
    duration_secs   REAL NOT NULL DEFAULT 0.0,
    details_json    TEXT NOT NULL DEFAULT '{}',
    path            TEXT,
    reversibility   TEXT NOT NULL DEFAULT 'reversible',
    UNIQUE(resource_id, machine_id, generation_id)
);

CREATE VIRTUAL TABLE IF NOT EXISTS resources_fts USING fts5(
    resource_id, resource_type, status, path, details_json,
    content='resources',
    content_rowid='id'
);

CREATE TABLE IF NOT EXISTS events (
    id          INTEGER PRIMARY KEY,
    run_id      TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    machine     TEXT NOT NULL,
    event_type  TEXT NOT NULL,
    timestamp   TEXT NOT NULL,
    duration_ms INTEGER,
    exit_code   INTEGER,
    stdout_tail TEXT,
    stderr_tail TEXT,
    details     TEXT
);

CREATE TABLE IF NOT EXISTS run_logs (
    id          INTEGER PRIMARY KEY,
    run_id      TEXT NOT NULL,
    machine     TEXT NOT NULL,
    resource_id TEXT,
    log_level   TEXT NOT NULL DEFAULT 'info',
    message     TEXT NOT NULL,
    timestamp   TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS run_logs_fts USING fts5(
    message, resource_id, machine,
    content='run_logs',
    content_rowid='id'
);

CREATE INDEX IF NOT EXISTS idx_resources_machine ON resources(machine_id);
CREATE INDEX IF NOT EXISTS idx_resources_gen ON resources(generation_id);
CREATE INDEX IF NOT EXISTS idx_resources_type ON resources(resource_type);
CREATE INDEX IF NOT EXISTS idx_events_run ON events(run_id);
CREATE INDEX IF NOT EXISTS idx_events_resource ON events(resource_id);
CREATE INDEX IF NOT EXISTS idx_run_logs_run ON run_logs(run_id);
"#;

/// Open (or create) the state database at the given path.
pub fn open_state_db(path: &std::path::Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| format!("sqlite open: {e}"))?;
    conn.execute_batch(SCHEMA_SQL)
        .map_err(|e| format!("sqlite schema: {e}"))?;
    Ok(conn)
}

/// Query the schema version (user_version pragma).
pub fn schema_version(conn: &Connection) -> SqlResult<u32> {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
}

/// Set the schema version.
pub fn set_schema_version(conn: &Connection, version: u32) -> SqlResult<()> {
    conn.pragma_update(None, "user_version", version)
}

/// FTS5 search across resources.
pub fn fts5_search(conn: &Connection, query: &str, limit: u32) -> Result<Vec<FtsResult>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT resource_id, resource_type, status, path, rank \
             FROM resources_fts WHERE resources_fts MATCH ?1 \
             ORDER BY rank LIMIT ?2",
        )
        .map_err(|e| format!("prepare: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params![query, limit], |row| {
            Ok(FtsResult {
                resource_id: row.get(0)?,
                resource_type: row.get(1)?,
                status: row.get(2)?,
                path: row.get(3)?,
                rank: row.get(4)?,
            })
        })
        .map_err(|e| format!("query: {e}"))?;
    rows.collect::<SqlResult<Vec<_>>>()
        .map_err(|e| format!("collect: {e}"))
}

/// FTS5 search result row.
#[derive(Debug, Clone)]
pub struct FtsResult {
    /// Resource identifier.
    pub resource_id: String,
    /// Resource type (package, file, service, etc.).
    pub resource_type: String,
    /// Status (converged, failed, drifted).
    pub status: String,
    /// Optional path.
    pub path: Option<String>,
    /// FTS5 rank (lower = more relevant).
    pub rank: f64,
}
