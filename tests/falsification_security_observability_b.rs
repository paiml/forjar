//! FJ-2300/2301: Popperian falsification for security model and observability.
//!
//! Each test states conditions under which the security or observability
//! system would be rejected as invalid.
#![allow(clippy::field_reassign_with_default)]

use forjar::core::types::{CoverageLevel, CoverageReport, ResourceCoverage};

// ── FJ-2300: Security Model ────────────────────────────────────────

#[test]
fn f_2605_3_coverage_report_histogram() {
    let entries = vec![
        ResourceCoverage {
            resource_id: "a".into(),
            level: CoverageLevel::L5,
            resource_type: "file".into(),
        },
        ResourceCoverage {
            resource_id: "b".into(),
            level: CoverageLevel::L5,
            resource_type: "file".into(),
        },
        ResourceCoverage {
            resource_id: "c".into(),
            level: CoverageLevel::L0,
            resource_type: "task".into(),
        },
    ];
    let report = CoverageReport::from_entries(entries);
    assert_eq!(report.histogram[0], 1); // L0
    assert_eq!(report.histogram[5], 2); // L5
    assert_eq!(report.min_level, CoverageLevel::L0);
}
