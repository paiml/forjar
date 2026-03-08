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

/// Recent events across all resources (optionally filtered).
pub fn query_events(
    conn: &Connection,
    since: Option<&str>,
    run_id: Option<&str>,
    limit: usize,
) -> Result<Vec<ResourceEvent>, String> {
    let mut sql = String::from(
        "SELECT run_id, resource_id || ' [' || machine || ']', event_type, timestamp, duration_ms \
         FROM events WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(since) = since {
        sql.push_str(" AND timestamp >= ?");
        params.push(Box::new(since.to_string()));
    }
    if let Some(run) = run_id {
        sql.push_str(" AND run_id = ?");
        params.push(Box::new(run.to_string()));
    }
    sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT {limit}"));

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("prepare events: {e}"))?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(ResourceEvent {
                run_id: row.get(0)?,
                event_type: row.get(2)?,
                timestamp: row.get(3)?,
                duration_ms: row.get(4)?,
            })
        })
        .map_err(|e| format!("query events: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect events: {e}"))
}

/// Failed events only.
pub fn query_failures(
    conn: &Connection,
    since: Option<&str>,
    limit: usize,
) -> Result<Vec<FailureEntry>, String> {
    let mut sql = String::from(
        "SELECT run_id, resource_id, machine, event_type, timestamp, \
         exit_code, stderr_tail FROM events \
         WHERE event_type LIKE '%failed%'",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(since) = since {
        sql.push_str(" AND timestamp >= ?");
        params.push(Box::new(since.to_string()));
    }
    sql.push_str(&format!(" ORDER BY timestamp DESC LIMIT {limit}"));

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("prepare failures: {e}"))?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(FailureEntry {
                run_id: row.get(0)?,
                resource_id: row.get(1)?,
                machine: row.get(2)?,
                event_type: row.get(3)?,
                timestamp: row.get(4)?,
                exit_code: row.get(5)?,
                stderr_tail: row.get(6)?,
            })
        })
        .map_err(|e| format!("query failures: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("collect failures: {e}"))
}

/// Failure event entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FailureEntry {
    pub run_id: String,
    pub resource_id: String,
    pub machine: String,
    pub event_type: String,
    pub timestamp: String,
    pub exit_code: Option<i64>,
    pub stderr_tail: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::store::db;

    fn setup_test_db() -> Connection {
        let conn = db::open_state_db(std::path::Path::new(":memory:")).unwrap();
        conn.execute(
            "INSERT INTO machines (name, first_seen, last_seen) VALUES (?1, ?2, ?3)",
            ["testmachine", "2026-03-08T00:00:00", "2026-03-08T12:00:00"],
        )
        .unwrap();
        let machine_id: i64 = conn
            .query_row(
                "SELECT id FROM machines WHERE name = ?1",
                ["testmachine"],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "INSERT INTO generations (generation_num, run_id, config_hash, created_at) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![1, "run-001", "hash123", "2026-03-08T00:00:00"],
        )
        .unwrap();
        let gen_id: i64 = conn
            .query_row(
                "SELECT id FROM generations WHERE generation_num = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        conn.execute(
            "INSERT INTO resources (machine_id, generation_id, resource_id, resource_type, status, \
             content_hash, live_hash, applied_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![machine_id, gen_id, "nginx-pkg", "package", "converged", "abc123", "abc123", "2026-03-08T12:00:00"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp, duration_ms, exit_code) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["run-001", "nginx-pkg", "testmachine", "resource_converged", "2026-03-08T12:00:00", 150, 0],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO events (run_id, resource_id, machine, event_type, timestamp, exit_code, stderr_tail) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params!["run-002", "bad-pkg", "testmachine", "resource_failed", "2026-03-08T13:00:00", 1, "E: Package not found"],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_query_health() {
        let conn = setup_test_db();
        let health = query_health(&conn).unwrap();
        assert_eq!(health.machines.len(), 1);
        assert_eq!(health.machines[0].name, "testmachine");
        assert_eq!(health.total_converged, 1);
    }

    #[test]
    fn test_query_events_all() {
        let conn = setup_test_db();
        let events = query_events(&conn, None, None, 50).unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_query_events_by_run() {
        let conn = setup_test_db();
        let events = query_events(&conn, None, Some("run-001"), 50).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "resource_converged");
    }

    #[test]
    fn test_query_events_since() {
        let conn = setup_test_db();
        let events = query_events(&conn, Some("2026-03-08T12:30:00"), None, 50).unwrap();
        assert_eq!(events.len(), 1); // only the failure after 12:30
    }

    #[test]
    fn test_query_failures() {
        let conn = setup_test_db();
        let failures = query_failures(&conn, None, 50).unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].resource_id, "bad-pkg");
        assert_eq!(failures[0].exit_code, Some(1));
        assert_eq!(
            failures[0].stderr_tail.as_deref(),
            Some("E: Package not found")
        );
    }

    #[test]
    fn test_query_failures_since_filter() {
        let conn = setup_test_db();
        // Before the failure — should return nothing
        let failures = query_failures(&conn, Some("2026-03-08T14:00:00"), 50).unwrap();
        assert!(failures.is_empty());
    }

    #[test]
    fn test_query_drift_empty() {
        let conn = setup_test_db();
        // content_hash == live_hash, so no drift
        let drift = query_drift(&conn).unwrap();
        assert!(drift.is_empty());
    }

    #[test]
    fn test_query_churn() {
        let conn = setup_test_db();
        let churn = query_churn(&conn).unwrap();
        // Only "resource_converged" events match churn query
        assert!(churn.len() <= 1);
    }
}
