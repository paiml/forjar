#![allow(clippy::field_reassign_with_default)]
//! FJ-2200/2602/2604/2605/3000-3040: Testing strategy and defect analysis falsification.
//!
//! Demonstrates Popperian rejection criteria for:
//! - Design by contract (hash invariant checks, handler audit reports)
//! - Behavior-driven infrastructure specs
//! - Mutation testing score model
//! - Resource coverage levels (L0-L5)
//! - Defect analysis lints (semicolon chains, nohup hazards)
//!
//! Usage: cargo run --example testing_defects_falsification

use forjar::cli::lint::{has_bare_semicolon, lint_nohup_ld_path, lint_semicolon_chains};
use forjar::core::types::{
    BehaviorReport, BehaviorResult, ContractKind, CoverageLevel, CoverageReport, ForjarConfig,
    HandlerAuditReport, HashInvariantCheck, MutationScore, Resource, ResourceCoverage,
    ResourceType,
};

fn mark_pair(ok: bool) -> (&'static str, &'static str) {
    if ok {
        ("✓", "")
    } else {
        ("✗", "FALSIFIED")
    }
}

fn report_yes_no(label: &str, ok: bool) {
    let status = if ok { "yes" } else { "no" };
    let mark = if ok { "✓" } else { "✗ FALSIFIED" };
    println!("  {label}: {status} {mark}");
}

fn check_design_by_contract() {
    println!("\n[FJ-2200] Design by Contract:");

    let report = HandlerAuditReport {
        checks: vec![
            HashInvariantCheck::pass("nginx-pkg", "package", "blake3:abc123"),
            HashInvariantCheck::pass("config", "file", "blake3:def456"),
            HashInvariantCheck::fail("cron-job", "cron", "blake3:a", "blake3:b", "schedule hash"),
        ],
        exemptions: vec![],
    };
    println!(
        "  Audit: {}/{} passed, {} failed",
        report.pass_count(),
        report.checks.len(),
        report.fail_count()
    );
    assert_eq!(report.pass_count(), 2);
    assert_eq!(report.fail_count(), 1);
    println!("  Hash invariant tracking: ✓");

    for (kind, label) in [
        (ContractKind::Requires, "requires"),
        (ContractKind::Ensures, "ensures"),
        (ContractKind::Invariant, "invariant"),
    ] {
        assert_eq!(kind.to_string(), label);
    }
    println!("  Contract kinds (requires/ensures/invariant): ✓");
}

fn check_behavior_specs() {
    println!("\n[FJ-2602] Behavior-Driven Infrastructure Specs:");

    let results = vec![
        BehaviorResult {
            name: "nginx installed".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 50,
        },
        BehaviorResult {
            name: "nginx running".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 30,
        },
    ];
    let full_pass = BehaviorReport::from_results("nginx".into(), results);
    let pass_ok = full_pass.all_passed() && full_pass.total == 2;
    let (mark, tail) = mark_pair(pass_ok);
    println!(
        "  All pass: {}/{}  {} {}",
        full_pass.passed, full_pass.total, mark, tail
    );
    assert!(pass_ok);

    let results_fail = vec![
        BehaviorResult {
            name: "installed".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 50,
        },
        BehaviorResult {
            name: "running".into(),
            passed: false,
            failure: Some("exit code 1".into()),
            actual_exit_code: Some(1),
            actual_stdout: None,
            duration_ms: 100,
        },
    ];
    let partial_fail = BehaviorReport::from_results("nginx".into(), results_fail);
    let fail_ok = !partial_fail.all_passed() && partial_fail.failed == 1;
    let (mark, tail) = mark_pair(fail_ok);
    println!(
        "  Partial fail: {}/{} passed  {} {}",
        partial_fail.passed, partial_fail.total, mark, tail
    );
    assert!(fail_ok);
}

