//! FJ-2602/2001/2203: Test runner, query engine, handler contract types.
//! Usage: cargo test --test falsification_test_query_handler

use forjar::core::types::*;

// ── helpers ──

fn tr(name: &str, sub: TestSubcommand, passed: bool) -> TestResult {
    TestResult {
        name: name.into(),
        test_type: sub,
        passed,
        duration_secs: 1.5,
        message: if passed { None } else { Some("failed".into()) },
        artifacts: vec![],
    }
}

fn tsr(total: u32, passed: u32, failed: u32, skipped: u32) -> TestSuiteReport {
    TestSuiteReport {
        total,
        passed,
        failed,
        skipped,
        duration_secs: 30.0,
        results: vec![],
    }
}

fn mhr(name: &str, total: u32, converged: u32, drifted: u32, failed: u32) -> MachineHealthRow {
    MachineHealthRow {
        name: name.into(),
        total,
        converged,
        drifted,
        failed,
        last_apply: "2026-03-09T10:00:00Z".into(),
        generation: 5,
    }
}

// ── FJ-2602: TestSubcommand ──

#[test]
fn test_subcommand_display() {
    assert_eq!(TestSubcommand::Behavior.to_string(), "behavior");
    assert_eq!(TestSubcommand::Convergence.to_string(), "convergence");
    assert_eq!(TestSubcommand::Mutation.to_string(), "mutation");
    assert_eq!(TestSubcommand::All.to_string(), "all");
}

#[test]
fn test_subcommand_serde() {
    for sub in [
        TestSubcommand::Behavior,
        TestSubcommand::Convergence,
        TestSubcommand::All,
    ] {
        let json = serde_json::to_string(&sub).unwrap();
        let parsed: TestSubcommand = serde_json::from_str(&json).unwrap();
        assert_eq!(sub, parsed);
    }
}

// ── FJ-2603: SandboxConfig ──

#[test]
fn sandbox_config_default() {
    let c = SandboxConfig::default();
    assert_eq!(c.backend, SandboxBackend::Pepita);
    assert!(c.cleanup);
    assert_eq!(c.timeout_secs, 300);
    assert!(!c.capture_overlay);
}

#[test]
fn sandbox_backend_display() {
    assert_eq!(SandboxBackend::Pepita.to_string(), "pepita");
    assert_eq!(SandboxBackend::Container.to_string(), "container");
    assert_eq!(SandboxBackend::Chroot.to_string(), "chroot");
}

// ── FJ-2602: TestResult ──

#[test]
fn test_result_display_pass() {
    let s = tr("nginx check", TestSubcommand::Behavior, true).to_string();
    assert!(s.contains("[PASS]"));
    assert!(s.contains("nginx check"));
}

#[test]
fn test_result_display_fail() {
    let s = tr("convergence", TestSubcommand::Convergence, false).to_string();
    assert!(s.contains("[FAIL]"));
    assert!(s.contains("failed"));
}

// ── FJ-2602: TestSuiteReport ──

#[test]
fn test_suite_pass_rate() {
    let r = tsr(10, 8, 1, 1);
    assert!((r.pass_rate() - 80.0).abs() < 0.01);
    assert!(!r.all_passed());
}

#[test]
fn test_suite_all_passed() {
    let r = tsr(5, 5, 0, 0);
    assert!(r.all_passed());
    assert!((r.pass_rate() - 100.0).abs() < 0.01);
}

#[test]
fn test_suite_empty() {
    assert!((tsr(0, 0, 0, 0).pass_rate() - 100.0).abs() < 0.01);
}

#[test]
fn test_suite_format_summary() {
    let s = tsr(10, 9, 1, 0).format_summary();
    assert!(s.contains("9 passed"));
    assert!(s.contains("1 failed"));
    assert!(s.contains("90%"));
}

// ── FJ-2604: CoverageThreshold ──

#[test]
fn coverage_threshold_check() {
    let t = CoverageThreshold {
        min_line_pct: 95.0,
        min_branch_pct: Some(80.0),
        enforce: true,
    };
    assert!(t.check(96.0, Some(85.0)));
    assert!(!t.check(94.0, Some(85.0)));
    assert!(!t.check(96.0, Some(75.0)));
    assert!(t.check(96.0, None)); // branch not reported
}

