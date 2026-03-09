//! FJ-3103: In-process cron event source.
//!
//! Parses cron expressions (minute/hour/dom/month/dow) and determines
//! whether a schedule matches a given datetime. No system crontab —
//! purely in-process evaluation for `forjar watch` daemon.

use std::collections::BTreeSet;

/// A parsed cron schedule (minute hour dom month dow).
#[derive(Debug, Clone)]
pub struct CronSchedule {
    /// Minutes (0-59).
    pub minutes: BTreeSet<u8>,
    /// Hours (0-23).
    pub hours: BTreeSet<u8>,
    /// Days of month (1-31).
    pub days_of_month: BTreeSet<u8>,
    /// Months (1-12).
    pub months: BTreeSet<u8>,
    /// Days of week (0-6, 0=Sunday).
    pub days_of_week: BTreeSet<u8>,
}

/// A point in time for cron matching (no external datetime crate).
#[derive(Debug, Clone)]
pub struct CronTime {
    /// Minute (0-59).
    pub minute: u8,
    /// Hour (0-23).
    pub hour: u8,
    /// Day of month (1-31).
    pub day: u8,
    /// Month (1-12).
    pub month: u8,
    /// Day of week (0-6, 0=Sunday).
    pub weekday: u8,
}

/// Parse a cron expression string (5 fields: min hour dom month dow).
///
/// Supports: `*`, exact values, ranges (`1-5`), steps (`*/5`),
/// and comma-separated lists (`1,3,5`).
pub fn parse_cron(expr: &str) -> Result<CronSchedule, String> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(format!(
            "cron expression must have 5 fields (got {}): {expr}",
            parts.len()
        ));
    }

    Ok(CronSchedule {
        minutes: parse_field(parts[0], 0, 59)?,
        hours: parse_field(parts[1], 0, 23)?,
        days_of_month: parse_field(parts[2], 1, 31)?,
        months: parse_field(parts[3], 1, 12)?,
        days_of_week: parse_field(parts[4], 0, 6)?,
    })
}

/// Check if a cron schedule matches a given time.
pub fn matches(schedule: &CronSchedule, time: &CronTime) -> bool {
    schedule.minutes.contains(&time.minute)
        && schedule.hours.contains(&time.hour)
        && schedule.days_of_month.contains(&time.day)
        && schedule.months.contains(&time.month)
        && schedule.days_of_week.contains(&time.weekday)
}

/// Parse a single cron field (e.g., `*/5`, `1-3`, `1,2,3`, `*`).
fn parse_field(field: &str, min: u8, max: u8) -> Result<BTreeSet<u8>, String> {
    let mut values = BTreeSet::new();

    for part in field.split(',') {
        if part == "*" {
            for v in min..=max {
                values.insert(v);
            }
        } else if let Some(step_str) = part.strip_prefix("*/") {
            let step: u8 = step_str
                .parse()
                .map_err(|_| format!("invalid step: {part}"))?;
            if step == 0 {
                return Err(format!("step cannot be 0: {part}"));
            }
            let mut v = min;
            while v <= max {
                values.insert(v);
                v = v.saturating_add(step);
            }
        } else if part.contains('-') {
            let (start_str, end_str) = part
                .split_once('-')
                .ok_or_else(|| format!("invalid range: {part}"))?;
            let start: u8 = start_str
                .parse()
                .map_err(|_| format!("invalid range start: {part}"))?;
            let end: u8 = end_str
                .parse()
                .map_err(|_| format!("invalid range end: {part}"))?;
            if start > end || start < min || end > max {
                return Err(format!("range out of bounds: {part} (valid: {min}-{max})"));
            }
            for v in start..=end {
                values.insert(v);
            }
        } else {
            let v: u8 = part.parse().map_err(|_| format!("invalid value: {part}"))?;
            if v < min || v > max {
                return Err(format!("value {v} out of bounds ({min}-{max})"));
            }
            values.insert(v);
        }
    }

    Ok(values)
}

/// Summary of a parsed cron schedule for display.
pub fn schedule_summary(schedule: &CronSchedule) -> String {
    format!(
        "min={} hr={} dom={} mon={} dow={}",
        set_summary(&schedule.minutes),
        set_summary(&schedule.hours),
        set_summary(&schedule.days_of_month),
        set_summary(&schedule.months),
        set_summary(&schedule.days_of_week),
    )
}

