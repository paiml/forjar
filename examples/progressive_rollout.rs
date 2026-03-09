//! FJ-3507: Progressive rollout example.
//!
//! Demonstrates canary, percentage, and all-at-once rollout strategies
//! with health check integration.

use forjar::core::rollout::*;
use forjar::core::types::environment::RolloutConfig;

fn main() {
    println!("=== FJ-3507: Progressive Rollout ===\n");

    // 1. Canary rollout (1 machine first, then 10%, 25%, 50%, 100%)
    println!("--- Canary Rollout (10 machines) ---");
    let canary_config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: Some("true".into()), // always passes
        health_timeout: Some("30s".into()),
        percentage_steps: vec![10, 25, 50, 100],
    };

    let steps = plan_rollout(&canary_config, 10);
    for step in &steps {
        println!(
            "  Step {}: {}% ({} machines: {:?})",
            step.index,
            step.percentage,
            step.machine_indices.len(),
            step.machine_indices
        );
    }

    let result = execute_rollout(&canary_config, 10, false);
    println!(
        "  Result: completed={}, deployed={}/10\n",
        result.completed,
        result.deployed_count()
    );

    // 2. Percentage rollout
    println!("--- Percentage Rollout (8 machines) ---");
    let pct_config = RolloutConfig {
        strategy: "percentage".into(),
        canary_count: 0,
        health_check: Some("true".into()),
        health_timeout: None,
        percentage_steps: vec![25, 50, 75, 100],
    };

    let steps = plan_rollout(&pct_config, 8);
    for step in &steps {
        println!(
            "  Step {}: {}% ({} machines)",
            step.index,
            step.percentage,
            step.machine_indices.len()
        );
    }

    let result = execute_rollout(&pct_config, 8, false);
    println!(
        "  Result: completed={}, deployed={}/8\n",
        result.completed,
        result.deployed_count()
    );

    // 3. All-at-once rollout
    println!("--- All-at-Once Rollout (5 machines) ---");
    let all_config = RolloutConfig {
        strategy: "all-at-once".into(),
        canary_count: 0,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![],
    };

    let result = execute_rollout(&all_config, 5, false);
    println!(
        "  Steps: {}, completed={}\n",
        result.steps.len(),
        result.completed
    );

    // 4. Failing health check (auto-rollback)
    println!("--- Failing Health Check (auto-rollback) ---");
    let fail_config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: Some("exit 1".into()), // always fails
        health_timeout: Some("10s".into()),
        percentage_steps: vec![50, 100],
    };

    let result = execute_rollout(&fail_config, 4, false);
    println!(
        "  Completed: {}, rollback at step: {:?}",
        result.completed, result.rollback_at
    );
    for step in &result.steps {
        let icon = if step.health_passed { "OK" } else { "FAIL" };
        println!("  [{icon}] Step {}: {}", step.index, step.message);
    }

    // 5. Dry-run mode
    println!("\n--- Dry-Run Mode ---");
    let result = execute_rollout(&canary_config, 10, true);
    println!(
        "  Steps: {}, all passed: {}",
        result.steps.len(),
        result.steps.iter().all(|s| s.health_passed)
    );

    println!("\n--- Summary ---");
    println!("Strategies: canary, percentage, all-at-once");
    println!("Health checks run between each step");
    println!("Auto-rollback on health check failure");
}
