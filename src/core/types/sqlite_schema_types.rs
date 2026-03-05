//! FJ-2001: SQLite schema types — DDL, ingest pipeline, query builder.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2001: SQLite database configuration.
///
/// # Examples
///
/// ```
/// use forjar::core::types::SqliteConfig;
///
/// let config = SqliteConfig::default();
/// assert!(config.wal_mode);
/// assert!(config.fts5);
/// assert_eq!(config.db_path(), "state/forjar.db");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    /// State directory containing the database.
    #[serde(default = "default_state_dir")]
    pub state_dir: String,
    /// Enable WAL mode for concurrent reads.
    #[serde(default = "default_true_sql")]
    pub wal_mode: bool,
    /// Enable FTS5 for full-text search.
    #[serde(default = "default_true_sql")]
    pub fts5: bool,
    /// Cache size in pages (negative = KB).
    #[serde(default = "default_cache_size")]
    pub cache_size: i32,
}

fn default_state_dir() -> String {
    "state".into()
}
fn default_true_sql() -> bool {
    true
}
fn default_cache_size() -> i32 {
    -8000 // 8MB
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            state_dir: "state".into(),
            wal_mode: true,
            fts5: true,
            cache_size: -8000,
        }
    }
}

impl SqliteConfig {
    /// Full path to the database file.
    pub fn db_path(&self) -> String {
        format!("{}/forjar.db", self.state_dir)
    }

    /// Generate PRAGMA statements for database configuration.
    pub fn pragma_statements(&self) -> Vec<String> {
        let mut pragmas = vec![
            "PRAGMA busy_timeout = 5000".into(),
            format!("PRAGMA cache_size = {}", self.cache_size),
            "PRAGMA temp_store = MEMORY".into(),
            "PRAGMA mmap_size = 268435456".into(), // 256MB
        ];
        if self.wal_mode {
            pragmas.push("PRAGMA journal_mode = WAL".into());
            pragmas.push("PRAGMA synchronous = NORMAL".into());
        }
        pragmas
    }
}

/// FJ-2001: Schema DDL for the forjar database.
pub struct SchemaV1;

impl SchemaV1 {
    /// Resources table DDL.
    pub const RESOURCES: &'static str = r#"
CREATE TABLE IF NOT EXISTS resources (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    resource_id TEXT    NOT NULL,
    machine     TEXT    NOT NULL,
    type        TEXT    NOT NULL,
    status      TEXT    NOT NULL DEFAULT 'pending',
    hash        TEXT,
    generation  INTEGER NOT NULL DEFAULT 0,
    applied_at  TEXT,
    duration_s  REAL,
    path        TEXT,
    UNIQUE(resource_id, machine)
)"#;

    /// Generations table DDL.
    pub const GENERATIONS: &'static str = r#"
CREATE TABLE IF NOT EXISTS generations (
    id          INTEGER PRIMARY KEY,
    generation  INTEGER NOT NULL UNIQUE,
    created_at  TEXT    NOT NULL,
    config_hash TEXT,
    git_ref     TEXT,
    action      TEXT    NOT NULL DEFAULT 'apply',
    operator    TEXT,
    created     INTEGER NOT NULL DEFAULT 0,
    updated     INTEGER NOT NULL DEFAULT 0,
    destroyed   INTEGER NOT NULL DEFAULT 0,
    unchanged   INTEGER NOT NULL DEFAULT 0
)"#;

    /// Run logs table DDL.
    pub const RUN_LOGS: &'static str = r#"
CREATE TABLE IF NOT EXISTS run_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id      TEXT    NOT NULL,
    machine     TEXT    NOT NULL,
    resource_id TEXT    NOT NULL,
    action      TEXT    NOT NULL,
    exit_code   INTEGER,
    duration_s  REAL,
    log_path    TEXT,
    started_at  TEXT,
    finished_at TEXT
)"#;

    /// FTS5 virtual table for full-text search.
    pub const FTS5_RESOURCES: &'static str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS resources_fts USING fts5(
    resource_id,
    machine,
    type,
    status,
    path,
    content=resources,
    content_rowid=id
)"#;

    /// Indexes DDL.
    pub const INDEXES: &'static [&'static str] = &[
        "CREATE INDEX IF NOT EXISTS idx_resources_machine ON resources(machine)",
        "CREATE INDEX IF NOT EXISTS idx_resources_type ON resources(type)",
        "CREATE INDEX IF NOT EXISTS idx_resources_status ON resources(status)",
        "CREATE INDEX IF NOT EXISTS idx_resources_gen ON resources(generation)",
        "CREATE INDEX IF NOT EXISTS idx_run_logs_run ON run_logs(run_id)",
        "CREATE INDEX IF NOT EXISTS idx_run_logs_machine ON run_logs(machine)",
        "CREATE INDEX IF NOT EXISTS idx_run_logs_resource ON run_logs(resource_id)",
    ];

    /// All DDL statements in creation order.
    pub fn all_ddl() -> Vec<&'static str> {
        let mut ddl = vec![
            Self::RESOURCES,
            Self::GENERATIONS,
            Self::RUN_LOGS,
            Self::FTS5_RESOURCES,
        ];
        ddl.extend(Self::INDEXES.iter());
        ddl
    }

    /// Schema version.
    pub const VERSION: u32 = 1;
}

/// FJ-2001: Ingest cursor for incremental state file ingestion.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestCursor {
    /// Last ingested generation per machine.
    #[serde(default)]
    pub last_generation: std::collections::HashMap<String, u32>,
    /// Last ingest timestamp.
    #[serde(default)]
    pub last_ingest_at: Option<String>,
    /// Total resources ingested.
    #[serde(default)]
    pub total_ingested: u64,
}

