//! FJ-2001/2004: Query output formatting helpers.

use crate::core::store::db::FtsResult;
use crate::core::store::ingest;

/// Print timing stats for matched resources.
pub(crate) fn print_timing_stats(
    conn: &rusqlite::Connection,
    results: &[FtsResult],
) -> Result<(), String> {
    let rids: Vec<&str> = results.iter().map(|r| r.resource_id.as_str()).collect();
    if rids.is_empty() {
        return Ok(());
    }

    let placeholders: Vec<String> = (1..=rids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT duration_secs FROM resources WHERE resource_id IN ({}) ORDER BY duration_secs",
        placeholders.join(",")
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("timing prepare: {e}"))?;
    let params: Vec<&dyn rusqlite::types::ToSql> = rids
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    let durations: Vec<f64> = stmt
        .query_map(params.as_slice(), |row| row.get(0))
        .map_err(|e| format!("timing query: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    if durations.is_empty() {
        return Ok(());
    }
    let n = durations.len();
    let avg = durations.iter().sum::<f64>() / n as f64;
    let p50 = durations[n / 2];
    let p95 = durations[(n as f64 * 0.95) as usize];
    println!("\n Timing: avg={avg:.2}s p50={p50:.2}s p95={p95:.2}s (n={n})");
    Ok(())
}

/// Print event history for matched resources.
pub(crate) fn print_history(
    conn: &rusqlite::Connection,
    results: &[FtsResult],
) -> Result<(), String> {
    println!("\n History:");
    for r in results {
        let events = ingest::query_history(conn, &r.resource_id)?;
        if events.is_empty() {
            continue;
        }
        println!("  {}: {} event(s)", r.resource_id, events.len());
        for ev in events.iter().take(3) {
            let dur = ev
                .duration_ms
                .map(|d| format!(" ({d}ms)"))
                .unwrap_or_default();
            println!(
                "    {} {} [{}]{dur}",
                ev.timestamp, ev.event_type, ev.run_id
            );
        }
    }
    Ok(())
}

/// Print reversibility info for matched resources.
pub(crate) fn print_reversibility(
    conn: &rusqlite::Connection,
    results: &[FtsResult],
) -> Result<(), String> {
    let rids: Vec<&str> = results.iter().map(|r| r.resource_id.as_str()).collect();
    if rids.is_empty() {
        return Ok(());
    }

    let placeholders: Vec<String> = (1..=rids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT resource_id, reversibility FROM resources WHERE resource_id IN ({})",
        placeholders.join(",")
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("rev prepare: {e}"))?;
    let params: Vec<&dyn rusqlite::types::ToSql> = rids
        .iter()
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
    conn: &rusqlite::Connection,
    query: &str,
    results: &[FtsResult],
    history: bool,
) {
    let mut rows: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "resource_id": r.resource_id, "type": r.resource_type,
                "status": r.status, "path": r.path, "rank": r.rank,
            })
        })
        .collect();
    if history {
        for row in &mut rows {
            let rid = row["resource_id"].as_str().unwrap_or("");
            let events = ingest::query_history(conn, rid).unwrap_or_default();
            row["history"] = serde_json::to_value(&events).unwrap_or_default();
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "query": query, "results": rows, "count": results.len()
        }))
        .unwrap_or_default()
    );
}

/// Print FTS results as CSV.
pub(crate) fn print_csv(results: &[FtsResult]) {
    println!("resource,type,status,path,rank");
    for r in results {
        println!(
            "{},{},{},{},{:.4}",
            r.resource_id,
            r.resource_type,
            r.status,
            r.path.as_deref().unwrap_or(""),
            r.rank
        );
    }
}

/// Git history entry for RRF fusion.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct GitLogEntry {
    pub hash: String,
    pub message: String,
    pub files: Vec<String>,
}

/// Search git log for query terms, fuse with FTS results via RRF.
pub(crate) fn print_git_history(query: &str, results: &[FtsResult]) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .args([
            "log",
            "--oneline",
            "--all",
            "-50",
            &format!("--grep={query}"),
        ])
        .output()
        .map_err(|e| format!("git log: {e}"))?;

    if !output.status.success() {
        println!("\n Git: (not in a git repository)");
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let commits: Vec<GitLogEntry> = stdout
        .lines()
        .take(10)
        .map(|line| {
            let (hash, msg) = line.split_once(' ').unwrap_or((line, ""));
            GitLogEntry {
                hash: hash.to_string(),
                message: msg.to_string(),
                files: vec![],
            }
        })
        .collect();

    if commits.is_empty() && results.is_empty() {
        println!("\n Git: no commits matching \"{query}\"");
        return Ok(());
    }

    println!("\n Git history (RRF-fused):");
    for (i, commit) in commits.iter().enumerate() {
        let rrf_score = 1.0 / (60.0 + i as f64);
        println!("  [{:.4}] {} {}", rrf_score, commit.hash, commit.message);
    }

    if !commits.is_empty() && !results.is_empty() {
        println!(
            "  Combined: {} resource(s) + {} commit(s)",
            results.len(),
            commits.len()
        );
    }
    Ok(())
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

/// FJ-2004: Show drifted resources.
pub(crate) fn cmd_query_drift(state_dir: &std::path::Path, json: bool) -> Result<(), String> {
    use crate::core::store::ingest;
    let conn = super::dispatch_misc_b::open_state_conn(state_dir)?;
    let entries = ingest::query_drift(&conn)?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&entries).unwrap_or_default()
        );
    } else if entries.is_empty() {
        println!("No drift detected");
    } else {
        println!(
            " {:20} {:10} {:10} EXPECTED → ACTUAL",
            "RESOURCE", "MACHINE", "TYPE"
        );
        for e in &entries {
            println!(
                " {:20} {:10} {:10} {} → {}",
                e.resource_id,
                e.machine,
                e.resource_type,
                &e.content_hash[..20.min(e.content_hash.len())],
                &e.live_hash[..20.min(e.live_hash.len())]
            );
        }
        println!("\n {} drifted resource(s)", entries.len());
    }
    Ok(())
}

/// FJ-2004: Show change frequency (churn).
pub(crate) fn cmd_query_churn(state_dir: &std::path::Path, json: bool) -> Result<(), String> {
    use crate::core::store::ingest;
    let conn = super::dispatch_misc_b::open_state_conn(state_dir)?;
    let entries = ingest::query_churn(&conn)?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&entries).unwrap_or_default()
        );
    } else if entries.is_empty() {
        println!("No churn data");
    } else {
        println!(" {:20} {:>8} {:>8}", "RESOURCE", "EVENTS", "RUNS");
        for e in &entries {
            println!(
                " {:20} {:>8} {:>8}",
                e.resource_id, e.event_count, e.distinct_runs
            );
        }
    }
    Ok(())
}
