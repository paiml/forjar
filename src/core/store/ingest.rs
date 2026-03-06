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

        // F7: Ingest destroy-log.jsonl → destroy_log table
        let destroy_path = path.join("destroy-log.jsonl");
        if destroy_path.exists() {
            ingest_destroy_log(conn, machine_id, gen_id, &destroy_path)?;
        }
    }

    // Ingest generations from state/generations/ if present
    let gens_dir = state_dir.join("generations");
    if gens_dir.is_dir() {
        ingest_generations(conn, &gens_dir)?;
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

        // FTS5 field extraction: packages for package resources, content_preview for files
        let packages = if rtype == "package" { Some(rid.to_string()) } else { None };
        let content_preview = details
            .and_then(|d| d.get("content_preview"))
            .and_then(|v| v.as_str())
            .map(|s| s.chars().take(200).collect::<String>());

        conn.execute(
            "INSERT OR REPLACE INTO resources \
             (resource_id, machine_id, generation_id, resource_type, status, \
              state_hash, content_hash, live_hash, applied_at, duration_secs, \
              details_json, path, packages, content_preview) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            rusqlite::params![
                rid, machine_id, gen_id, rtype, status,
                state_hash, content_hash, live_hash, applied_at, duration,
                details_json, path, packages, content_preview,
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

/// Ingest generation metadata from `state/generations/` directory.
fn ingest_generations(conn: &Connection, gens_dir: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(gens_dir)
        .map_err(|e| format!("read generations dir: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("read gen entry: {e}"))?;
        let path = entry.path();
        let gen_file = if path.is_dir() {
            path.join(".generation.yaml")
        } else if path.extension().is_some_and(|e| e == "yaml") {
            path
        } else {
            continue;
        };
        if !gen_file.exists() { continue; }

        let yaml_str = match std::fs::read_to_string(&gen_file) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let doc: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&yaml_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let gen_num = doc.get("generation").and_then(|v| v.as_u64()).unwrap_or(0) as i64;
        let run_id = doc.get("run_id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let config_hash = doc.get("config_hash").and_then(|v| v.as_str()).unwrap_or("unknown");
        let created_at = doc.get("created_at").and_then(|v| v.as_str()).unwrap_or("unknown");
        let git_ref = doc.get("git_ref").and_then(|v| v.as_str());
        let action = doc.get("action").and_then(|v| v.as_str()).unwrap_or("apply");

        conn.execute(
            "INSERT OR REPLACE INTO generations \
             (generation_num, run_id, config_hash, created_at, git_ref, action) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![gen_num, run_id, config_hash, created_at, git_ref, action],
        ).map_err(|e| format!("insert generation: {e}"))?;
    }
    Ok(())
}

/// Ingest destroy-log.jsonl into the destroy_log table.
fn ingest_destroy_log(
    conn: &Connection, machine_id: i64, gen_id: i64, path: &Path,
) -> Result<usize, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("read destroy-log: {e}"))?;

    let mut count = 0;
    for line in content.lines() {
        if line.trim().is_empty() { continue; }
        let ev: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let resource_id = ev.get("resource_id").and_then(|v| v.as_str()).unwrap_or("");
        let resource_type = ev.get("resource_type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let pre_hash = ev.get("pre_hash").and_then(|v| v.as_str());
        let timestamp = ev.get("timestamp").and_then(|v| v.as_str()).unwrap_or("unknown");

        conn.execute(
            "INSERT INTO destroy_log \
             (machine_id, generation_id, resource_id, resource_type, \
              pre_destroy_hash, destroyed_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![machine_id, gen_id, resource_id, resource_type, pre_hash, timestamp],
        ).map_err(|e| format!("insert destroy_log: {e}"))?;
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

/// Event history for a resource across runs.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceEvent {
    pub run_id: String,
    pub event_type: String,
    pub timestamp: String,
    pub duration_ms: Option<i64>,
}

/// Query event history for a specific resource.
pub fn query_history(conn: &Connection, resource_id: &str) -> Result<Vec<ResourceEvent>, String> {
    let mut stmt = conn.prepare(
        "SELECT run_id, event_type, timestamp, duration_ms \
         FROM events WHERE resource_id = ?1 ORDER BY timestamp DESC LIMIT 50"
    ).map_err(|e| format!("prepare history: {e}"))?;

    let rows = stmt.query_map([resource_id], |row| {
        Ok(ResourceEvent {
            run_id: row.get(0)?,
            event_type: row.get(1)?,
            timestamp: row.get(2)?,
            duration_ms: row.get(3)?,
        })
    }).map_err(|e| format!("query history: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect history: {e}"))
}

/// Drift: resources where live_hash differs from content_hash.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DriftEntry {
    pub resource_id: String,
    pub machine: String,
    pub resource_type: String,
    pub content_hash: String,
    pub live_hash: String,
}

/// Find drifted resources (live_hash != content_hash).
pub fn query_drift(conn: &Connection) -> Result<Vec<DriftEntry>, String> {
    let mut stmt = conn.prepare(
        "SELECT r.resource_id, m.name, r.resource_type, r.content_hash, r.live_hash \
         FROM resources r JOIN machines m ON r.machine_id = m.id \
         WHERE r.content_hash IS NOT NULL AND r.live_hash IS NOT NULL \
         AND r.content_hash != r.live_hash"
    ).map_err(|e| format!("prepare drift: {e}"))?;

    let rows = stmt.query_map([], |row| {
        Ok(DriftEntry {
            resource_id: row.get(0)?,
            machine: row.get(1)?,
            resource_type: row.get(2)?,
            content_hash: row.get(3)?,
            live_hash: row.get(4)?,
        })
    }).map_err(|e| format!("query drift: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect drift: {e}"))
}

/// Churn: how often a resource has changed across events.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChurnEntry {
    pub resource_id: String,
    pub event_count: usize,
    pub distinct_runs: usize,
}

/// Query change frequency (churn) for resources.
pub fn query_churn(conn: &Connection) -> Result<Vec<ChurnEntry>, String> {
    let mut stmt = conn.prepare(
        "SELECT resource_id, COUNT(*) as events, COUNT(DISTINCT run_id) as runs \
         FROM events WHERE resource_id != '' AND event_type LIKE '%converged%' \
         GROUP BY resource_id ORDER BY events DESC LIMIT 50"
    ).map_err(|e| format!("prepare churn: {e}"))?;

    let rows = stmt.query_map([], |row| {
        Ok(ChurnEntry {
            resource_id: row.get(0)?,
            event_count: row.get::<_, i64>(1)? as usize,
            distinct_runs: row.get::<_, i64>(2)? as usize,
        })
    }).map_err(|e| format!("query churn: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect churn: {e}"))
}