impl IngestCursor {
    /// Check if a generation has already been ingested for a machine.
    pub fn is_ingested(&self, machine: &str, generation: u32) -> bool {
        self.last_generation
            .get(machine)
            .is_some_and(|&last| generation <= last)
    }

    /// Mark a generation as ingested.
    pub fn mark_ingested(&mut self, machine: &str, generation: u32, count: u64) {
        self.last_generation
            .insert(machine.to_string(), generation);
        self.total_ingested += count;
    }
}

/// FJ-2001: Ingest result summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestResult {
    /// Number of resources upserted.
    pub resources_upserted: u64,
    /// Number of generations ingested.
    pub generations_ingested: u32,
    /// Number of run logs ingested.
    pub run_logs_ingested: u64,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Machines processed.
    pub machines: Vec<String>,
}

impl fmt::Display for IngestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Ingested: {} resources, {} generations, {} run logs from {} machines ({:.2}s)",
            self.resources_upserted,
            self.generations_ingested,
            self.run_logs_ingested,
            self.machines.len(),
            self.duration_secs,
        )
    }
}

/// FJ-2004: Query enrichment flags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryEnrichments {
    /// Include generation history.
    pub history: bool,
    /// Include drift findings.
    pub drift: bool,
    /// Include timing stats.
    pub timing: bool,
    /// Include churn metrics.
    pub churn: bool,
    /// Include health summary.
    pub health: bool,
    /// Include destroy log.
    pub destroy_log: bool,
    /// Include reversibility analysis.
    pub reversibility: bool,
    /// Git history fusion via RRF.
    pub git_history: bool,
}

impl QueryEnrichments {
    /// Whether any enrichment is enabled.
    pub fn any_enabled(&self) -> bool {
        self.history
            || self.drift
            || self.timing
            || self.churn
            || self.health
            || self.destroy_log
            || self.reversibility
            || self.git_history
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_config_default() {
        let c = SqliteConfig::default();
        assert!(c.wal_mode);
        assert!(c.fts5);
        assert_eq!(c.cache_size, -8000);
        assert_eq!(c.db_path(), "state/forjar.db");
    }

    #[test]
    fn sqlite_config_pragmas() {
        let c = SqliteConfig::default();
        let pragmas = c.pragma_statements();
        assert!(pragmas.iter().any(|p| p.contains("journal_mode = WAL")));
        assert!(pragmas.iter().any(|p| p.contains("busy_timeout")));
        assert!(pragmas.iter().any(|p| p.contains("cache_size")));
    }

    #[test]
    fn sqlite_config_no_wal() {
        let c = SqliteConfig {
            wal_mode: false,
            ..Default::default()
        };
        let pragmas = c.pragma_statements();
        assert!(!pragmas.iter().any(|p| p.contains("WAL")));
    }

    #[test]
    fn schema_v1_all_ddl() {
        let ddl = SchemaV1::all_ddl();
        assert!(ddl.len() >= 4);
        assert!(ddl[0].contains("resources"));
        assert!(ddl[1].contains("generations"));
        assert!(ddl[2].contains("run_logs"));
        assert!(ddl[3].contains("fts5"));
    }

    #[test]
    fn schema_v1_indexes() {
        assert!(SchemaV1::INDEXES.len() >= 5);
        for idx in SchemaV1::INDEXES {
            assert!(idx.starts_with("CREATE INDEX"));
        }
    }

    #[test]
    fn ingest_cursor_default() {
        let c = IngestCursor::default();
        assert!(c.last_generation.is_empty());
        assert_eq!(c.total_ingested, 0);
    }

    #[test]
    fn ingest_cursor_mark_and_check() {
        let mut c = IngestCursor::default();
        assert!(!c.is_ingested("intel", 1));
        c.mark_ingested("intel", 3, 10);
        assert!(c.is_ingested("intel", 1));
        assert!(c.is_ingested("intel", 3));
        assert!(!c.is_ingested("intel", 4));
        assert!(!c.is_ingested("jetson", 1));
        assert_eq!(c.total_ingested, 10);
    }

    #[test]
    fn ingest_result_display() {
        let r = IngestResult {
            resources_upserted: 50,
            generations_ingested: 5,
            run_logs_ingested: 25,
            duration_secs: 0.5,
            machines: vec!["intel".into(), "jetson".into()],
        };
        let s = r.to_string();
        assert!(s.contains("50 resources"));
        assert!(s.contains("5 generations"));
        assert!(s.contains("2 machines"));
    }

    #[test]
    fn query_enrichments_default() {
        let e = QueryEnrichments::default();
        assert!(!e.any_enabled());
    }

    #[test]
    fn query_enrichments_any_enabled() {
        let e = QueryEnrichments {
            timing: true,
            ..Default::default()
        };
        assert!(e.any_enabled());
    }

    #[test]
    fn ingest_cursor_serde() {
        let mut c = IngestCursor::default();
        c.mark_ingested("m", 5, 20);
        let json = serde_json::to_string(&c).unwrap();
        let parsed: IngestCursor = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_ingested("m", 5));
    }

    #[test]
    fn sqlite_config_custom_dir() {
        let c = SqliteConfig {
            state_dir: "/opt/forjar/state".into(),
            ..Default::default()
        };
        assert_eq!(c.db_path(), "/opt/forjar/state/forjar.db");
    }
}
