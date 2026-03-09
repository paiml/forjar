//! Example: In-process cron event source (FJ-3103)
//!
//! Demonstrates parsing cron expressions and matching them
//! against specific times.
//!
//! ```bash
//! cargo run --example cron_source
//! ```

use forjar::core::cron_source::{self, CronTime};

fn main() {
    println!("=== In-Process Cron Event Source (FJ-3103) ===\n");

    // Parse various cron expressions
    let expressions = [
        ("Every minute", "* * * * *"),
        ("Every 15 minutes", "*/15 * * * *"),
        ("Hourly at :00", "0 * * * *"),
        ("Daily at 3:30 AM", "30 3 * * *"),
        ("Weekdays at 9 AM", "0 9 * * 1-5"),
        ("Monthly on 1st at noon", "0 12 1 * *"),
        ("Every 6 hours", "0 */6 * * *"),
    ];

    println!("1. Parsed Schedules:");
    for (label, expr) in &expressions {
        match cron_source::parse_cron(expr) {
            Ok(schedule) => {
                let summary = cron_source::schedule_summary(&schedule);
                println!("  {label:<30} {expr:<20} → {summary}");
            }
            Err(e) => println!("  {label:<30} ERROR: {e}"),
        }
    }

    // Match against specific times
    println!("\n2. Schedule Matching:");
    let schedule = cron_source::parse_cron("0 9 * * 1-5").unwrap();
    let times = [
        (
            "Mon 9:00",
            CronTime {
                minute: 0,
                hour: 9,
                day: 6,
                month: 3,
                weekday: 1,
            },
        ),
        (
            "Mon 10:00",
            CronTime {
                minute: 0,
                hour: 10,
                day: 6,
                month: 3,
                weekday: 1,
            },
        ),
        (
            "Sat 9:00",
            CronTime {
                minute: 0,
                hour: 9,
                day: 8,
                month: 3,
                weekday: 6,
            },
        ),
        (
            "Sun 9:00",
            CronTime {
                minute: 0,
                hour: 9,
                day: 9,
                month: 3,
                weekday: 0,
            },
        ),
    ];

    println!("  Schedule: 0 9 * * 1-5 (weekdays at 9:00)");
    for (label, time) in &times {
        let matched = cron_source::matches(&schedule, time);
        println!(
            "    {label:<12} → {}",
            if matched { "MATCH" } else { "no match" }
        );
    }

    // Invalid expressions
    println!("\n3. Invalid Expressions:");
    let invalid = ["* *", "60 * * * *", "*/0 * * * *", "* 25-30 * * *"];
    for expr in &invalid {
        match cron_source::parse_cron(expr) {
            Ok(_) => println!("  {expr:<20} → unexpected pass"),
            Err(e) => println!("  {expr:<20} → {e}"),
        }
    }

    println!("\nDone.");
}
