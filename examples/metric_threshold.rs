//! Example: Metric threshold polling and system metric collection (FJ-3105)
//!
//! Demonstrates metric threshold evaluation with consecutive
//! violation tracking, plus live system metric collection via
//! the `metric_collector` module.
//!
//! ```bash
//! cargo run --example metric_threshold
//! ```

use forjar::core::metric_collector;
use forjar::core::metric_source::{self, MetricThreshold, ThresholdOp, ThresholdTracker};
use std::collections::HashMap;

fn main() {
    println!("=== Metric Threshold Polling & Collection (FJ-3105) ===\n");

    // 1. Collect live system metrics via metric_collector
    println!("1. Live System Metrics (metric_collector):");
    let live_metrics = metric_collector::collect_system_metrics();
    if live_metrics.is_empty() {
        println!("   (no metrics available — non-Linux or /proc unreadable)");
    } else {
        let mut keys: Vec<_> = live_metrics.keys().collect();
        keys.sort();
        for key in keys {
            println!("   {:<24} = {:>8.2}", key, live_metrics[key]);
        }
    }

    // 2. Define thresholds
    let thresholds = vec![
        MetricThreshold {
            name: "cpu_percent".into(),
            operator: ThresholdOp::Gt,
            value: 80.0,
            consecutive: 3, // Fire after 3 consecutive violations
        },
        MetricThreshold {
            name: "memory_percent".into(),
            operator: ThresholdOp::Gt,
            value: 90.0,
            consecutive: 1, // Fire immediately
        },
        MetricThreshold {
            name: "disk_free_gb".into(),
            operator: ThresholdOp::Lt,
            value: 10.0,
            consecutive: 2,
        },
    ];

    println!("\n2. Threshold Definitions:");
    for t in &thresholds {
        println!(
            "  {} {} {} (consecutive: {})",
            t.name, t.operator, t.value, t.consecutive
        );
    }

    // 3. Evaluate live metrics against thresholds
    println!("\n3. Live Metrics vs Thresholds:");
    if !live_metrics.is_empty() {
        let mut tracker = ThresholdTracker::default();
        let results = metric_source::evaluate_metrics(&thresholds, &live_metrics, &mut tracker);
        for r in &results {
            let status = if r.should_fire {
                "FIRE"
            } else if r.violated {
                "violated"
            } else {
                "ok"
            };
            println!(
                "    {:<18} = {:>5.1} ({} {:.1}) -> {}",
                r.name, r.current, r.operator, r.threshold, status
            );
        }
    } else {
        println!("   (skipped — no live metrics)");
    }

    // 4. Simulate metric readings over time
    let readings = vec![
        (
            "T=0",
            vec![
                ("cpu_percent", 75.0),
                ("memory_percent", 85.0),
                ("disk_free_gb", 15.0),
            ],
        ),
        (
            "T=1",
            vec![
                ("cpu_percent", 82.0),
                ("memory_percent", 88.0),
                ("disk_free_gb", 12.0),
            ],
        ),
        (
            "T=2",
            vec![
                ("cpu_percent", 85.0),
                ("memory_percent", 92.0),
                ("disk_free_gb", 8.0),
            ],
        ),
        (
            "T=3",
            vec![
                ("cpu_percent", 90.0),
                ("memory_percent", 75.0),
                ("disk_free_gb", 5.0),
            ],
        ),
        (
            "T=4",
            vec![
                ("cpu_percent", 70.0),
                ("memory_percent", 70.0),
                ("disk_free_gb", 20.0),
            ],
        ),
    ];

    let mut tracker = ThresholdTracker::default();

    println!("\n4. Simulated Evaluation Over Time:");
    for (label, metrics) in &readings {
        let values: HashMap<String, f64> =
            metrics.iter().map(|(k, v)| (k.to_string(), *v)).collect();

        let results = metric_source::evaluate_metrics(&thresholds, &values, &mut tracker);

        println!("\n  [{label}]");
        for r in &results {
            let status = if r.should_fire {
                "FIRE"
            } else if r.violated {
                "violated"
            } else {
                "ok"
            };
            println!(
                "    {:<18} = {:>5.1} ({} {:.1}) -> {}",
                r.name, r.current, r.operator, r.threshold, status
            );
        }
    }

    println!("\nDone.");
}
