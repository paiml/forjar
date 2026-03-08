//! FJ-2001/2004: Query engine enrichment flags.
//!
//! Demonstrates the `forjar state-query` enrichment flags including
//! --events, --failures, --since, --status, --run, --history, --timing.
//!
//! Usage: cargo run --example query_enrichments

use std::time::{SystemTime, UNIX_EPOCH};

/// Convert relative duration string ("1h", "7d", "30m") to ISO 8601 timestamp.
fn resolve_since(s: &str) -> String {
    let s = s.trim();
    if s.contains('T') || s.contains('-') {
        return s.to_string(); // already ISO 8601
    }
    let (num_str, unit) = s.split_at(s.len().saturating_sub(1));
    let num: i64 = match num_str.parse() {
        Ok(n) => n,
        Err(_) => return s.to_string(),
    };
    let secs = match unit {
        "h" => num * 3600,
        "d" => num * 86400,
        "m" => num * 60,
        "s" => num,
        _ => return s.to_string(),
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let target = now - secs;
    let days = target / 86400;
    let rem = target % 86400;
    let (y, mo, d) = epoch_days_to_ymd(days);
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    format!("{y:04}-{mo:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}")
}

/// Convert epoch days to (year, month, day) — no chrono dependency.
fn epoch_days_to_ymd(days: i64) -> (i64, i64, i64) {
    // Algorithm from Howard Hinnant's civil_from_days
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn main() {
    println!("=== FJ-2001: Query Engine Enrichment Flags ===\n");

    println!("Existing flags (implemented):");
    println!("  forjar state-query \"nginx\" --history     # event history");
    println!("  forjar state-query \"nginx\" --timing      # p50/p95 latency");
    println!("  forjar state-query \"nginx\" --reversibility");
    println!("  forjar state-query \"nginx\" --drift       # drift findings");
    println!("  forjar state-query \"nginx\" --churn       # change frequency");
    println!("  forjar state-query \"nginx\" -G            # git log RRF fusion");
    println!("  forjar state-query --health              # stack-wide summary");
    println!("  forjar state-query \"nginx\" --json/--csv/--sql");

    println!("\nNew enrichment flags:");
    println!("  forjar state-query --events              # recent events");
    println!("  forjar state-query --events --since 1h   # events in last hour");
    println!("  forjar state-query --events --run <id>   # events for run");
    println!("  forjar state-query --failures            # failure history");
    println!("  forjar state-query --failures --since 7d # failures in week");
    println!("  forjar state-query \"nginx\" --status converged  # filter by status");

    // Demonstrate the resolve_since function
    println!("\n--- Relative time resolution ---\n");
    let test_cases = ["1h", "7d", "30m", "2026-03-01T00:00:00", "custom"];
    for tc in test_cases {
        let resolved = resolve_since(tc);
        println!("  --since {tc:30} => {resolved}");
    }

    // Demonstrate epoch_days_to_ymd
    println!("\n--- Date calculation (no chrono dependency) ---\n");
    let test_days: [(i64, &str); 3] = [
        (0, "Unix epoch"),
        (18262, "~2020-01-01"),
        (20520, "~2026-03-08"),
    ];
    for (days, label) in test_days {
        let (y, m, d) = epoch_days_to_ymd(days);
        println!("  day {days:>6} => {y:04}-{m:02}-{d:02}  ({label})");
    }

    println!("\n--- Event query SQL ---\n");
    println!("  SELECT run_id, resource_id, event_type, timestamp, duration_ms");
    println!("  FROM events");
    println!("  WHERE timestamp >= ?  -- --since filter");
    println!("  AND run_id = ?        -- --run filter");
    println!("  ORDER BY timestamp DESC LIMIT 50;");

    println!("\n--- Failure query SQL ---\n");
    println!("  SELECT run_id, resource_id, machine, event_type, timestamp,");
    println!("         exit_code, stderr_tail");
    println!("  FROM events");
    println!("  WHERE event_type LIKE '%failed%'");
    println!("  AND timestamp >= ?  -- --since filter");
    println!("  ORDER BY timestamp DESC LIMIT 50;");
}
