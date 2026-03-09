//! FJ-202/3507/036: Conditions, rollout, purifier falsification.
//! Usage: cargo test --test falsification_conditions_rollout_purifier

use forjar::core::conditions::evaluate_when;
use forjar::core::purifier::{
    lint_error_count, purify_script, validate_or_purify, validate_script,
};
use forjar::core::rollout::{
    execute_rollout, plan_rollout, run_health_check, RolloutResult, RolloutStep,
};
use forjar::core::types::environment::RolloutConfig;
use forjar::core::types::Machine;
use std::collections::HashMap;

// ── helpers ──

fn machine(arch: &str) -> Machine {
    Machine {
        hostname: "test-host".into(),
        addr: "10.0.0.1".into(),
        user: "root".into(),
        arch: arch.into(),
        ssh_key: None,
        roles: vec!["web".into(), "gpu".into()],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

fn params() -> HashMap<String, serde_yaml_ng::Value> {
    let mut p = HashMap::new();
    p.insert("env".into(), serde_yaml_ng::Value::String("prod".into()));
    p.insert("flag".into(), serde_yaml_ng::Value::Bool(true));
    p
}

fn rc(strategy: &str, canary: usize, steps: Vec<u32>) -> RolloutConfig {
    RolloutConfig {
        strategy: strategy.into(),
        canary_count: canary,
        health_check: None,
        health_timeout: None,
        percentage_steps: steps,
    }
}

fn step(idx: usize, pct: u32, machines: Vec<usize>, passed: bool) -> RolloutStep {
    RolloutStep {
        index: idx,
        percentage: pct,
        machine_indices: machines,
        health_passed: passed,
        message: String::new(),
    }
}

// ── FJ-202: evaluate_when ──

#[test]
fn when_literal_true_false() {
    let m = machine("x86_64");
    let p = HashMap::new();
    assert!(evaluate_when("true", &p, &m).unwrap());
    assert!(!evaluate_when("false", &p, &m).unwrap());
    assert!(evaluate_when("TRUE", &p, &m).unwrap());
    assert!(!evaluate_when("FALSE", &p, &m).unwrap());
}

#[test]
fn when_machine_arch_eq() {
    let m = machine("aarch64");
    let p = HashMap::new();
    assert!(evaluate_when("{{machine.arch}} == \"aarch64\"", &p, &m).unwrap());
    assert!(!evaluate_when("{{machine.arch}} == \"x86_64\"", &p, &m).unwrap());
}

#[test]
fn when_machine_hostname_addr_user() {
    let m = machine("x86_64");
    let p = HashMap::new();
    assert!(evaluate_when("{{machine.hostname}} == \"test-host\"", &p, &m).unwrap());
    assert!(evaluate_when("{{machine.addr}} == \"10.0.0.1\"", &p, &m).unwrap());
    assert!(evaluate_when("{{machine.user}} == \"root\"", &p, &m).unwrap());
}

#[test]
fn when_not_equals() {
    let m = machine("x86_64");
    let p = params();
    assert!(evaluate_when("{{params.env}} != \"staging\"", &p, &m).unwrap());
    assert!(!evaluate_when("{{params.env}} != \"prod\"", &p, &m).unwrap());
}

#[test]
fn when_contains() {
    let m = machine("x86_64");
    let p = HashMap::new();
    assert!(evaluate_when("{{machine.roles}} contains \"gpu\"", &p, &m).unwrap());
    assert!(!evaluate_when("{{machine.roles}} contains \"storage\"", &p, &m).unwrap());
}

#[test]
fn when_param_substitution() {
    let m = machine("x86_64");
    let p = params();
    assert!(evaluate_when("{{params.env}} == \"prod\"", &p, &m).unwrap());
    assert!(evaluate_when("{{params.flag}} == \"true\"", &p, &m).unwrap());
}

#[test]
fn when_single_quoted() {
    let m = machine("x86_64");
    let p = HashMap::new();
    assert!(evaluate_when("{{machine.arch}} == 'x86_64'", &p, &m).unwrap());
}

#[test]
fn when_whitespace_trim() {
    let m = machine("x86_64");
    assert!(evaluate_when("  true  ", &HashMap::new(), &m).unwrap());
}

#[test]
fn when_error_unknown_param() {
    let m = machine("x86_64");
    let err = evaluate_when("{{params.missing}} == \"x\"", &HashMap::new(), &m).unwrap_err();
    assert!(err.contains("unknown param"));
}

#[test]
fn when_error_unknown_field() {
    let m = machine("x86_64");
    let err = evaluate_when("{{machine.bogus}} == \"x\"", &HashMap::new(), &m).unwrap_err();
    assert!(err.contains("unknown machine field"));
}

#[test]
fn when_error_invalid_expr() {
    let m = machine("x86_64");
    let err = evaluate_when("no operator here", &HashMap::new(), &m).unwrap_err();
    assert!(err.contains("invalid when expression"));
}

#[test]
fn when_error_unclosed_template() {
    let m = machine("x86_64");
    let err = evaluate_when("{{machine.arch == \"x\"", &HashMap::new(), &m).unwrap_err();
    assert!(err.contains("unclosed template"));
}

#[test]
fn when_error_unknown_template_var() {
    let m = machine("x86_64");
    let err = evaluate_when("{{unknown.var}} == \"x\"", &HashMap::new(), &m).unwrap_err();
    assert!(err.contains("unknown template variable"));
}

// ── FJ-3507: plan_rollout ──

#[test]
fn plan_canary_basic() {
    let steps = plan_rollout(&rc("canary", 1, vec![10, 25, 50, 100]), 10);
    assert!(!steps.is_empty());
    assert_eq!(steps[0].machine_indices.len(), 1);
    assert_eq!(steps.last().unwrap().percentage, 100);
}

#[test]
fn plan_canary_single_machine() {
    let steps = plan_rollout(&rc("canary", 1, vec![50, 100]), 1);
    assert!(!steps.is_empty());
    assert_eq!(steps[0].machine_indices.len(), 1);
}

#[test]
fn plan_percentage_explicit() {
    let steps = plan_rollout(&rc("percentage", 0, vec![25, 50, 100]), 8);
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].percentage, 25);
    assert_eq!(steps[0].machine_indices.len(), 2);
    assert_eq!(steps[2].machine_indices.len(), 8);
}

