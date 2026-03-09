//! FJ-202/3507/036: Conditions, rollout, purifier.
//!
//! Usage: cargo run --example conditions_rollout_purifier

use forjar::core::conditions::evaluate_when;
use forjar::core::purifier::{lint_error_count, purify_script, validate_script};
use forjar::core::rollout::{execute_rollout, plan_rollout, run_health_check};
use forjar::core::types::environment::RolloutConfig;
use forjar::core::types::Machine;
use std::collections::HashMap;

fn main() {
    println!("Forjar: Conditions, Rollout & Purifier");
    println!("{}", "=".repeat(55));

    // ── FJ-202: Conditions ──
    println!("\n[FJ-202] When-Condition Evaluation:");
    let m = Machine {
        hostname: "intel-01".into(),
        addr: "10.0.0.1".into(),
        user: "root".into(),
        arch: "x86_64".into(),
        ssh_key: None,
        roles: vec!["web".into(), "gpu".into()],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };
    let mut params = HashMap::new();
    params.insert("env".into(), serde_yaml_ng::Value::String("prod".into()));

    for expr in [
        "true",
        "{{machine.arch}} == \"x86_64\"",
        "{{params.env}} != \"staging\"",
        "{{machine.roles}} contains \"gpu\"",
    ] {
        let result = evaluate_when(expr, &params, &m).unwrap();
        println!("  {expr:>45} → {result}");
    }

    // ── FJ-3507: Rollout ──
    println!("\n[FJ-3507] Rollout Planning:");
    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![25, 50, 100],
    };
    let steps = plan_rollout(&config, 10);
    for s in &steps {
        println!(
            "  Step {}: {}% → {} machines",
            s.index,
            s.percentage,
            s.machine_indices.len()
        );
    }

    println!("\n[FJ-3507] Dry-Run Execution:");
    let result = execute_rollout(&config, 10, true);
    println!(
        "  Strategy: {} | Completed: {} | Deployed: {}",
        result.strategy,
        result.completed,
        result.deployed_count()
    );

    println!("\n[FJ-3507] Health Check:");
    let (passed, msg) = run_health_check("true", Some("5s"));
    println!("  {msg} (passed={passed})");

    // ── FJ-036: Purifier ──
    println!("\n[FJ-036] Shell Purification:");
    let script = "echo hello world\n";
    println!("  validate: {:?}", validate_script(script));
    println!("  lint errors: {}", lint_error_count(script));
    let purified = purify_script(script).unwrap();
    println!("  purified: {:?}", purified.trim());

    println!("\n{}", "=".repeat(55));
    println!("All conditions/rollout/purifier criteria survived.");
}
