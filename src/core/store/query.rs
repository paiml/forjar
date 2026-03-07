//! FJ-2001: Query functions for state database — health, history, drift, churn.

use rusqlite::Connection;

/// Stack-wide health summary.
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
        if self.total_resources == 0 {
            return 100.0;
        }
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
    let mut stmt = conn
        .prepare(
            "SELECT m.name, \
         COUNT(*) as total, \
         SUM(CASE WHEN r.status = 'converged' THEN 1 ELSE 0 END), \
         SUM(CASE WHEN r.status = 'drifted' THEN 1 ELSE 0 END), \
         SUM(CASE WHEN r.status = 'failed' THEN 1 ELSE 0 END) \
         FROM resources r JOIN machines m ON r.machine_id = m.id \
         GROUP BY m.name ORDER BY m.name",
        )
        .map_err(|e| format!("prepare health: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(MachineHealth {
                name: row.get(0)?,
                resources: row.get::<_, i64>(1)? as usize,
                converged: row.get::<_, i64>(2)? as usize,
                drifted: row.get::<_, i64>(3)? as usize,
                failed: row.get::<_, i64>(4)? as usize,
            })
        })
        .map_err(|e| format!("query health: {e}"))?;

    let machines: Vec<MachineHealth> = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect health: {e}"))?;

    let total_resources = machines.iter().map(|m| m.resources).sum();
    let total_converged = machines.iter().map(|m| m.converged).sum();
    let total_drifted = machines.iter().map(|m| m.drifted).sum();
    let total_failed = machines.iter().map(|m| m.failed).sum();

    Ok(HealthSummary {
        machines,
        total_resources,
        total_converged,
        total_drifted,
        total_failed,
    })
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
    let mut stmt = conn
        .prepare(
            "SELECT run_id, event_type, timestamp, duration_ms \
         FROM events WHERE resource_id = ?1 ORDER BY timestamp DESC LIMIT 50",
        )
        .map_err(|e| format!("prepare history: {e}"))?;

    let rows = stmt
        .query_map([resource_id], |row| {
            Ok(ResourceEvent {
                run_id: row.get(0)?,
                event_type: row.get(1)?,
                timestamp: row.get(2)?,
                duration_ms: row.get(3)?,
            })
        })
        .map_err(|e| format!("query history: {e}"))?;

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
    let mut stmt = conn
        .prepare(
            "SELECT r.resource_id, m.name, r.resource_type, r.content_hash, r.live_hash \
         FROM resources r JOIN machines m ON r.machine_id = m.id \
         WHERE r.content_hash IS NOT NULL AND r.live_hash IS NOT NULL \
         AND r.content_hash != r.live_hash",
        )
        .map_err(|e| format!("prepare drift: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(DriftEntry {
                resource_id: row.get(0)?,
                machine: row.get(1)?,
                resource_type: row.get(2)?,
                content_hash: row.get(3)?,
                live_hash: row.get(4)?,
            })
        })
        .map_err(|e| format!("query drift: {e}"))?;

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
    let mut stmt = conn
        .prepare(
            "SELECT resource_id, COUNT(*) as events, COUNT(DISTINCT run_id) as runs \
         FROM events WHERE resource_id != '' AND event_type LIKE '%converged%' \
         GROUP BY resource_id ORDER BY events DESC LIMIT 50",
        )
        .map_err(|e| format!("prepare churn: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ChurnEntry {
                resource_id: row.get(0)?,
                event_count: row.get::<_, i64>(1)? as usize,
                distinct_runs: row.get::<_, i64>(2)? as usize,
            })
        })
        .map_err(|e| format!("query churn: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect churn: {e}"))
}
