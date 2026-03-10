//! FJ-3103: Cron event source falsification.
//!
//! Popperian rejection criteria for:
//! - Cron expression parsing (5-field: min hour dom month dow)
//! - Wildcard, exact, range, step, list, mixed field parsing
//! - Schedule matching against CronTime
//! - Schedule summary formatting
//! - Boundary and error conditions
//!
//! Usage: cargo test --test falsification_cron_source

use forjar::core::cron_source::{matches, parse_cron, schedule_summary, CronTime};
use std::collections::BTreeSet;

fn time(minute: u8, hour: u8, day: u8, month: u8, weekday: u8) -> CronTime {
    CronTime {
        minute,
        hour,
        day,
        month,
        weekday,
    }
}

// ============================================================================
// FJ-3103: Parse — Wildcards
// ============================================================================

#[test]
fn parse_all_star_produces_full_ranges() {
    let s = parse_cron("* * * * *").unwrap();
    assert_eq!(s.minutes.len(), 60); // 0-59
    assert_eq!(s.hours.len(), 24); // 0-23
    assert_eq!(s.days_of_month.len(), 31); // 1-31
    assert_eq!(s.months.len(), 12); // 1-12
    assert_eq!(s.days_of_week.len(), 7); // 0-6
}

#[test]
fn parse_star_minute_contains_boundaries() {
    let s = parse_cron("* 0 1 1 0").unwrap();
    assert!(s.minutes.contains(&0));
    assert!(s.minutes.contains(&59));
}

#[test]
fn parse_star_hour_contains_boundaries() {
    let s = parse_cron("0 * 1 1 0").unwrap();
    assert!(s.hours.contains(&0));
    assert!(s.hours.contains(&23));
}

#[test]
fn parse_star_dom_starts_at_1() {
    let s = parse_cron("0 0 * 1 0").unwrap();
    assert!(!s.days_of_month.contains(&0));
    assert!(s.days_of_month.contains(&1));
    assert!(s.days_of_month.contains(&31));
}

#[test]
fn parse_star_month_starts_at_1() {
    let s = parse_cron("0 0 1 * 0").unwrap();
    assert!(!s.months.contains(&0));
    assert!(s.months.contains(&1));
    assert!(s.months.contains(&12));
}

#[test]
fn parse_star_dow_starts_at_0() {
    let s = parse_cron("0 0 1 1 *").unwrap();
    assert!(s.days_of_week.contains(&0));
    assert!(s.days_of_week.contains(&6));
}

// ============================================================================
// FJ-3103: Parse — Exact Values
// ============================================================================

#[test]
fn parse_exact_single_values() {
    let s = parse_cron("30 12 15 6 3").unwrap();
    assert_eq!(s.minutes, BTreeSet::from([30]));
    assert_eq!(s.hours, BTreeSet::from([12]));
    assert_eq!(s.days_of_month, BTreeSet::from([15]));
    assert_eq!(s.months, BTreeSet::from([6]));
    assert_eq!(s.days_of_week, BTreeSet::from([3]));
}

#[test]
fn parse_exact_boundary_min() {
    let s = parse_cron("0 0 1 1 0").unwrap();
    assert_eq!(s.minutes, BTreeSet::from([0]));
    assert_eq!(s.hours, BTreeSet::from([0]));
    assert_eq!(s.days_of_month, BTreeSet::from([1]));
    assert_eq!(s.months, BTreeSet::from([1]));
    assert_eq!(s.days_of_week, BTreeSet::from([0]));
}

#[test]
fn parse_exact_boundary_max() {
    let s = parse_cron("59 23 31 12 6").unwrap();
    assert_eq!(s.minutes, BTreeSet::from([59]));
    assert_eq!(s.hours, BTreeSet::from([23]));
    assert_eq!(s.days_of_month, BTreeSet::from([31]));
    assert_eq!(s.months, BTreeSet::from([12]));
    assert_eq!(s.days_of_week, BTreeSet::from([6]));
}

// ============================================================================
// FJ-3103: Parse — Ranges
// ============================================================================

#[test]
fn parse_range_hours() {
    let s = parse_cron("0 9-17 * * *").unwrap();
    assert_eq!(s.hours.len(), 9);
    assert!(s.hours.contains(&9));
    assert!(s.hours.contains(&17));
    assert!(!s.hours.contains(&8));
    assert!(!s.hours.contains(&18));
}

#[test]
fn parse_range_weekdays_mon_fri() {
    let s = parse_cron("0 9 * * 1-5").unwrap();
    assert_eq!(s.days_of_week, BTreeSet::from([1, 2, 3, 4, 5]));
}

#[test]
fn parse_range_single_value() {
    let s = parse_cron("0 5-5 * * *").unwrap();
    assert_eq!(s.hours, BTreeSet::from([5]));
}

// ============================================================================
// FJ-3103: Parse — Steps
// ============================================================================

