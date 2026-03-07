//! Demonstrates FJ-2602 test runner types: test commands, sandbox config, reports.

use forjar::core::types::{
    BadgeColor, CoverageBadge, CoverageThreshold, SandboxBackend, SandboxConfig, TestArtifact,
    TestCommand, TestResult, TestSubcommand, TestSuiteReport,
};

fn main() {
    // Unified test command
    println!("=== Test Command ===");
    let cmd = TestCommand {
        subcommand: TestSubcommand::All,
        config: "forjar.yaml".into(),
        parallel: true,
        json: false,
        verbose: true,
    };
    println!("  Subcommand: {}", cmd.subcommand);
    println!("  Parallel: {}", cmd.parallel);

    // Sandbox configuration
    println!("\n=== Sandbox Config ===");
    let sandbox = SandboxConfig::default();
    println!("  Backend: {}", sandbox.backend);
    println!("  Cleanup: {}", sandbox.cleanup);
    println!("  Timeout: {}s", sandbox.timeout_secs);

    let container = SandboxConfig {
        backend: SandboxBackend::Container,
        cleanup: true,
        timeout_secs: 600,
        capture_overlay: true,
    };
    println!("  Container backend: {}", container.backend);

    // Test results
    println!("\n=== Test Results ===");
    let results = vec![
        TestResult {
            name: "nginx is installed".into(),
            test_type: TestSubcommand::Behavior,
            passed: true,
            duration_secs: 1.2,
            message: None,
            artifacts: vec![],
        },
        TestResult {
            name: "convergence: nginx stack".into(),
            test_type: TestSubcommand::Convergence,
            passed: true,
            duration_secs: 8.5,
            message: None,
            artifacts: vec![TestArtifact {
                name: "sandbox-diff.tar".into(),
                path: "artifacts/sandbox-diff.tar".into(),
                content_type: Some("application/x-tar".into()),
                size_bytes: Some(1024),
            }],
        },
        TestResult {
            name: "mutation: file content".into(),
            test_type: TestSubcommand::Mutation,
            passed: false,
            duration_secs: 3.0,
            message: Some("mutation survived: content not re-applied".into()),
            artifacts: vec![],
        },
    ];
    for r in &results {
        println!("  {r}");
    }

    // Suite report
    println!("\n=== Suite Report ===");
    let report = TestSuiteReport {
        total: 3,
        passed: 2,
        failed: 1,
        skipped: 0,
        duration_secs: 12.7,
        results,
    };
    println!("  {}", report.format_summary());

    // Coverage threshold
    println!("\n=== Coverage Threshold ===");
    let threshold = CoverageThreshold {
        min_line_pct: 95.0,
        min_branch_pct: Some(80.0),
        enforce: true,
    };
    println!(
        "  95.91% line, 85% branch: {}",
        threshold.check(95.91, Some(85.0))
    );
    println!(
        "  94.00% line, 85% branch: {}",
        threshold.check(94.0, Some(85.0))
    );

    // Coverage badge
    println!("\n=== Coverage Badge ===");
    for pct in [98.0, 92.0, 85.0, 72.0, 63.0, 45.0] {
        let badge = CoverageBadge::from_pct(pct);
        println!(
            "  {pct:.0}% -> {} ({})",
            badge.color,
            BadgeColor::from_pct(pct)
        );
    }
}
