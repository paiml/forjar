//! FJ-3505: Promotion gate evaluation example.
//!
//! Demonstrates evaluating quality gates before promoting
//! infrastructure from dev to staging to production.

use forjar::core::promotion::{evaluate_gates, GateResult};
use forjar::core::types::environment::*;

fn main() {
    println!("=== FJ-3505: Promotion Gate Evaluation ===\n");

    // Write a temp forjar.yaml for gate evaluation
    let dir = tempfile::tempdir().expect("create temp dir");
    let cfg_path = dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg_path,
        r#"
version: "1.0"
name: promotion-demo
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
resources:
  app-pkg:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
"#,
    )
    .expect("write config");

    // 1. Simple gates: validate + script
    println!("--- Promotion: dev → staging ---");
    let dev_to_staging = PromotionConfig {
        from: "dev".into(),
        gates: vec![
            PromotionGate {
                validate: Some(ValidateGateOptions {
                    deep: false,
                    exhaustive: false,
                }),
                ..Default::default()
            },
            PromotionGate {
                script: Some("echo 'smoke tests passed'".into()),
                ..Default::default()
            },
        ],
        auto_approve: true,
        rollout: None,
    };

    let result = evaluate_gates(&cfg_path, "staging", &dev_to_staging);
    print_result(&result.gates, result.all_passed, result.auto_approve);

    // 2. Full gates: validate + policy + coverage + script
    println!("\n--- Promotion: staging → production ---");
    let staging_to_prod = PromotionConfig {
        from: "staging".into(),
        gates: vec![
            PromotionGate {
                validate: Some(ValidateGateOptions {
                    deep: true,
                    exhaustive: false,
                }),
                ..Default::default()
            },
            PromotionGate {
                policy: Some(PolicyGateOptions { strict: true }),
                ..Default::default()
            },
            PromotionGate {
                coverage: Some(CoverageGateOptions { min: 90 }),
                ..Default::default()
            },
            PromotionGate {
                script: Some("echo 'integration tests passed'".into()),
                ..Default::default()
            },
        ],
        auto_approve: false,
        rollout: Some(RolloutConfig {
            strategy: "canary".into(),
            canary_count: 1,
            health_check: Some("curl -sf http://localhost/health".into()),
            health_timeout: Some("30s".into()),
            percentage_steps: vec![10, 25, 50, 100],
        }),
    };

    let result = evaluate_gates(&cfg_path, "production", &staging_to_prod);
    print_result(&result.gates, result.all_passed, result.auto_approve);

    // 3. Failing gate
    println!("\n--- Promotion with failing gate ---");
    let failing = PromotionConfig {
        from: "dev".into(),
        gates: vec![PromotionGate {
            script: Some("exit 1".into()),
            ..Default::default()
        }],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg_path, "staging", &failing);
    print_result(&result.gates, result.all_passed, result.auto_approve);

    println!("\n--- Summary ---");
    println!("Gate types: validate, policy, coverage, script");
    println!("Auto-approve: configurable per promotion");
    println!("Rollout: canary, percentage, all-at-once");
}

fn print_result(gates: &[GateResult], all_passed: bool, auto_approve: bool) {
    for gate in gates {
        let icon = if gate.passed { "PASS" } else { "FAIL" };
        println!("  [{icon}] {}: {}", gate.gate_type, gate.message);
    }
    println!(
        "  Result: {} (auto-approve: {})",
        if all_passed { "APPROVED" } else { "BLOCKED" },
        auto_approve
    );
}