fn check_mutation_scores() {
    println!("\n[FJ-2604] Mutation Testing Scores:");

    let cases = [
        (100, 100, 'A', "100%"),
        (90, 100, 'A', "90%"),
        (80, 100, 'B', "80%"),
        (60, 100, 'C', "60%"),
        (50, 100, 'F', "50%"),
    ];
    for (detected, total, expected_grade, label) in &cases {
        let score = MutationScore {
            total: *total,
            detected: *detected,
            survived: total - detected,
            errored: 0,
        };
        let ok = score.grade() == *expected_grade;
        let (mark, tail) = mark_pair(ok);
        println!("  {label}: grade={} {} {}", score.grade(), mark, tail);
        assert!(ok);
    }
}

fn check_coverage_levels() {
    println!("\n[FJ-2605] Resource Coverage Model:");

    let entries = vec![
        ResourceCoverage {
            resource_id: "nginx-pkg".into(),
            level: CoverageLevel::L4,
            resource_type: "package".into(),
        },
        ResourceCoverage {
            resource_id: "nginx-conf".into(),
            level: CoverageLevel::L3,
            resource_type: "file".into(),
        },
        ResourceCoverage {
            resource_id: "deploy-task".into(),
            level: CoverageLevel::L1,
            resource_type: "task".into(),
        },
    ];
    let cov_report = CoverageReport::from_entries(entries);
    let cov_ok = cov_report.min_level == CoverageLevel::L1
        && cov_report.meets_threshold(CoverageLevel::L1)
        && !cov_report.meets_threshold(CoverageLevel::L2);
    let (mark, tail) = mark_pair(cov_ok);
    println!(
        "  Min={}, Avg={:.1}, Threshold L1={}, L2={}  {} {}",
        cov_report.min_level,
        cov_report.avg_level,
        cov_report.meets_threshold(CoverageLevel::L1),
        cov_report.meets_threshold(CoverageLevel::L2),
        mark,
        tail
    );
    assert!(cov_ok);

    let order_ok = CoverageLevel::L0 < CoverageLevel::L1
        && CoverageLevel::L1 < CoverageLevel::L2
        && CoverageLevel::L4 < CoverageLevel::L5;
    let (mark, tail) = mark_pair(order_ok);
    println!("  Level ordering L0 < L1 < ... < L5: {} {}", mark, tail);
    assert!(order_ok);
}

fn check_semicolon_chains() {
    println!("\n[FJ-3000] Exit Code Safety (Semicolon Chains):");

    let bare_ok = has_bare_semicolon("cmd1 ; cmd2");
    let quoted_ok = !has_bare_semicolon("echo 'a;b'");
    let double_ok = !has_bare_semicolon("echo \"a;b\"");
    report_yes_no("Bare ';' detected", bare_ok);
    report_yes_no("Single-quoted ';' ignored", quoted_ok);
    report_yes_no("Double-quoted ';' ignored", double_ok);
    assert!(bare_ok && quoted_ok && double_ok);

    let mut config = ForjarConfig::default();
    let mut task = Resource::default();
    task.resource_type = ResourceType::Task;
    task.command = Some("cd /app ; make install".into());
    config.resources.insert("build".into(), task);
    let chain_warnings = lint_semicolon_chains(&config);
    let chain_ok = !chain_warnings.is_empty();
    report_yes_no("Task with ';' flagged", chain_ok);
    assert!(chain_ok);
}

fn check_nohup_hazard() {
    println!("\n[FJ-3030] Nohup LD_LIBRARY_PATH Hazard:");

    let mut config2 = ForjarConfig::default();
    let mut task2 = Resource::default();
    task2.resource_type = ResourceType::Task;
    task2.command = Some("nohup /opt/cuda/bin/train &".into());
    config2.resources.insert("train".into(), task2);
    let ld_warnings = lint_nohup_ld_path(&config2);
    let ld_ok = !ld_warnings.is_empty();
    report_yes_no("nohup /abs/path without LD flagged", ld_ok);
    assert!(ld_ok);
}

fn main() {
    println!("Forjar Testing Strategy & Defect Analysis Falsification");
    println!("{}", "=".repeat(58));

    check_design_by_contract();
    check_behavior_specs();
    check_mutation_scores();
    check_coverage_levels();
    check_semicolon_chains();
    check_nohup_hazard();

    println!("\n{}", "=".repeat(58));
    println!("All testing strategy & defect analysis criteria survived.");
}