#[test]
fn parse_step_every_15_minutes() {
    let s = parse_cron("*/15 * * * *").unwrap();
    assert_eq!(s.minutes, BTreeSet::from([0, 15, 30, 45]));
}

#[test]
fn parse_step_every_6_hours() {
    let s = parse_cron("0 */6 * * *").unwrap();
    assert_eq!(s.hours, BTreeSet::from([0, 6, 12, 18]));
}

#[test]
fn parse_step_every_2_months() {
    let s = parse_cron("0 0 1 */2 *").unwrap();
    assert_eq!(s.months, BTreeSet::from([1, 3, 5, 7, 9, 11]));
}

#[test]
fn parse_step_every_1_same_as_star() {
    let s_step = parse_cron("*/1 * * * *").unwrap();
    let s_star = parse_cron("* * * * *").unwrap();
    assert_eq!(s_step.minutes, s_star.minutes);
}

// ============================================================================
// FJ-3103: Parse — Lists
// ============================================================================

#[test]
fn parse_list_values() {
    let s = parse_cron("0 6,12,18 * * *").unwrap();
    assert_eq!(s.hours, BTreeSet::from([6, 12, 18]));
}

#[test]
fn parse_list_single_element() {
    let s = parse_cron("5 * * * *").unwrap();
    assert_eq!(s.minutes, BTreeSet::from([5]));
}

// ============================================================================
// FJ-3103: Parse — Mixed
// ============================================================================

#[test]
fn parse_mixed_list_and_range() {
    let s = parse_cron("0,30 */6 1-15 * 1-5").unwrap();
    assert_eq!(s.minutes, BTreeSet::from([0, 30]));
    assert_eq!(s.hours, BTreeSet::from([0, 6, 12, 18]));
    assert_eq!(s.days_of_month.len(), 15);
    assert!(s.days_of_month.contains(&1));
    assert!(s.days_of_month.contains(&15));
    assert!(!s.days_of_month.contains(&16));
    assert_eq!(s.days_of_week, BTreeSet::from([1, 2, 3, 4, 5]));
}

// ============================================================================
// FJ-3103: Parse — Errors
// ============================================================================

#[test]
fn parse_too_few_fields() {
    let err = parse_cron("* *").unwrap_err();
    assert!(err.contains("5 fields"), "error: {err}");
}

#[test]
fn parse_too_many_fields() {
    let err = parse_cron("* * * * * *").unwrap_err();
    assert!(err.contains("5 fields"), "error: {err}");
}

#[test]
fn parse_empty_string() {
    assert!(parse_cron("").is_err());
}

#[test]
fn parse_minute_out_of_range() {
    assert!(parse_cron("60 * * * *").is_err());
}

#[test]
fn parse_hour_out_of_range() {
    assert!(parse_cron("* 25 * * *").is_err());
}

#[test]
fn parse_dom_zero_out_of_range() {
    assert!(parse_cron("* * 0 * *").is_err());
}

#[test]
fn parse_dom_32_out_of_range() {
    assert!(parse_cron("* * 32 * *").is_err());
}

#[test]
fn parse_month_zero_out_of_range() {
    assert!(parse_cron("* * * 0 *").is_err());
}

#[test]
fn parse_month_13_out_of_range() {
    assert!(parse_cron("* * * 13 *").is_err());
}

#[test]
fn parse_dow_7_out_of_range() {
    assert!(parse_cron("* * * * 7").is_err());
}

#[test]
fn parse_step_zero_rejected() {
    let err = parse_cron("*/0 * * * *").unwrap_err();
    assert!(err.contains("0"), "error: {err}");
}

#[test]
fn parse_reversed_range_rejected() {
    let err = parse_cron("* 17-9 * * *").unwrap_err();
    assert!(err.contains("range"), "error: {err}");
}

#[test]
fn parse_non_numeric_rejected() {
    assert!(parse_cron("abc * * * *").is_err());
}

// ============================================================================
// FJ-3103: Schedule Matching
// ============================================================================

#[test]
fn matches_exact_all_fields() {
    let s = parse_cron("30 12 15 6 3").unwrap();
    assert!(matches(&s, &time(30, 12, 15, 6, 3)));
}

#[test]
fn matches_fails_wrong_minute() {
    let s = parse_cron("30 12 15 6 3").unwrap();
    assert!(!matches(&s, &time(31, 12, 15, 6, 3)));
}

#[test]
fn matches_fails_wrong_hour() {
    let s = parse_cron("30 12 15 6 3").unwrap();
    assert!(!matches(&s, &time(30, 11, 15, 6, 3)));
}

#[test]
fn matches_fails_wrong_day() {
    let s = parse_cron("30 12 15 6 3").unwrap();
    assert!(!matches(&s, &time(30, 12, 14, 6, 3)));
}

#[test]
fn matches_fails_wrong_month() {
    let s = parse_cron("30 12 15 6 3").unwrap();
    assert!(!matches(&s, &time(30, 12, 15, 7, 3)));
}

