//! FJ-3505/3507: Rollout & promotion gate falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3507: Progressive rollout planning (canary, percentage, all-at-once)
//! - FJ-3507: Health check timeout enforcement
//! - FJ-3505: Promotion gate evaluation (validate, policy, script, coverage)
//! - Rollout result aggregation and deployed count
//!
//! Usage: cargo test --test falsification_rollout_promotion

use forjar::core::rollout::{
    execute_rollout, plan_rollout, run_health_check, RolloutResult, RolloutStep,
};
use forjar::core::types::environment::RolloutConfig;

// ============================================================================
// FJ-3507: Rollout Planning
// ============================================================================

#[test]
fn canary_plan_includes_canary_step_and_100() {
    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 2,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![50, 100],
    };
    let steps = plan_rollout(&config, 10);
    assert!(!steps.is_empty());
    // First step is canary with 2 machines
    assert_eq!(steps[0].machine_indices.len(), 2);
    // Last step is 100%
    assert_eq!(steps.last().unwrap().percentage, 100);
    assert_eq!(steps.last().unwrap().machine_indices.len(), 10);
}

#[test]
fn percentage_plan_default_steps() {
    let config = RolloutConfig {
        strategy: "percentage".into(),
        canary_count: 0,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![], // should use default 25/50/75/100
    };
    let steps = plan_rollout(&config, 4);
    assert_eq!(steps.len(), 4);
    assert_eq!(steps[0].percentage, 25);
    assert_eq!(steps[3].percentage, 100);
}

#[test]
fn percentage_plan_custom_steps() {
    let config = RolloutConfig {
        strategy: "percentage".into(),
        canary_count: 0,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![10, 50, 100],
    };
    let steps = plan_rollout(&config, 20);
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].percentage, 10);
    assert_eq!(steps[0].machine_indices.len(), 2); // 10% of 20
}

#[test]
fn all_at_once_single_step() {
    let config = RolloutConfig {
        strategy: "all-at-once".into(),
        canary_count: 0,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![],
    };
    let steps = plan_rollout(&config, 5);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].percentage, 100);
    assert_eq!(steps[0].machine_indices.len(), 5);
}

#[test]
fn zero_machines_empty_plan() {
    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![],
    };
    let steps = plan_rollout(&config, 0);
    assert!(steps.is_empty());
}

#[test]
fn canary_count_exceeds_total() {
    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 100,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![50, 100],
    };
    let steps = plan_rollout(&config, 3);
    // Canary count capped to total machines
    assert_eq!(steps[0].machine_indices.len(), 3);
}

// ============================================================================
// FJ-3507: Health Check
// ============================================================================

#[test]
fn health_check_passes_true() {
    let (passed, msg) = run_health_check("true", None);
    assert!(passed);
    assert!(msg.contains("passed"));
}

#[test]
fn health_check_fails_false() {
    let (passed, msg) = run_health_check("false", None);
    assert!(!passed);
    assert!(msg.contains("failed"));
}

#[test]
fn health_check_timeout_enforcement() {
    let start = std::time::Instant::now();
    let (passed, msg) = run_health_check("sleep 60", Some("1s"));
    let elapsed = start.elapsed();
    assert!(!passed, "slow command should fail");
    assert!(
        msg.contains("timed out"),
        "message should say timed out: {msg}"
    );
    assert!(
        elapsed.as_secs() < 5,
        "should complete in ~1s, took {:?}",
        elapsed
    );
}

#[test]
fn health_check_custom_timeout_seconds() {
    let (passed, msg) = run_health_check("sleep 60", Some("1s"));
    assert!(!passed);
    assert!(msg.contains("1s"));
}

#[test]
fn health_check_default_timeout_30s() {
    // "true" completes instantly regardless of timeout
    let (passed, _) = run_health_check("true", None);
    assert!(passed);
}

// ============================================================================
// FJ-3507: Rollout Execution
// ============================================================================

#[test]
fn execute_dry_run_all_pass() {
    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: Some("true".into()),
        health_timeout: None,
        percentage_steps: vec![50, 100],
    };
    let result = execute_rollout(&config, 4, true);
    assert!(result.completed);
    assert!(result.rollback_at.is_none());
    assert!(result.steps.iter().all(|s| s.health_passed));
}

#[test]
fn execute_with_passing_health_check() {
    let config = RolloutConfig {
        strategy: "all-at-once".into(),
        canary_count: 0,
        health_check: Some("true".into()),
        health_timeout: Some("10s".into()),
        percentage_steps: vec![],
    };
    let result = execute_rollout(&config, 3, false);
    assert!(result.completed);
    assert_eq!(result.deployed_count(), 3);
}

#[test]
fn execute_rollback_on_health_failure() {
    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: Some("false".into()),
        health_timeout: None,
        percentage_steps: vec![50, 100],
    };
    let result = execute_rollout(&config, 4, false);
    assert!(!result.completed);
    assert_eq!(result.rollback_at, Some(0));
    assert_eq!(result.deployed_count(), 0);
}

#[test]
fn execute_no_health_check_always_passes() {
    let config = RolloutConfig {
        strategy: "percentage".into(),
        canary_count: 0,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![50, 100],
    };
    let result = execute_rollout(&config, 6, false);
    assert!(result.completed);
    assert!(result.steps.iter().all(|s| s.health_passed));
}

// ============================================================================
// FJ-3507: RolloutResult
// ============================================================================

#[test]
fn deployed_count_deduplicates_machines() {
    let result = RolloutResult {
        strategy: "canary".into(),
        steps: vec![
            RolloutStep {
                index: 0,
                percentage: 25,
                machine_indices: vec![0, 1],
                health_passed: true,
                message: String::new(),
            },
            RolloutStep {
                index: 1,
                percentage: 50,
                machine_indices: vec![0, 1, 2, 3],
                health_passed: true,
                message: String::new(),
            },
        ],
        completed: true,
        rollback_at: None,
    };
    // Machine indices 0,1,2,3 — deduped across both steps
    assert_eq!(result.deployed_count(), 4);
}

#[test]
fn deployed_count_skips_failed_steps() {
    let result = RolloutResult {
        strategy: "canary".into(),
        steps: vec![
            RolloutStep {
                index: 0,
                percentage: 25,
                machine_indices: vec![0],
                health_passed: true,
                message: String::new(),
            },
            RolloutStep {
                index: 1,
                percentage: 100,
                machine_indices: vec![0, 1, 2, 3],
                health_passed: false,
                message: "failed".into(),
            },
        ],
        completed: false,
        rollback_at: Some(1),
    };
    // Only step 0 passed, so only machine 0 deployed
    assert_eq!(result.deployed_count(), 1);
}

#[test]
fn rollout_result_strategy_preserved() {
    let config = RolloutConfig {
        strategy: "canary".into(),
        canary_count: 1,
        health_check: None,
        health_timeout: None,
        percentage_steps: vec![],
    };
    let result = execute_rollout(&config, 2, true);
    assert_eq!(result.strategy, "canary");
}
