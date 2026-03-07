//! FJ-2605: Resource coverage model example.
//!
//! Demonstrates the five-level coverage model (L0–L5) for tracking
//! testing maturity of infrastructure resources.
//!
//! ```bash
//! cargo run --example coverage_model
//! ```

use forjar::core::types::{CoverageLevel, CoverageReport, ResourceCoverage};

fn main() {
    demo_coverage_levels();
    demo_coverage_report();
}

fn demo_coverage_levels() {
    println!("=== FJ-2605: Coverage Levels ===\n");

    for level in [
        CoverageLevel::L0,
        CoverageLevel::L1,
        CoverageLevel::L2,
        CoverageLevel::L3,
        CoverageLevel::L4,
        CoverageLevel::L5,
    ] {
        println!("  {level}");
    }
    println!();
}

fn demo_coverage_report() {
    println!("=== FJ-2605: Coverage Report ===\n");

    let entries = vec![
        ResourceCoverage {
            resource_id: "nginx-pkg".into(),
            level: CoverageLevel::L4,
            resource_type: "package".into(),
        },
        ResourceCoverage {
            resource_id: "nginx-config".into(),
            level: CoverageLevel::L3,
            resource_type: "file".into(),
        },
        ResourceCoverage {
            resource_id: "nginx-service".into(),
            level: CoverageLevel::L2,
            resource_type: "service".into(),
        },
        ResourceCoverage {
            resource_id: "app-deploy".into(),
            level: CoverageLevel::L1,
            resource_type: "task".into(),
        },
        ResourceCoverage {
            resource_id: "firewall-rule".into(),
            level: CoverageLevel::L0,
            resource_type: "network".into(),
        },
    ];

    let report = CoverageReport::from_entries(entries);
    print!("{}", report.format_report());

    println!(
        "Meets L1 threshold: {}",
        report.meets_threshold(CoverageLevel::L1)
    );
    println!(
        "Meets L2 threshold: {}",
        report.meets_threshold(CoverageLevel::L2)
    );
}