#[test]
fn plan_percentage_default_steps() {
    let steps = plan_rollout(&rc("percentage", 0, vec![]), 4);
    assert_eq!(steps.len(), 4); // 25, 50, 75, 100
}

#[test]
fn plan_all_at_once() {
    let steps = plan_rollout(&rc("all-at-once", 0, vec![]), 5);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].percentage, 100);
    assert_eq!(steps[0].machine_indices.len(), 5);
}

#[test]
fn plan_zero_machines() {
    assert!(plan_rollout(&rc("canary", 1, vec![]), 0).is_empty());
}

#[test]
fn plan_unknown_strategy_falls_to_all_at_once() {
    let steps = plan_rollout(&rc("yolo", 0, vec![]), 3);
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].percentage, 100);
}

// ── FJ-3507: RolloutResult ──

#[test]
fn deployed_count_dedup() {
    let r = RolloutResult {
        strategy: "canary".into(),
        steps: vec![step(0, 25, vec![0], true), step(1, 50, vec![0, 1], true)],
        completed: true,
        rollback_at: None,
    };
    assert_eq!(r.deployed_count(), 2); // {0,1} deduped
}

#[test]
fn deployed_count_excludes_failed() {
    let r = RolloutResult {
        strategy: "canary".into(),
        steps: vec![
            step(0, 25, vec![0], true),
            step(1, 50, vec![0, 1, 2], false),
        ],
        completed: false,
        rollback_at: Some(1),
    };
    assert_eq!(r.deployed_count(), 1); // only step 0 passed
}

#[test]
fn deployed_count_empty() {
    let r = RolloutResult {
        strategy: "x".into(),
        steps: vec![],
        completed: true,
        rollback_at: None,
    };
    assert_eq!(r.deployed_count(), 0);
}

// ── FJ-3507: execute_rollout ──

#[test]
fn execute_dry_run() {
    let result = execute_rollout(&rc("canary", 1, vec![50, 100]), 5, true);
    assert!(result.completed);
    assert!(result.rollback_at.is_none());
    assert!(result.steps.iter().all(|s| s.health_passed));
}

#[test]
fn execute_no_health_check() {
    let result = execute_rollout(&rc("all-at-once", 0, vec![]), 3, false);
    assert!(result.completed);
    assert!(result
        .steps
        .iter()
        .all(|s| s.message.contains("no health check")));
}

#[test]
fn execute_passing_health() {
    let mut config = rc("all-at-once", 0, vec![]);
    config.health_check = Some("true".into());
    config.health_timeout = Some("5s".into());
    let result = execute_rollout(&config, 2, false);
    assert!(result.completed);
    assert_eq!(result.deployed_count(), 2);
}

#[test]
fn execute_failing_health_rolls_back() {
    let mut config = rc("canary", 1, vec![50, 100]);
    config.health_check = Some("false".into());
    let result = execute_rollout(&config, 4, false);
    assert!(!result.completed);
    assert_eq!(result.rollback_at, Some(0));
}

// ── FJ-3507: run_health_check ──

#[test]
fn health_check_pass_and_fail() {
    let (ok, msg) = run_health_check("true", None);
    assert!(ok);
    assert!(msg.contains("passed"));
    let (fail, _) = run_health_check("false", None);
    assert!(!fail);
}

#[test]
fn health_check_timeout() {
    let start = std::time::Instant::now();
    let (passed, msg) = run_health_check("sleep 60", Some("1s"));
    assert!(!passed);
    assert!(msg.contains("timed out"));
    assert!(start.elapsed().as_secs() < 5);
}

// ── FJ-036: validate_script ──

#[test]
fn validate_clean_script() {
    assert!(validate_script("echo hello\n").is_ok());
}

#[test]
fn validate_simple_if() {
    assert!(validate_script("if [ -f /etc/hosts ]; then echo ok; fi\n").is_ok());
}

// ── FJ-036: lint_error_count ──

#[test]
fn lint_error_count_clean() {
    assert_eq!(lint_error_count("echo hello\n"), 0);
}

// ── FJ-036: purify_script ──

#[test]
fn purify_clean_script() {
    let result = purify_script("echo hello\n").unwrap();
    assert!(result.contains("echo"));
}

#[test]
fn purify_preserves_semantics() {
    let result = purify_script("x=1\necho $x\n").unwrap();
    assert!(result.contains("x="));
    assert!(result.contains("echo"));
}

// ── FJ-036: validate_or_purify ──

#[test]
fn validate_or_purify_clean_passes_fast() {
    let result = validate_or_purify("echo hello\n").unwrap();
    assert_eq!(result, "echo hello\n"); // fast path: returned as-is
}
