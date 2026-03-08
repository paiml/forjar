//! Tests for FJ-2001 query format enrichment flags.

#[test]
fn test_resolve_since_hours() {
    let result = super::query_format::resolve_since("1h");
    // Should be a valid ISO timestamp in the past
    assert!(result.contains('T'));
    assert!(result.len() >= 19);
}

#[test]
fn test_resolve_since_days() {
    let result = super::query_format::resolve_since("7d");
    assert!(result.contains('T'));
}

#[test]
fn test_resolve_since_minutes() {
    let result = super::query_format::resolve_since("30m");
    assert!(result.contains('T'));
}

#[test]
fn test_resolve_since_iso_passthrough() {
    let result = super::query_format::resolve_since("2026-03-01T00:00:00");
    assert_eq!(result, "2026-03-01T00:00:00");
}

#[test]
fn test_resolve_since_invalid_returns_as_is() {
    let result = super::query_format::resolve_since("yesterday");
    assert_eq!(result, "yesterday");
}

#[test]
fn test_epoch_days_to_ymd_epoch() {
    let (y, m, d) = super::query_format::epoch_days_to_ymd(0);
    assert_eq!((y, m, d), (1970, 1, 1));
}

#[test]
fn test_epoch_days_to_ymd_known_date() {
    // 2026-03-08 is day 20520 since epoch
    let days = (2026 - 1970) * 365 + 14 + 31 + 28 + 8; // approximate
    let (y, _m, _d) = super::query_format::epoch_days_to_ymd(days);
    assert_eq!(y, 2026);
}

#[test]
fn test_chrono_now_minus_seconds() {
    let result = super::query_format::chrono_now_minus_seconds(3600);
    assert!(result.contains('T'));
    assert!(result.len() >= 19);
}