#[test]
fn matches_fails_wrong_weekday() {
    let s = parse_cron("30 12 15 6 3").unwrap();
    assert!(!matches(&s, &time(30, 12, 15, 6, 4)));
}

#[test]
fn matches_star_any_time() {
    let s = parse_cron("* * * * *").unwrap();
    assert!(matches(&s, &time(0, 0, 1, 1, 0)));
    assert!(matches(&s, &time(59, 23, 31, 12, 6)));
    assert!(matches(&s, &time(42, 3, 28, 2, 0)));
}

#[test]
fn matches_step_hit() {
    let s = parse_cron("*/15 * * * *").unwrap();
    assert!(matches(&s, &time(0, 0, 1, 1, 0)));
    assert!(matches(&s, &time(15, 0, 1, 1, 0)));
    assert!(matches(&s, &time(30, 0, 1, 1, 0)));
    assert!(matches(&s, &time(45, 0, 1, 1, 0)));
}

#[test]
fn matches_step_miss() {
    let s = parse_cron("*/15 * * * *").unwrap();
    assert!(!matches(&s, &time(7, 0, 1, 1, 0)));
    assert!(!matches(&s, &time(14, 0, 1, 1, 0)));
    assert!(!matches(&s, &time(16, 0, 1, 1, 0)));
}

#[test]
fn matches_weekday_business_hours() {
    let s = parse_cron("0 9 * * 1-5").unwrap();
    // Monday at 9:00
    assert!(matches(&s, &time(0, 9, 1, 1, 1)));
    // Friday at 9:00
    assert!(matches(&s, &time(0, 9, 5, 1, 5)));
    // Sunday at 9:00
    assert!(!matches(&s, &time(0, 9, 7, 1, 0)));
    // Saturday at 9:00
    assert!(!matches(&s, &time(0, 9, 6, 1, 6)));
    // Monday at 10:00
    assert!(!matches(&s, &time(0, 10, 1, 1, 1)));
}

#[test]
fn matches_quarterly_first_day() {
    // First of Jan, Apr, Jul, Oct at midnight
    let s = parse_cron("0 0 1 1,4,7,10 *").unwrap();
    assert!(matches(&s, &time(0, 0, 1, 1, 3)));
    assert!(matches(&s, &time(0, 0, 1, 4, 1)));
    assert!(!matches(&s, &time(0, 0, 1, 2, 5)));
    assert!(!matches(&s, &time(0, 0, 2, 1, 4)));
}

// ============================================================================
// FJ-3103: Schedule Summary
// ============================================================================

#[test]
fn summary_small_sets_listed() {
    let s = parse_cron("0 12 * * *").unwrap();
    let summary = schedule_summary(&s);
    assert!(summary.contains("min=0"));
    assert!(summary.contains("hr=12"));
}

#[test]
fn summary_large_sets_condensed() {
    let s = parse_cron("* * * * *").unwrap();
    let summary = schedule_summary(&s);
    assert!(summary.contains("*(60 values)")); // minutes > 10
    assert!(summary.contains("*(24 values)")); // hours > 10
    assert!(summary.contains("*(31 values)")); // dom > 10
    assert!(summary.contains("*(12 values)")); // months > 10
                                               // dow has only 7 values (≤ 10), so it's listed not condensed
    assert!(summary.contains("dow=0,1,2,3,4,5,6"));
}

#[test]
fn summary_list_format() {
    let s = parse_cron("0 6,12,18 * * *").unwrap();
    let summary = schedule_summary(&s);
    assert!(summary.contains("6,12,18"));
}

// ============================================================================
// FJ-3103: CronSchedule properties
// ============================================================================

#[test]
fn cron_schedule_clone_eq() {
    let s = parse_cron("*/5 9-17 * * 1-5").unwrap();
    let cloned = s.clone();
    assert_eq!(s.minutes, cloned.minutes);
    assert_eq!(s.hours, cloned.hours);
    assert_eq!(s.days_of_month, cloned.days_of_month);
    assert_eq!(s.months, cloned.months);
    assert_eq!(s.days_of_week, cloned.days_of_week);
}

#[test]
fn cron_schedule_debug() {
    let s = parse_cron("0 0 1 1 0").unwrap();
    let debug = format!("{s:?}");
    assert!(debug.contains("CronSchedule"));
}

#[test]
fn cron_time_clone_eq() {
    let t = time(30, 12, 15, 6, 3);
    let cloned = t.clone();
    assert_eq!(t.minute, cloned.minute);
    assert_eq!(t.hour, cloned.hour);
    assert_eq!(t.day, cloned.day);
    assert_eq!(t.month, cloned.month);
    assert_eq!(t.weekday, cloned.weekday);
}

#[test]
fn cron_time_debug() {
    let t = time(0, 0, 1, 1, 0);
    let debug = format!("{t:?}");
    assert!(debug.contains("CronTime"));
}