// ── FJ-2604: CoverageBadge & BadgeColor ──

#[test]
fn coverage_badge_from_pct() {
    let b = CoverageBadge::from_pct(97.0);
    assert_eq!(b.color, BadgeColor::BrightGreen);
    assert!((b.line_pct - 97.0).abs() < 0.01);
    assert_eq!(CoverageBadge::from_pct(55.0).color, BadgeColor::Red);
}

#[test]
fn badge_color_ranges() {
    assert_eq!(BadgeColor::from_pct(95.0), BadgeColor::BrightGreen);
    assert_eq!(BadgeColor::from_pct(92.0), BadgeColor::Green);
    assert_eq!(BadgeColor::from_pct(85.0), BadgeColor::YellowGreen);
    assert_eq!(BadgeColor::from_pct(75.0), BadgeColor::Yellow);
    assert_eq!(BadgeColor::from_pct(65.0), BadgeColor::Orange);
    assert_eq!(BadgeColor::from_pct(50.0), BadgeColor::Red);
}

#[test]
fn badge_color_display() {
    assert_eq!(BadgeColor::BrightGreen.to_string(), "brightgreen");
    assert_eq!(BadgeColor::Red.to_string(), "red");
    assert_eq!(BadgeColor::Yellow.to_string(), "yellow");
}

// ── FJ-2001: HealthSummary ──

fn sample_health() -> HealthSummary {
    HealthSummary {
        machines: vec![mhr("intel", 17, 17, 0, 0), mhr("jetson", 8, 7, 1, 0)],
    }
}

#[test]
fn health_summary_totals() {
    let h = sample_health();
    assert_eq!(h.total_resources(), 25);
    assert_eq!(h.total_converged(), 24);
    assert_eq!(h.total_drifted(), 1);
    assert_eq!(h.total_failed(), 0);
}

#[test]
fn health_summary_pct() {
    assert!((sample_health().stack_health_pct() - 96.0).abs() < 0.1);
}

#[test]
fn health_summary_empty() {
    let h = HealthSummary { machines: vec![] };
    assert_eq!(h.stack_health_pct(), 100.0);
    assert_eq!(h.total_resources(), 0);
}

#[test]
fn health_summary_format_table() {
    let table = sample_health().format_table();
    assert!(table.contains("intel"));
    assert!(table.contains("jetson"));
    assert!(table.contains("TOTAL"));
    assert!(table.contains("96%"));
}

// ── FJ-2004: TimingStats ──

#[test]
fn timing_stats_from_sorted() {
    let d = vec![0.1, 0.2, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0, 3.0, 5.0];
    let s = TimingStats::from_sorted(&d).unwrap();
    assert_eq!(s.count, 10);
    assert!(s.avg > 0.0);
    assert_eq!(s.max, 5.0);
}

#[test]
fn timing_stats_empty() {
    assert!(TimingStats::from_sorted(&[]).is_none());
}

#[test]
fn timing_stats_single() {
    let s = TimingStats::from_sorted(&[1.5]).unwrap();
    assert_eq!(s.count, 1);
    assert!((s.avg - 1.5).abs() < 0.001);
}

#[test]
fn timing_stats_format_compact() {
    let s = TimingStats {
        count: 10,
        avg: 1.23,
        p50: 0.95,
        p95: 3.40,
        p99: 4.80,
        max: 5.0,
    };
    let text = s.format_compact();
    assert!(text.contains("avg=1.23s"));
    assert!(text.contains("p50=0.95s"));
    assert!(text.contains("p95=3.40s"));
}

// ── FJ-2004: ChurnMetric ──

#[test]
fn churn_metric_pct() {
    let c = ChurnMetric {
        resource_id: "bash".into(),
        changed_gens: 3,
        total_gens: 12,
    };
    assert!((c.churn_pct() - 25.0).abs() < 0.1);
}

#[test]
fn churn_metric_zero() {
    let c = ChurnMetric {
        resource_id: "x".into(),
        changed_gens: 0,
        total_gens: 0,
    };
    assert_eq!(c.churn_pct(), 0.0);
}

// ── FJ-2001: QueryParams & QueryOutputFormat ──