fn set_summary(set: &BTreeSet<u8>) -> String {
    if set.len() > 10 {
        format!("*({} values)", set.len())
    } else {
        let vals: Vec<String> = set.iter().map(|v| v.to_string()).collect();
        vals.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_star() {
        let s = parse_cron("* * * * *").unwrap();
        assert_eq!(s.minutes.len(), 60);
        assert_eq!(s.hours.len(), 24);
        assert_eq!(s.days_of_month.len(), 31);
        assert_eq!(s.months.len(), 12);
        assert_eq!(s.days_of_week.len(), 7);
    }

    #[test]
    fn parse_exact() {
        let s = parse_cron("30 12 15 6 3").unwrap();
        assert_eq!(s.minutes, BTreeSet::from([30]));
        assert_eq!(s.hours, BTreeSet::from([12]));
        assert_eq!(s.days_of_month, BTreeSet::from([15]));
        assert_eq!(s.months, BTreeSet::from([6]));
        assert_eq!(s.days_of_week, BTreeSet::from([3]));
    }

    #[test]
    fn parse_step() {
        let s = parse_cron("*/15 * * * *").unwrap();
        assert_eq!(s.minutes, BTreeSet::from([0, 15, 30, 45]));
    }

    #[test]
    fn parse_range() {
        let s = parse_cron("* 9-17 * * *").unwrap();
        assert_eq!(s.hours, BTreeSet::from([9, 10, 11, 12, 13, 14, 15, 16, 17]));
    }

    #[test]
    fn parse_list() {
        let s = parse_cron("0 6,12,18 * * *").unwrap();
        assert_eq!(s.minutes, BTreeSet::from([0]));
        assert_eq!(s.hours, BTreeSet::from([6, 12, 18]));
    }

    #[test]
    fn parse_mixed() {
        let s = parse_cron("0,30 */6 1-15 * 1-5").unwrap();
        assert_eq!(s.minutes, BTreeSet::from([0, 30]));
        assert_eq!(s.hours, BTreeSet::from([0, 6, 12, 18]));
        assert!(s.days_of_month.contains(&1));
        assert!(s.days_of_month.contains(&15));
        assert!(!s.days_of_month.contains(&16));
        assert_eq!(s.days_of_week, BTreeSet::from([1, 2, 3, 4, 5]));
    }

    #[test]
    fn parse_invalid_fields() {
        assert!(parse_cron("* *").is_err());
        assert!(parse_cron("* * * * * *").is_err());
    }

    #[test]
    fn parse_invalid_value() {
        assert!(parse_cron("60 * * * *").is_err());
        assert!(parse_cron("* 25 * * *").is_err());
        assert!(parse_cron("* * 0 * *").is_err());
        assert!(parse_cron("* * * 13 *").is_err());
        assert!(parse_cron("* * * * 7").is_err());
    }

    #[test]
    fn parse_invalid_step() {
        assert!(parse_cron("*/0 * * * *").is_err());
    }

    #[test]
    fn parse_invalid_range() {
        assert!(parse_cron("* 17-9 * * *").is_err());
    }

    #[test]
    fn matches_exact() {
        let s = parse_cron("30 12 15 6 3").unwrap();
        let t = CronTime {
            minute: 30,
            hour: 12,
            day: 15,
            month: 6,
            weekday: 3,
        };
        assert!(matches(&s, &t));
    }

    #[test]
    fn matches_not() {
        let s = parse_cron("30 12 15 6 3").unwrap();
        let t = CronTime {
            minute: 31,
            hour: 12,
            day: 15,
            month: 6,
            weekday: 3,
        };
        assert!(!matches(&s, &t));
    }

    #[test]
    fn matches_every_minute() {
        let s = parse_cron("* * * * *").unwrap();
        let t = CronTime {
            minute: 42,
            hour: 3,
            day: 28,
            month: 2,
            weekday: 0,
        };
        assert!(matches(&s, &t));
    }

    #[test]
    fn matches_step_hit() {
        let s = parse_cron("*/15 * * * *").unwrap();
        let t = CronTime {
            minute: 45,
            hour: 0,
            day: 1,
            month: 1,
            weekday: 0,
        };
        assert!(matches(&s, &t));
    }

    #[test]
    fn matches_step_miss() {
        let s = parse_cron("*/15 * * * *").unwrap();
        let t = CronTime {
            minute: 7,
            hour: 0,
            day: 1,
            month: 1,
            weekday: 0,
        };
        assert!(!matches(&s, &t));
    }

    #[test]
    fn matches_weekday_range() {
        let s = parse_cron("0 9 * * 1-5").unwrap();
        // Monday at 9:00
        assert!(matches(
            &s,
            &CronTime {
                minute: 0,
                hour: 9,
                day: 1,
                month: 1,
                weekday: 1
            }
        ));
        // Sunday at 9:00
        assert!(!matches(
            &s,
            &CronTime {
                minute: 0,
                hour: 9,
                day: 1,
                month: 1,
                weekday: 0
            }
        ));
    }

    #[test]
    fn summary_format() {
        let s = parse_cron("0 12 * * *").unwrap();
        let summary = schedule_summary(&s);
        assert!(summary.contains("min=0"));
        assert!(summary.contains("hr=12"));
    }

    #[test]
    fn summary_star_condensed() {
        let s = parse_cron("* * * * *").unwrap();
        let summary = schedule_summary(&s);
        assert!(summary.contains("*(60 values)"));
    }
}
