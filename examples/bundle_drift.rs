//! Example: Bundle size drift detection
//!
//! Demonstrates WASM bundle size monitoring for deployment pipelines.
//! Alerts when bundles exceed budget or grow more than 20% between builds.

use forjar::core::types::{BundleSizeDrift, WasmSizeBudget};

fn main() {
    let budget = WasmSizeBudget {
        core_kb: 100,
        widgets_kb: 150,
        full_app_kb: 500,
    };

    // Case 1: Within budget, modest growth
    let drift = BundleSizeDrift::check(&budget, 90 * 1024, Some(85 * 1024));
    println!("Case 1 — {drift}");
    assert!(drift.is_ok());

    // Case 2: Exceeds budget
    let drift = BundleSizeDrift::check(&budget, 110 * 1024, Some(90 * 1024));
    println!("Case 2 — {drift}");
    assert!(!drift.is_ok());
    assert!(drift.exceeds_budget);

    // Case 3: Within budget but growth >20%
    let large_budget = WasmSizeBudget {
        core_kb: 200,
        ..Default::default()
    };
    let drift = BundleSizeDrift::check(&large_budget, 130 * 1024, Some(100 * 1024));
    println!("Case 3 — {drift}");
    assert!(!drift.is_ok());
    assert!(drift.exceeds_growth_limit);

    // Case 4: First build (no previous)
    let drift = BundleSizeDrift::check(&budget, 90 * 1024, None);
    println!("Case 4 — {drift}");
    assert!(drift.is_ok());

    println!("\nAll drift checks complete.");
}
