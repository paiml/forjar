//! FJ-2602/2001/2203: Test runner, query engine, handler contracts.
//!
//! Usage: cargo run --example test_query_handler

use forjar::core::types::*;

fn main() {
    println!("Forjar: Test Runner, Query Engine & Handler Contracts");
    println!("{}", "=".repeat(55));

    // ── Test Runner ──
    println!("\n[FJ-2602] Test Subcommands:");
    for sub in [
        TestSubcommand::Behavior,
        TestSubcommand::Convergence,
        TestSubcommand::Mutation,
        TestSubcommand::All,
    ] {
        println!("  {sub}");
    }

    println!("\n[FJ-2603] Sandbox Config:");
    let sandbox = SandboxConfig::default();
    println!(
        "  Backend: {} | Cleanup: {} | Timeout: {}s",
        sandbox.backend, sandbox.cleanup, sandbox.timeout_secs
    );

    println!("\n[FJ-2602] Test Suite Report:");
    let report = TestSuiteReport {
        total: 10,
        passed: 9,
        failed: 1,
        skipped: 0,
        duration_secs: 45.0,
        results: vec![],
    };
    println!("  {}", report.format_summary());
    println!("  All passed: {}", report.all_passed());

    println!("\n[FJ-2604] Coverage Badge:");
    let badge = CoverageBadge::from_pct(97.0);
    println!(
        "  {:.1}% → {} ({})",
        badge.line_pct, badge.color, badge.label
    );

    // ── Query Engine ──
    println!("\n[FJ-2001] Health Summary:");
    let health = HealthSummary {
        machines: vec![
            MachineHealthRow {
                name: "intel".into(),
                total: 17,
                converged: 17,
                drifted: 0,
                failed: 0,
                last_apply: "2026-03-09T10:00:00Z".into(),
                generation: 12,
            },
            MachineHealthRow {
                name: "jetson".into(),
                total: 8,
                converged: 7,
                drifted: 1,
                failed: 0,
                last_apply: "2026-03-09T09:00:00Z".into(),
                generation: 5,
            },
        ],
    };
    println!("{}", health.format_table());

    println!("[FJ-2004] Timing Stats:");
    let durations = vec![0.1, 0.2, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0, 3.0, 5.0];
    if let Some(stats) = TimingStats::from_sorted(&durations) {
        println!("  {}", stats.format_compact());
    }

    // ── Handler Contracts ──
    println!("\n[FJ-2203] Handler Audit Report:");
    let audit = HandlerAuditReport {
        checks: vec![
            HashInvariantCheck::pass("nginx-pkg", "package", "blake3:abc"),
            HashInvariantCheck::pass("nginx-conf", "file", "blake3:def"),
            HashInvariantCheck::fail("cron-job", "cron", "blake3:a", "blake3:b", "schedule only"),
        ],
        exemptions: vec![HandlerExemption {
            handler: "task".into(),
            reason: "imperative by nature".into(),
            approved_by: Some("spec review".into()),
        }],
    };
    println!("{}", audit.format_report());

    println!("[FJ-2200] Contract Kinds:");
    for kind in [
        ContractKind::Requires,
        ContractKind::Ensures,
        ContractKind::Invariant,
    ] {
        println!("  {kind}");
    }

    println!("\n{}", "=".repeat(55));
    println!("All test/query/handler criteria survived.");
}
