# Test Runner, Query Engine & Handler Contracts

Falsification coverage for FJ-2602, FJ-2603, FJ-2604, FJ-2001, FJ-2004, FJ-2200, and FJ-2203.

## Test Runner (FJ-2602/2603/2604)

Test subcommands, sandbox isolation, suite reporting, and coverage badges:

```rust
use forjar::core::types::*;

// Test subcommands
for sub in [TestSubcommand::Behavior, TestSubcommand::Convergence,
            TestSubcommand::Mutation, TestSubcommand::All] {
    println!("{sub}"); // "behavior", "convergence", "mutation", "all"
}

// Sandbox config defaults
let sandbox = SandboxConfig::default();
assert_eq!(sandbox.backend, SandboxBackend::Pepita);
assert!(sandbox.cleanup);
assert_eq!(sandbox.timeout_secs, 300);

// Suite report
let report = TestSuiteReport { total: 10, passed: 9, failed: 1, skipped: 0,
                                duration_secs: 45.0, results: vec![] };
assert!((report.pass_rate() - 90.0).abs() < 0.01);
assert!(!report.all_passed());
println!("{}", report.format_summary()); // "9 passed, 1 failed, 0 skipped (90%)"

// Coverage badge
let badge = CoverageBadge::from_pct(97.0);
assert_eq!(badge.color, BadgeColor::BrightGreen);
```

Badge color thresholds: BrightGreen (>=95), Green (>=90), YellowGreen (>=80), Yellow (>=70), Orange (>=60), Red (<60).

## Query Engine (FJ-2001/2004)

Health summaries, timing statistics, and churn metrics:

```rust
use forjar::core::types::*;

// Health summary with per-machine rows
let health = HealthSummary {
    machines: vec![
        MachineHealthRow { name: "intel".into(), total: 17, converged: 17,
                           drifted: 0, failed: 0, last_apply: "...".into(), generation: 12 },
        MachineHealthRow { name: "jetson".into(), total: 8, converged: 7,
                           drifted: 1, failed: 0, last_apply: "...".into(), generation: 5 },
    ],
};
assert_eq!(health.total_resources(), 25);
assert!((health.stack_health_pct() - 96.0).abs() < 0.1);
println!("{}", health.format_table()); // tabular output with TOTAL row

// Timing stats from sorted durations
let durations = vec![0.1, 0.2, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0, 3.0, 5.0];
let stats = TimingStats::from_sorted(&durations).unwrap();
println!("{}", stats.format_compact()); // "avg=X p50=Y p95=Z p99=W max=M (10 samples)"

// Churn metric
let churn = ChurnMetric { resource_id: "bash".into(), changed_gens: 3, total_gens: 12 };
assert!((churn.churn_pct() - 25.0).abs() < 0.1);
```

## Handler Contracts (FJ-2200/2203)

Hash invariant checks, audit reports, contract assertions, and Kani harnesses:

```rust
use forjar::core::types::*;

// Hash invariant checks
let pass = HashInvariantCheck::pass("nginx-pkg", "package", "blake3:abc");
assert!(pass.passed);
let fail = HashInvariantCheck::fail("cron-job", "cron", "blake3:a", "blake3:b", "schedule only");
assert!(!fail.passed);

// Audit report
let report = HandlerAuditReport {
    checks: vec![pass, fail],
    exemptions: vec![HandlerExemption {
        handler: "task".into(), reason: "imperative".into(),
        approved_by: Some("spec review".into()),
    }],
};
assert_eq!(report.pass_count(), 1);
assert_eq!(report.fail_count(), 1);
println!("{}", report.format_report()); // includes [PASS], [FAIL], [EXEMPT] sections

// Contract kinds and proof status
assert_eq!(ContractKind::Ensures.to_string(), "ensures");
assert_eq!(ProofStatus::Verified.to_string(), "verified");
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_test_query_handler.rs` | 35 | ~396 |
