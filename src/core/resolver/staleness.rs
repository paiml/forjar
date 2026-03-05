//! FJ-1270: Cross-stack staleness detection helpers.
//!
//! Parses human-readable durations like "1h", "24h", "7d" and checks
//! whether an ISO 8601 timestamp is older than the threshold.

/// Parse a human-readable duration string to seconds.
/// Supports: `Ns` (seconds), `Nm` (minutes), `Nh` (hours), `Nd` (days).
pub fn parse_duration_secs(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty duration string".to_string());
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let n: u64 = num_str
        .parse()
        .map_err(|_| format!("invalid duration number: '{}'", num_str))?;

    match unit {
        "s" => Ok(n),
        "m" => Ok(n * 60),
        "h" => Ok(n * 3600),
        "d" => Ok(n * 86400),
        _ => Err(format!(
            "unknown duration unit '{}' (use s, m, h, or d)",
            unit
        )),
    }
}

/// Check whether an ISO 8601 timestamp is stale relative to now.
/// Returns `true` if the timestamp is older than `max_secs` seconds.
pub fn is_stale(iso_ts: &str, max_secs: u64) -> bool {
    let Some(ts_epoch) = parse_iso8601_epoch(iso_ts) else {
        return false; // unparseable timestamp — don't flag as stale
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    now.saturating_sub(ts_epoch) > max_secs
}

/// Minimal ISO 8601 parser: "YYYY-MM-DDTHH:MM:SSZ" → epoch seconds.
fn parse_iso8601_epoch(s: &str) -> Option<u64> {
    // Expected format: 2026-03-01T14:30:00Z
    if s.len() < 19 {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: u64 = s.get(5..7)?.parse().ok()?;
    let day: u64 = s.get(8..10)?.parse().ok()?;
    let hour: u64 = s.get(11..13)?.parse().ok()?;
    let min: u64 = s.get(14..16)?.parse().ok()?;
    let sec: u64 = s.get(17..19)?.parse().ok()?;

    // Days from epoch (1970-01-01) to start of year
    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }

    // Days in months of current year
    let leap = is_leap(year);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for md in month_days.iter().take((month as usize).saturating_sub(1)) {
        days += *md as u64;
    }
    days += day.saturating_sub(1);

    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
