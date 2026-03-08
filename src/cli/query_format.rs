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

/// Print table results for state query.
pub(crate) fn print_table_results(
    query: &str,
    conn: &rusqlite::Connection,
    results: &[FtsResult],
    history: bool,
    timing: bool,
    reversibility: bool,
) -> Result<(), String> {
    if results.is_empty() {
        println!("No results for \"{query}\"");
        return Ok(());
    }
    println!(" {:20} {:10} {:10} PATH", "RESOURCE", "TYPE", "STATUS");
    for r in results {
        let p = r.path.as_deref().unwrap_or("—");
        println!(
            " {:20} {:10} {:10} {p}",
            r.resource_id, r.resource_type, r.status
        );
    }
    if history {
        print_history(conn, results)?;
    }
    if timing {
        print_timing_stats(conn, results)?;
    }
    if reversibility {
        print_reversibility(conn, results)?;
    }
    println!("\n {} result(s)", results.len());
    Ok(())
}

/// FJ-2001: Show recent events for matched resources.
pub(crate) fn cmd_query_events(
    state_dir: &std::path::Path,
    since: Option<&str>,
    run_id: Option<&str>,
    json: bool,
) -> Result<(), String> {
    use crate::core::store::query;
    let conn = super::dispatch_misc_b::open_state_conn(state_dir)?;
    let since_ts = since.map(resolve_since);
    let events = query::query_events(&conn, since_ts.as_deref(), run_id, 50)?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&events).unwrap_or_default()
        );
    } else if events.is_empty() {
        println!("No events found");
    } else {
        println!(" {:20} {:20} {:>8} {:26}", "RUN", "TYPE", "MS", "TIMESTAMP");
        for ev in &events {
            let dur = ev.duration_ms.map(|d| format!("{d}")).unwrap_or_default();
            println!(
                " {:20} {:20} {:>8} {:26}",
                &ev.run_id[..20.min(ev.run_id.len())],
                ev.event_type,
                dur,
                ev.timestamp
            );
        }
        println!("\n {} event(s)", events.len());
    }
    Ok(())
}

/// FJ-2001: Show failure history.
pub(crate) fn cmd_query_failures(
    state_dir: &std::path::Path,
    since: Option<&str>,
    json: bool,
) -> Result<(), String> {
    use crate::core::store::query;
    let conn = super::dispatch_misc_b::open_state_conn(state_dir)?;
    let since_ts = since.map(resolve_since);
    let failures = query::query_failures(&conn, since_ts.as_deref(), 50)?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&failures).unwrap_or_default()
        );
    } else if failures.is_empty() {
        println!("No failures found");
    } else {
        println!(
            " {:20} {:10} {:20} {:>6} TIMESTAMP",
            "RESOURCE", "MACHINE", "TYPE", "EXIT"
        );
        for f in &failures {
            let exit = f
                .exit_code
                .map(|c| format!("{c}"))
                .unwrap_or_else(|| "—".to_string());
            println!(
                " {:20} {:10} {:20} {:>6} {}",
                f.resource_id, f.machine, f.event_type, exit, f.timestamp
            );
            if let Some(ref stderr) = f.stderr_tail {
                if !stderr.is_empty() {
                    for line in stderr.lines().take(2) {
                        println!("   {line}");
                    }
                }
            }
        }
        println!("\n {} failure(s)", failures.len());
    }
    Ok(())
}

/// Resolve a --since value to an ISO timestamp.
/// Supports relative durations ("1h", "7d", "30m") and ISO timestamps.
pub(crate) fn resolve_since(s: &str) -> String {
    let s = s.trim();
    // Try relative duration
    if let Some(num_str) = s.strip_suffix('h') {
        if let Ok(hours) = num_str.parse::<i64>() {
            let now = chrono_now_minus_seconds(hours * 3600);
            return now;
        }
    }
    if let Some(num_str) = s.strip_suffix('d') {
        if let Ok(days) = num_str.parse::<i64>() {
            let now = chrono_now_minus_seconds(days * 86400);
            return now;
        }
    }
    if let Some(num_str) = s.strip_suffix('m') {
        if let Ok(mins) = num_str.parse::<i64>() {
            let now = chrono_now_minus_seconds(mins * 60);
            return now;
        }
    }
    // Assume ISO timestamp
    s.to_string()
}

/// Get ISO timestamp for now minus N seconds (no chrono dependency).
pub(crate) fn chrono_now_minus_seconds(secs: i64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        - secs;
    // Format as ISO 8601 (approximate — no chrono needed)
    let days = now / 86400;
    let remaining = now % 86400;
    let hours = remaining / 3600;
    let mins = (remaining % 3600) / 60;
    let s = remaining % 60;
    // Simple epoch-to-date (good enough for SQLite comparison)
    // Use days since epoch to compute Y-M-D
    let (year, month, day) = epoch_days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{mins:02}:{s:02}")
}

/// Convert days since Unix epoch to (year, month, day).
pub(crate) fn epoch_days_to_ymd(days: i64) -> (i64, i64, i64) {
    // Civil calendar from days since epoch (Rata Die algorithm)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// FJ-2001: Health summary across all machines.
pub(crate) fn cmd_query_health(state_dir: &std::path::Path, json: bool) -> Result<(), String> {
    let conn = super::dispatch_misc_b::open_state_conn(state_dir)?;
    let health = ingest::query_health(&conn)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&health).unwrap_or_default()
        );
    } else if health.machines.is_empty() {
        println!("No machines found in {}", state_dir.display());
    } else {
        println!(
            " {:10} {:>10} {:>10} {:>8} {:>8}",
            "MACHINE", "RESOURCES", "CONVERGED", "DRIFTED", "FAILED"
        );
        for m in &health.machines {
            println!(
                " {:10} {:>10} {:>10} {:>8} {:>8}",
                m.name, m.resources, m.converged, m.drifted, m.failed
            );
        }
        println!(" {}", "─".repeat(56));
        println!(
            " {:10} {:>10} {:>10} {:>8} {:>8}  Stack health: {:.0}%",
            "TOTAL",
            health.total_resources,
            health.total_converged,
            health.total_drifted,
            health.total_failed,
            health.health_pct()
        );
    }
    Ok(())
}
