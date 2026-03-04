//! Tests for FJ-1270: Cross-stack staleness detection.

use super::staleness::{is_stale, parse_duration_secs};

#[test]
fn parse_seconds() {
    assert_eq!(parse_duration_secs("30s").unwrap(), 30);
}

#[test]
fn parse_minutes() {
    assert_eq!(parse_duration_secs("5m").unwrap(), 300);
}

#[test]
fn parse_hours() {
    assert_eq!(parse_duration_secs("1h").unwrap(), 3600);
    assert_eq!(parse_duration_secs("24h").unwrap(), 86400);
}

#[test]
fn parse_days() {
    assert_eq!(parse_duration_secs("7d").unwrap(), 604800);
}

#[test]
fn parse_invalid_unit() {
    assert!(parse_duration_secs("5w").is_err());
}

#[test]
fn parse_empty() {
    assert!(parse_duration_secs("").is_err());
}

#[test]
fn parse_non_numeric() {
    assert!(parse_duration_secs("abch").is_err());
}

#[test]
fn stale_old_timestamp() {
    // A timestamp from 2020 should be stale with max 1 hour
    assert!(is_stale("2020-01-01T00:00:00Z", 3600));
}

#[test]
fn fresh_recent_timestamp() {
    // Use a timestamp from "now" — generate one
    let ts = crate::tripwire::eventlog::now_iso8601();
    // With a 1-hour window, a just-generated timestamp should not be stale
    assert!(!is_stale(&ts, 3600));
}

#[test]
fn no_staleness_without_field() {
    // If no max_staleness is set, nothing should be flagged
    // This is implicitly tested by the None path in resolve_forjar_state_source
    // Here we just verify is_stale returns false for unparseable input
    assert!(!is_stale("not-a-timestamp", 3600));
}

#[test]
fn boundary_exactly_at_threshold() {
    // A timestamp from exactly max_secs ago should NOT be stale (> not >=)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Generate a timestamp exactly 3600 seconds ago
    let secs_ago = now - 3600;
    let ts = epoch_to_iso(secs_ago);
    assert!(!is_stale(&ts, 3600)); // exactly at threshold = not stale
}

#[test]
fn stale_just_past_threshold() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let ts = epoch_to_iso(now - 3601);
    assert!(is_stale(&ts, 3600));
}

/// Helper: convert epoch seconds to ISO 8601 string for testing.
fn epoch_to_iso(secs: u64) -> String {
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let year_days = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
            366
        } else {
            365
        };
        if remaining < year_days {
            break;
        }
        remaining -= year_days;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
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
    let mut m = 0;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining < md as i64 {
            m = i + 1;
            break;
        }
        remaining -= md as i64;
    }
    let d = remaining + 1;
    format!(
        "{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z"
    )
}
