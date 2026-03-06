//! FJ-2001: Ingest pipeline — parse state files into SQLite.
//!
//! Reads `state/<machine>/state.lock.yaml` and `events.jsonl` files,
//! inserting rows into machines, resources, events, and FTS5 tables.

use rusqlite::Connection;
use std::path::Path;

/// Result of a full ingest run.
#[derive(Debug, Clone)]
pub struct IngestResult {
    /// Number of machines ingested.
    pub machines: usize,
    /// Number of resources ingested.
    pub resources: usize,
    /// Number of events ingested.
    pub events: usize,
}

impl std::fmt::Display for IngestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Ingested {} machines, {} resources, {} events",
            self.machines, self.resources, self.events
        )
    }
}

/// Ingest all machine state directories into the database.
///
/// Scans `state_dir` for subdirectories, each representing a machine.
/// Parses `state.lock.yaml` for resources and `events.jsonl` for events.
pub fn ingest_state_dir(conn: &Connection, state_dir: &Path) -> Result<IngestResult, String> {
    let mut result = IngestResult { machines: 0, resources: 0, events: 0 };

    // Ensure a default generation exists for ingested resources
    let gen_id = ensure_default_generation(conn)?;

    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("read state dir: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("read entry: {e}"))?;
        let path = entry.path();
        if !path.is_dir() { continue; }

        let machine_name = path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "invalid dir name".to_string())?
            .to_string();

        let lock_path = path.join("state.lock.yaml");
        if !lock_path.exists() { continue; }

        let machine_id = upsert_machine(conn, &machine_name, &lock_path)?;
        result.machines += 1;

        result.resources += ingest_lock_file(conn, machine_id, gen_id, &lock_path)?;

        let events_path = path.join("events.jsonl");
        if events_path.exists() {
            result.events += ingest_events(conn, &machine_name, &events_path)?;
        }
    }

    // Rebuild FTS index
    populate_fts(conn)?;

    Ok(result)
}

/// Ensure a generation row exists for ingesting lock-file resources.
fn ensure_default_generation(conn: &Connection) -> Result<i64, String> {
    conn.execute(
        "INSERT OR IGNORE INTO generations (generation_num, run_id, config_hash, created_at) \
         VALUES (1, 'ingest', 'ingest', datetime('now'))",
        [],
    ).map_err(|e| format!("insert generation: {e}"))?;

    conn.query_row(
        "SELECT id FROM generations WHERE run_id = 'ingest'",
        [],
        |row| row.get(0),
    ).map_err(|e| format!("query generation: {e}"))
}

/// Upsert a machine from its lock file metadata.
fn upsert_machine(conn: &Connection, name: &str, lock_path: &Path) -> Result<i64, String> {
    let yaml_str = std::fs::read_to_string(lock_path)
        .map_err(|e| format!("read {}: {e}", lock_path.display()))?;
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml_str)
        .map_err(|e| format!("parse {}: {e}", lock_path.display()))?;

    let hostname = doc.get("hostname")
        .and_then(|v| v.as_str())
        .unwrap_or(name);
    let generated_at = doc.get("generated_at")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    conn.execute(
        "INSERT INTO machines (name, hostname, transport, first_seen, last_seen) \
         VALUES (?1, ?2, 'local', ?3, ?3) \
         ON CONFLICT(name) DO UPDATE SET last_seen = ?3, hostname = ?2",
        rusqlite::params![name, hostname, generated_at],
    ).map_err(|e| format!("upsert machine: {e}"))?;

    conn.query_row(
        "SELECT id FROM machines WHERE name = ?1",
        [name],
        |row| row.get(0),
    ).map_err(|e| format!("query machine id: {e}"))
}

/// Parse state.lock.yaml and insert resource rows.
fn ingest_lock_file(
    conn: &Connection, machine_id: i64, gen_id: i64, lock_path: &Path,
) -> Result<usize, String> {
    let yaml_str = std::fs::read_to_string(lock_path)
        .map_err(|e| format!("read lock: {e}"))?;
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml_str)
        .map_err(|e| format!("parse lock: {e}"))?;

    let resources = match doc.get("resources").and_then(|v| v.as_mapping()) {
        Some(m) => m,
        None => return Ok(0),
    };

    let mut count = 0;
    for (key, val) in resources {
        let rid = key.as_str().unwrap_or("unknown");
        let rtype = val.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let status = val.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
        let applied_at = val.get("applied_at").and_then(|v| v.as_str()).unwrap_or("unknown");
        let duration = val.get("duration_seconds")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let state_hash = val.get("hash").and_then(|v| v.as_str());

        let details = val.get("details");
        let path = details.and_then(|d| d.get("path")).and_then(|v| v.as_str());
        let content_hash = details.and_then(|d| d.get("content_hash")).and_then(|v| v.as_str());
        let live_hash = details.and_then(|d| d.get("live_hash")).and_then(|v| v.as_str());
        let details_json = details
            .map(|d| serde_json::to_string(d).unwrap_or_default())
            .unwrap_or_else(|| "{}".to_string());

        conn.execute(
            "INSERT OR REPLACE INTO resources \
             (resource_id, machine_id, generation_id, resource_type, status, \
              state_hash, content_hash, live_hash, applied_at, duration_secs, \
              details_json, path) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                rid, machine_id, gen_id, rtype, status,
                state_hash, content_hash, live_hash, applied_at, duration,
                details_json, path,
            ],
        ).map_err(|e| format!("insert resource {rid}: {e}"))?;
        count += 1;
    }
    Ok(count)
}

