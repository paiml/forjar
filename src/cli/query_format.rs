//! FJ-2001/2004: Query output formatting helpers.

use crate::core::store::db::FtsResult;
use crate::core::store::ingest;

/// Print timing stats for matched resources.
pub(crate) fn print_timing_stats(
    conn: &rusqlite::Connection, results: &[FtsResult],
) -> Result<(), String> {
    let rids: Vec<&str> = results.iter().map(|r| r.resource_id.as_str()).collect();
    if rids.is_empty() { return Ok(()); }

    let placeholders: Vec<String> = (1..=rids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT duration_secs FROM resources WHERE resource_id IN ({}) ORDER BY duration_secs",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("timing prepare: {e}"))?;
    let params: Vec<&dyn rusqlite::types::ToSql> = rids.iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    let durations: Vec<f64> = stmt
        .query_map(params.as_slice(), |row| row.get(0))
        .map_err(|e| format!("timing query: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    if durations.is_empty() { return Ok(()); }
    let n = durations.len();
    let avg = durations.iter().sum::<f64>() / n as f64;
    let p50 = durations[n / 2];
    let p95 = durations[(n as f64 * 0.95) as usize];
    println!("\n Timing: avg={avg:.2}s p50={p50:.2}s p95={p95:.2}s (n={n})");
    Ok(())
}

/// Print event history for matched resources.
pub(crate) fn print_history(
    conn: &rusqlite::Connection, results: &[FtsResult],
) -> Result<(), String> {
    println!("\n History:");
    for r in results {
        let events = ingest::query_history(conn, &r.resource_id)?;
        if events.is_empty() { continue; }
        println!("  {}: {} event(s)", r.resource_id, events.len());
        for ev in events.iter().take(3) {
            let dur = ev.duration_ms.map(|d| format!(" ({d}ms)")).unwrap_or_default();
            println!("    {} {} [{}]{dur}", ev.timestamp, ev.event_type, ev.run_id);
        }
    }
    Ok(())
}

/// Print reversibility info for matched resources.
pub(crate) fn print_reversibility(
    conn: &rusqlite::Connection, results: &[FtsResult],
) -> Result<(), String> {
    let rids: Vec<&str> = results.iter().map(|r| r.resource_id.as_str()).collect();
    if rids.is_empty() { return Ok(()); }

    let placeholders: Vec<String> = (1..=rids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT resource_id, reversibility FROM resources WHERE resource_id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("rev prepare: {e}"))?;
    let params: Vec<&dyn rusqlite::types::ToSql> = rids.iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    let rows: Vec<(String, String)> = stmt
        .query_map(params.as_slice(), |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| format!("rev query: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    println!("\n Reversibility:");
    for (rid, rev) in &rows {
        println!("  {rid}: {rev}");
    }
    Ok(())
}

/// Print FTS results as JSON.
pub(crate) fn print_json(
    conn: &rusqlite::Connection, query: &str, results: &[FtsResult], history: bool,
) {
    let mut rows: Vec<serde_json::Value> = results.iter().map(|r| {
        serde_json::json!({
            "resource_id": r.resource_id, "type": r.resource_type,
            "status": r.status, "path": r.path, "rank": r.rank,
        })
    }).collect();
    if history {
        for row in &mut rows {
            let rid = row["resource_id"].as_str().unwrap_or("");
            let events = ingest::query_history(conn, rid).unwrap_or_default();
            row["history"] = serde_json::to_value(&events).unwrap_or_default();
        }
    }
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "query": query, "results": rows, "count": results.len()
    })).unwrap_or_default());
}

/// Print FTS results as CSV.
pub(crate) fn print_csv(results: &[FtsResult]) {
    println!("resource,type,status,path,rank");
    for r in results {
        println!("{},{},{},{},{:.4}",
            r.resource_id, r.resource_type, r.status,
            r.path.as_deref().unwrap_or(""), r.rank);
    }
}

/// Print the SQL that would be executed for a query (--sql mode).
pub(crate) fn print_sql(query: &str, resource_type: Option<&str>) {
    println!("-- FTS5 search query");
    println!("SELECT r.resource_id, r.resource_type, r.status, r.path, rank");
    println!("FROM resources_fts");
    println!("JOIN resources r ON r.id = resources_fts.rowid");
    println!("WHERE resources_fts MATCH '{query}'");
    if let Some(rtype) = resource_type {
        println!("  AND r.resource_type = '{rtype}'");
    }
    println!("ORDER BY rank LIMIT 50;");
}
