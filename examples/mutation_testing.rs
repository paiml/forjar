//! FJ-2604: Infrastructure mutation testing — operators, scoring, reports.
//!
//! ```bash
//! cargo run --example mutation_testing
//! ```

use forjar::core::types::{
    MutationOperator, MutationReport, MutationResult, MutationScore,
};

fn main() {
    // Mutation operators
    println!("=== Mutation Operators ===");
    for op in [
        MutationOperator::DeleteFile,
        MutationOperator::ModifyContent,
        MutationOperator::ChangePermissions,
        MutationOperator::StopService,
        MutationOperator::RemovePackage,
        MutationOperator::KillProcess,
        MutationOperator::UnmountFilesystem,
        MutationOperator::CorruptConfig,
    ] {
        println!(
            "  {}: {} (applies to: {:?})",
            op,
            op.description(),
            op.applicable_types()
        );
    }
    println!();

    // Simulate mutation test results
    let results = vec![
        MutationResult {
            resource_id: "nginx-config".into(),
            resource_type: "file".into(),
            operator: MutationOperator::DeleteFile,
            detected: true,
            reconverged: Some(true),
            duration_ms: 120,
            error: None,
        },
        MutationResult {
            resource_id: "nginx-config".into(),
            resource_type: "file".into(),
            operator: MutationOperator::ModifyContent,
            detected: true,
            reconverged: Some(true),
            duration_ms: 95,
            error: None,
        },
        MutationResult {
            resource_id: "nginx-config".into(),
            resource_type: "file".into(),
            operator: MutationOperator::ChangePermissions,
            detected: true,
            reconverged: Some(true),
            duration_ms: 80,
            error: None,
        },
        MutationResult {
            resource_id: "nginx-service".into(),
            resource_type: "service".into(),
            operator: MutationOperator::StopService,
            detected: true,
            reconverged: Some(true),
            duration_ms: 230,
            error: None,
        },
        MutationResult {
            resource_id: "nginx-pkg".into(),
            resource_type: "package".into(),
            operator: MutationOperator::RemovePackage,
            detected: false,
            reconverged: None,
            duration_ms: 180,
            error: None,
        },
        MutationResult {
            resource_id: "data-mount".into(),
            resource_type: "mount".into(),
            operator: MutationOperator::UnmountFilesystem,
            detected: true,
            reconverged: Some(true),
            duration_ms: 310,
            error: None,
        },
    ];

    println!("=== Individual Results ===");
    for r in &results {
        println!("  {r}");
    }
    println!();

    // Build full report
    let report = MutationReport::from_results(results);
    println!("=== Mutation Report ===");
    print!("{}", report.format_summary());
    println!();

    // Score calculation demo
    println!("=== Score Grades ===");
    let scores = vec![
        MutationScore { total: 10, detected: 10, survived: 0, errored: 0 },
        MutationScore { total: 10, detected: 9, survived: 1, errored: 0 },
        MutationScore { total: 10, detected: 8, survived: 2, errored: 0 },
        MutationScore { total: 10, detected: 6, survived: 4, errored: 0 },
        MutationScore { total: 10, detected: 5, survived: 5, errored: 0 },
    ];
    for s in &scores {
        println!("  {:.0}% -> Grade {}", s.score_pct(), s.grade());
    }
}
