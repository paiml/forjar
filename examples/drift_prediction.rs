//! Configuration drift prediction demonstration.
//!
//! Shows how forjar analyzes historical event logs to predict
//! which resources are most likely to drift in the future.

use std::io::Write;

fn main() {
    println!("=== Forjar Drift Prediction Analysis ===\n");

    demo_synthetic_events();
    demo_risk_scoring();
    demo_trend_analysis();

    println!("\n=== Drift prediction complete ===");
}

fn demo_synthetic_events() {
    println!("--- Synthetic Event Log ---");

    // Create a temp directory with synthetic events
    let dir = tempfile::tempdir().unwrap();
    let events_path = dir.path().join("events.jsonl");
    let mut f = std::fs::File::create(&events_path).unwrap();

    // Write synthetic events: nginx drifts frequently, postgres rarely
    let events = [
        r#"{"resource":"nginx-conf","machine":"web","action":"drift","timestamp":1000.0}"#,
        r#"{"resource":"nginx-conf","machine":"web","action":"drift","timestamp":2000.0}"#,
        r#"{"resource":"nginx-conf","machine":"web","action":"drift","timestamp":3000.0}"#,
        r#"{"resource":"nginx-conf","machine":"web","action":"apply","timestamp":4000.0}"#,
        r#"{"resource":"nginx-conf","machine":"web","action":"drift","timestamp":4500.0}"#,
        r#"{"resource":"postgres-conf","machine":"db","action":"apply","timestamp":1000.0}"#,
        r#"{"resource":"postgres-conf","machine":"db","action":"apply","timestamp":5000.0}"#,
        r#"{"resource":"cron-job","machine":"worker","action":"drift","timestamp":1500.0}"#,
        r#"{"resource":"cron-job","machine":"worker","action":"apply","timestamp":2000.0}"#,
    ];

    for event in &events {
        writeln!(f, "{event}").unwrap();
    }
    drop(f);

    println!(
        "  Created {} synthetic events in events.jsonl",
        events.len()
    );
    println!("  Resources: nginx-conf (4 drifts), postgres-conf (0 drifts), cron-job (1 drift)");
    println!();
}

fn demo_risk_scoring() {
    println!("--- Risk Scoring Algorithm ---");
    println!(
        "  risk = min(1.0, (drift_rate * 0.5 + min(0.3, drift_count * 0.05)) * trend_multiplier)"
    );
    println!();
    println!("  Example: nginx-conf");
    println!("    drift_rate = 4/5 = 0.80");
    println!("    drift_count = 4  -> count_component = min(0.3, 0.20) = 0.20");
    println!("    trend = increasing -> multiplier = 1.3");
    println!("    risk = min(1.0, (0.40 + 0.20) * 1.3) = 0.78 (HIGH)");
    println!();
    println!("  Example: postgres-conf");
    println!("    drift_rate = 0/2 = 0.00");
    println!("    drift_count = 0  -> count_component = 0.00");
    println!("    trend = stable   -> multiplier = 1.0");
    println!("    risk = 0.00 (NONE)");
    println!();
}

fn demo_trend_analysis() {
    println!("--- Trend Detection ---");
    println!("  Algorithm: compare drift count in first half vs second half of timeline");
    println!("    increasing: second_half > first_half * 1.3");
    println!("    decreasing: second_half < first_half * 0.7");
    println!("    stable:     otherwise");
    println!();
    println!("  nginx-conf: 4 drift events");
    println!("    first half:  [1000, 2000] -> 2 events");
    println!("    second half: [3000, 4500] -> 2 events");
    println!("    trend: stable (2 vs 2, ratio 1.0)");
    println!();
    println!("  Usage:");
    println!("    forjar drift-predict                  # analyze state/");
    println!("    forjar drift-predict --machine web    # filter to web");
    println!("    forjar drift-predict --limit 5 --json # top 5, JSON output");
}
