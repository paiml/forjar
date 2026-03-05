//! FJ-2403: Build metrics — binary size tracking and regression detection.
//!
//! ```bash
//! cargo run --example build_metrics
//! ```

use forjar::core::types::{BuildMetrics, SizeThreshold};

fn main() {
    // Collect current build metrics from compile-time environment
    let mut current = BuildMetrics::current();
    println!("=== Current Build ===");
    println!("Version: {}", current.version);
    println!("Target:  {}", current.target);
    println!("Profile: {}", current.profile);
    println!("LTO:     {}", current.lto);
    println!();

    // Simulate binary size measurement
    current.binary_size = Some(8_500_000);
    current.dependency_count = Some(47);
    current.locked = true;

    println!("=== Build Summary ===");
    print!("{}", current.format_summary());
    println!();

    // Simulate a previous release for comparison
    let mut previous = BuildMetrics::current();
    previous.binary_size = Some(7_200_000);
    previous.dependency_count = Some(42);

    if let Some(pct) = current.size_change_pct(&previous) {
        println!("Size change from previous release: {pct:+.1}%");
    }
    println!();

    // Threshold-based regression detection
    let threshold = SizeThreshold::default();
    println!("=== Size Threshold Check ===");
    println!(
        "Max bytes: {} MB",
        threshold.max_bytes as f64 / (1024.0 * 1024.0)
    );
    println!("Max growth: {}%", threshold.max_growth_pct);
    println!();

    let violations = threshold.check(&current, Some(&previous));
    if violations.is_empty() {
        println!("PASS: No size regressions detected.");
    } else {
        println!("FAIL: {} violation(s):", violations.len());
        for v in &violations {
            println!("  - {v}");
        }
    }
    println!();

    // Demonstrate a build that exceeds the threshold
    let mut oversized = BuildMetrics::current();
    oversized.binary_size = Some(15_000_000);

    println!("=== Oversized Build Check ===");
    let violations = threshold.check(&oversized, Some(&current));
    println!("{} violation(s):", violations.len());
    for v in &violations {
        println!("  - {v}");
    }

    // JSON serialization for CI integration
    println!();
    println!("=== JSON Export (for CI) ===");
    let json = serde_json::to_string_pretty(&current).unwrap();
    println!("{json}");
}
