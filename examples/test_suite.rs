//! Demonstrates FJ-2606: Unified test suite — convergence + mutation + behavior.
//!
//! Shows how all three test modes work together, producing a combined report
//! suitable for CI gates.

use forjar::core::store::convergence_runner::{self, ConvergenceSummary, ConvergenceTarget};
use forjar::core::store::mutation_runner::{self, MutationRunConfig, MutationTarget};
use forjar::core::types::{BehaviorReport, BehaviorResult};

fn main() {
    println!("=== FJ-2606: Unified Test Suite ===\n");
    let t0 = std::time::Instant::now();

    // --- Phase 1: Convergence ---
    println!("--- Convergence Tests ---");
    let conv_targets = vec![
        conv_target(
            "nginx-config",
            "file",
            "install nginx-config",
            "check nginx-config",
        ),
        conv_target(
            "curl-pkg",
            "package",
            "apt-get install curl",
            "dpkg -l curl",
        ),
        conv_target(
            "app-svc",
            "service",
            "systemctl enable app",
            "systemctl is-active app",
        ),
    ];
    let conv_results = convergence_runner::run_convergence_parallel(conv_targets, 4);
    let conv_summary = ConvergenceSummary::from_results(&conv_results);
    print!(
        "{}",
        convergence_runner::format_convergence_report(&conv_results)
    );

    // --- Phase 2: Mutation ---
    println!("\n--- Mutation Tests ---");
    let mut_targets = vec![
        mut_target("nginx-config", "file", "install", "check", "hash1"),
        mut_target("curl-pkg", "package", "apt install", "dpkg -l", "hash2"),
        mut_target(
            "app-svc",
            "service",
            "systemctl enable",
            "systemctl is-active",
            "hash3",
        ),
    ];
    let mut_config = MutationRunConfig {
        mutations_per_resource: 3,
        ..Default::default()
    };
    let mut_report = mutation_runner::run_mutation_parallel(mut_targets, &mut_config);
    print!("{}", mutation_runner::format_mutation_run(&mut_report));

    // --- Phase 3: Behavior ---
    println!("\n--- Behavior Tests ---");
    let behavior_results = vec![
        BehaviorResult {
            name: "nginx is installed".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 45,
        },
        BehaviorResult {
            name: "port 80 is open".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 12,
        },
        BehaviorResult {
            name: "config is valid".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: Some("ok".into()),
            duration_ms: 8,
        },
    ];
    let behavior_report = BehaviorReport::from_results("nginx web server".into(), behavior_results);
    print!("{}", behavior_report.format_summary());

    // --- Combined Report ---
    let elapsed = t0.elapsed();
    println!("\n=== Combined Test Report ===");
    println!(
        "  Convergence: {}/{} passed ({:.0}%)",
        conv_summary.passed,
        conv_summary.total,
        conv_summary.pass_rate()
    );
    println!(
        "  Mutation:    {}/{} detected (grade {})",
        mut_report.score.detected,
        mut_report.score.total,
        mut_report.score.grade()
    );
    println!(
        "  Behavior:    {}/{} passed",
        behavior_report.passed, behavior_report.total
    );

    let all_pass = conv_summary.passed == conv_summary.total
        && mut_report.score.grade() != 'F'
        && behavior_report.all_passed();
    println!("\n  Overall: {}", if all_pass { "PASS" } else { "FAIL" });
    println!("  Duration: {:.1}s", elapsed.as_secs_f64());
}

fn conv_target(id: &str, rtype: &str, apply: &str, check: &str) -> ConvergenceTarget {
    // expected_hash must match what simulate_state_query returns (hash of check script)
    let refs = [check];
    ConvergenceTarget {
        resource_id: id.into(),
        resource_type: rtype.into(),
        apply_script: apply.into(),
        state_query_script: check.into(),
        expected_hash: forjar::tripwire::hasher::composite_hash(&refs),
    }
}

fn mut_target(id: &str, rtype: &str, apply: &str, drift: &str, hash: &str) -> MutationTarget {
    MutationTarget {
        resource_id: id.into(),
        resource_type: rtype.into(),
        apply_script: apply.into(),
        drift_script: drift.into(),
        expected_hash: hash.into(),
    }
}