/// Parse events.jsonl and insert event rows.
fn ingest_events(conn: &Connection, machine: &str, events_path: &Path) -> Result<usize, String> {
    let content = std::fs::read_to_string(events_path)
        .map_err(|e| format!("read events: {e}"))?;

    let mut count = 0;
    for line in content.lines() {
        if line.trim().is_empty() { continue; }
        let ev: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let run_id = ev.get("run_id").and_then(|v| v.as_str()).unwrap_or("");
        let event_type = ev.get("event").and_then(|v| v.as_str()).unwrap_or("unknown");
        let resource_id = ev.get("resource").and_then(|v| v.as_str()).unwrap_or("");
        let ts = ev.get("ts").and_then(|v| v.as_str()).unwrap_or("unknown");
        let duration_ms = ev.get("duration_seconds")
            .and_then(|v| v.as_f64())
            .map(|s| (s * 1000.0) as i64);

        conn.execute(
            "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp, duration_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![run_id, resource_id, machine, event_type, ts, duration_ms],
        ).map_err(|e| format!("insert event: {e}"))?;
        count += 1;
    }
    Ok(count)
}

/// Rebuild FTS5 index from resources table (content-sync rebuild).
fn populate_fts(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "INSERT INTO resources_fts(resources_fts) VALUES('rebuild')",
        [],
    ).map_err(|e| format!("rebuild fts: {e}"))?;
    conn.execute(
        "INSERT INTO resources_fts(resources_fts) VALUES('optimize')",
        [],
    ).map_err(|e| format!("optimize fts: {e}"))?;
    Ok(())
}

/// Health summary for the entire stack.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthSummary {
    pub machines: Vec<MachineHealth>,
    pub total_resources: usize,
    pub total_converged: usize,
    pub total_drifted: usize,
    pub total_failed: usize,
}

impl HealthSummary {
    /// Stack health as percentage.
    pub fn health_pct(&self) -> f64 {
        if self.total_resources == 0 { return 100.0; }
        (self.total_converged as f64 / self.total_resources as f64) * 100.0
    }
}

/// Per-machine health row.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MachineHealth {
    pub name: String,
    pub resources: usize,
    pub converged: usize,
    pub drifted: usize,
    pub failed: usize,
}

/// Query health summary from the database.
pub fn query_health(conn: &Connection) -> Result<HealthSummary, String> {
    let mut stmt = conn.prepare(
        "SELECT m.name, \
         COUNT(*) as total, \
         SUM(CASE WHEN r.status = 'converged' THEN 1 ELSE 0 END), \
         SUM(CASE WHEN r.status = 'drifted' THEN 1 ELSE 0 END), \
         SUM(CASE WHEN r.status = 'failed' THEN 1 ELSE 0 END) \
         FROM resources r JOIN machines m ON r.machine_id = m.id \
         GROUP BY m.name ORDER BY m.name"
    ).map_err(|e| format!("prepare health: {e}"))?;

    let rows = stmt.query_map([], |row| {
        Ok(MachineHealth {
            name: row.get(0)?,
            resources: row.get::<_, i64>(1)? as usize,
            converged: row.get::<_, i64>(2)? as usize,
            drifted: row.get::<_, i64>(3)? as usize,
            failed: row.get::<_, i64>(4)? as usize,
        })
    }).map_err(|e| format!("query health: {e}"))?;

    let machines: Vec<MachineHealth> = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect health: {e}"))?;

    let total_resources = machines.iter().map(|m| m.resources).sum();
    let total_converged = machines.iter().map(|m| m.converged).sum();
    let total_drifted = machines.iter().map(|m| m.drifted).sum();
    let total_failed = machines.iter().map(|m| m.failed).sum();

    Ok(HealthSummary { machines, total_resources, total_converged, total_drifted, total_failed })
}