#[test]
fn query_params_default() {
    let p = QueryParams::default();
    assert_eq!(p.limit, 50);
    assert_eq!(p.format, QueryOutputFormat::Table);
    assert!(!p.history);
    assert!(!p.drift);
}

#[test]
fn query_output_format_default() {
    assert_eq!(QueryOutputFormat::default(), QueryOutputFormat::Table);
}

// ── FJ-2203: HashInvariantCheck ──

#[test]
fn hash_invariant_pass() {
    let c = HashInvariantCheck::pass("pkg", "package", "blake3:abc");
    assert!(c.passed);
    assert_eq!(c.expected_hash, c.actual_hash);
    assert!(c.deviation_reason.is_none());
    assert!(c.to_string().contains("[PASS]"));
}

#[test]
fn hash_invariant_fail() {
    let c = HashInvariantCheck::fail("cron", "cron", "blake3:a", "blake3:b", "schedule only");
    assert!(!c.passed);
    assert_ne!(c.expected_hash, c.actual_hash);
    assert_eq!(c.deviation_reason.as_deref(), Some("schedule only"));
    let s = c.to_string();
    assert!(s.contains("[FAIL]"));
    assert!(s.contains("schedule only"));
}

// ── FJ-2203: HandlerAuditReport ──

#[test]
fn handler_audit_all_pass() {
    let report = HandlerAuditReport {
        checks: vec![
            HashInvariantCheck::pass("a", "file", "h1"),
            HashInvariantCheck::pass("b", "package", "h2"),
        ],
        exemptions: vec![],
    };
    assert!(report.all_passed());
    assert_eq!(report.pass_count(), 2);
    assert_eq!(report.fail_count(), 0);
}

#[test]
fn handler_audit_with_failure() {
    let report = HandlerAuditReport {
        checks: vec![
            HashInvariantCheck::pass("a", "file", "h1"),
            HashInvariantCheck::fail("b", "cron", "h2", "h3", "reason"),
        ],
        exemptions: vec![],
    };
    assert!(!report.all_passed());
    assert_eq!(report.fail_count(), 1);
}

#[test]
fn handler_audit_format_report() {
    let report = HandlerAuditReport {
        checks: vec![HashInvariantCheck::pass("pkg", "package", "h")],
        exemptions: vec![HandlerExemption {
            handler: "task".into(),
            reason: "imperative".into(),
            approved_by: Some("spec".into()),
        }],
    };
    let s = report.format_report();
    assert!(s.contains("Handler Hash Invariant Audit"));
    assert!(s.contains("1 passed"));
    assert!(s.contains("[EXEMPT] task"));
}

// ── FJ-2200: ContractKind & ProofStatus ──

#[test]
fn contract_kind_display() {
    assert_eq!(ContractKind::Requires.to_string(), "requires");
    assert_eq!(ContractKind::Ensures.to_string(), "ensures");
    assert_eq!(ContractKind::Invariant.to_string(), "invariant");
}

#[test]
fn proof_status_display() {
    assert_eq!(ProofStatus::Verified.to_string(), "verified");
    assert_eq!(ProofStatus::Failed.to_string(), "failed");
    assert_eq!(ProofStatus::Deprecated.to_string(), "deprecated");
}

#[test]
fn contract_assertion_serde() {
    let a = ContractAssertion {
        function: "determine_present_action".into(),
        module: "core::planner".into(),
        kind: ContractKind::Ensures,
        held: true,
        expression: Some("result.is_noop()".into()),
    };
    let json = serde_json::to_string(&a).unwrap();
    let parsed: ContractAssertion = serde_json::from_str(&json).unwrap();
    assert!(parsed.held);
    assert_eq!(parsed.kind, ContractKind::Ensures);
}

#[test]
fn kani_harness_serde() {
    let h = KaniHarness {
        name: "proof_blake3".into(),
        property: "deterministic".into(),
        target_function: "blake3::hash".into(),
        status: ProofStatus::Verified,
        bound: Some(16),
    };
    let json = serde_json::to_string(&h).unwrap();
    let parsed: KaniHarness = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.status, ProofStatus::Verified);
    assert_eq!(parsed.bound, Some(16));
}
