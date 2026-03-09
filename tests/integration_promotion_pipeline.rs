//! FJ-3500: Integration test — Environment promotion pipeline.
//!
//! Tests the full promotion flow: gate evaluation → event logging →
//! rollback logging → multi-environment state isolation.

use forjar::core::promotion::{evaluate_gates, GateResult, PromotionResult};
use forjar::core::promotion_events::{
    log_promotion, log_promotion_failure, log_rollback, PromotionParams,
};
use forjar::core::types::environment::{PromotionConfig, PromotionGate, ValidateGateOptions};
use tempfile::TempDir;

fn write_valid_config(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("forjar.yaml");
    std::fs::write(
        &path,
        r#"
version: "1.0"
name: promotion-test
machines:
  web:
    hostname: web-01
    addr: 127.0.0.1
resources:
  nginx:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
"#,
    )
    .unwrap();
    path
}

/// Test: dev→staging promotion with all gates passing.
#[test]
fn dev_to_staging_all_gates_pass() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());

    let promotion = PromotionConfig {
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
                script: Some("true".into()),
                ..Default::default()
            },
        ],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(result.all_passed);
    assert_eq!(result.from, "dev");
    assert_eq!(result.to, "staging");
    assert_eq!(result.passed_count(), 2);
    assert_eq!(result.failed_count(), 0);
}

/// Test: staging→prod promotion with failing gate blocks.
#[test]
fn staging_to_prod_gate_failure() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());

    let promotion = PromotionConfig {
        from: "staging".into(),
        gates: vec![
            PromotionGate {
                validate: Some(ValidateGateOptions {
                    deep: false,
                    exhaustive: false,
                }),
                ..Default::default()
            },
            PromotionGate {
                script: Some("exit 1".into()),
                ..Default::default()
            },
        ],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "prod", &promotion);
    assert!(!result.all_passed);
    assert_eq!(result.failed_count(), 1);
}

/// Test: auto-approve flag is preserved.
#[test]
fn auto_approve_preserved() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());

    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![],
        auto_approve: true,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(result.auto_approve);
    assert!(result.all_passed); // no gates = all passed
}

/// Test: promotion success event logged to target env directory.
#[test]
fn promotion_event_logged() {
    let dir = TempDir::new().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let params = PromotionParams {
        state_dir: &state_dir,
        target_env: "staging",
        source: "dev",
        target: "staging",
        gates_passed: 3,
        gates_total: 3,
        rollout_strategy: Some("canary"),
    };
    log_promotion(&params).unwrap();

    let log_path = state_dir.join("staging").join("events.jsonl");
    assert!(log_path.exists());
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("promotion_completed"));
    assert!(content.contains("\"success\":true"));
    assert!(content.contains("\"source\":\"dev\""));
    assert!(content.contains("canary"));
}

/// Test: promotion failure event logged.
#[test]
fn promotion_failure_event_logged() {
    let dir = TempDir::new().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    let params = PromotionParams {
        state_dir: &state_dir,
        target_env: "prod",
        source: "staging",
        target: "prod",
        gates_passed: 1,
        gates_total: 4,
        rollout_strategy: None,
    };
    log_promotion_failure(&params).unwrap();

    let log_path = state_dir.join("prod").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("\"success\":false"));
    assert!(content.contains("\"gates_passed\":1"));
    assert!(content.contains("\"gates_total\":4"));
}

/// Test: rollback event logged after health check failure.
#[test]
fn rollback_after_health_failure() {
    let dir = TempDir::new().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    log_rollback(&state_dir, "prod", 0, "canary health check timed out").unwrap();

    let log_path = state_dir.join("prod").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("rollback_triggered"));
    assert!(content.contains("canary health check timed out"));
}

/// Test: full pipeline: evaluate → log success → verify history.
#[test]
fn full_pipeline_evaluate_log_verify() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // 1. Evaluate gates
    let promotion = PromotionConfig {
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
                script: Some("echo 'health check OK'".into()),
                ..Default::default()
            },
        ],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(result.all_passed);

    // 2. Log the promotion
    let params = PromotionParams {
        state_dir: &state_dir,
        target_env: "staging",
        source: &result.from,
        target: &result.to,
        gates_passed: result.passed_count() as u32,
        gates_total: result.gates.len() as u32,
        rollout_strategy: None,
    };
    log_promotion(&params).unwrap();

    // 3. Verify the event log
    let log_path = state_dir.join("staging").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(content.contains("\"success\":true"));
    assert!(content.contains("\"gates_passed\":2"));
}

/// Test: full pipeline with failure: evaluate → log failure → rollback.
#[test]
fn full_pipeline_with_failure_and_rollback() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // 1. Evaluate gates (one fails)
    let promotion = PromotionConfig {
        from: "staging".into(),
        gates: vec![
            PromotionGate {
                validate: Some(ValidateGateOptions {
                    deep: false,
                    exhaustive: false,
                }),
                ..Default::default()
            },
            PromotionGate {
                script: Some("false".into()),
                ..Default::default()
            },
        ],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "prod", &promotion);
    assert!(!result.all_passed);

    // 2. Log the failure
    let params = PromotionParams {
        state_dir: &state_dir,
        target_env: "prod",
        source: &result.from,
        target: &result.to,
        gates_passed: result.passed_count() as u32,
        gates_total: result.gates.len() as u32,
        rollout_strategy: None,
    };
    log_promotion_failure(&params).unwrap();

    // 3. Log rollback
    log_rollback(&state_dir, "prod", 1, "gate 'script' failed").unwrap();

    // 4. Verify event log has both events
    let log_path = state_dir.join("prod").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(content.contains("\"success\":false"));
    assert!(content.contains("rollback_triggered"));
}

/// Test: multiple environments have isolated event logs.
#[test]
fn multi_env_isolated_logs() {
    let dir = TempDir::new().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();

    // Log to staging
    let staging = PromotionParams {
        state_dir: &state_dir,
        target_env: "staging",
        source: "dev",
        target: "staging",
        gates_passed: 2,
        gates_total: 2,
        rollout_strategy: None,
    };
    log_promotion(&staging).unwrap();

    // Log to prod
    let prod = PromotionParams {
        state_dir: &state_dir,
        target_env: "prod",
        source: "staging",
        target: "prod",
        gates_passed: 4,
        gates_total: 4,
        rollout_strategy: Some("canary"),
    };
    log_promotion(&prod).unwrap();

    // Each env has its own log
    let staging_log = std::fs::read_to_string(state_dir.join("staging/events.jsonl")).unwrap();
    let prod_log = std::fs::read_to_string(state_dir.join("prod/events.jsonl")).unwrap();

    assert!(staging_log.contains("\"source\":\"dev\""));
    assert!(!staging_log.contains("\"source\":\"staging\""));
    assert!(prod_log.contains("\"source\":\"staging\""));
    assert!(prod_log.contains("canary"));
}

/// Test: validate gate with deep mode.
#[test]
fn validate_gate_deep_mode() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());

    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![PromotionGate {
            validate: Some(ValidateGateOptions {
                deep: true,
                exhaustive: false,
            }),
            ..Default::default()
        }],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(result.all_passed);
    assert!(result.gates[0].message.contains("deep"));
}

/// Test: invalid config fails validate gate.
#[test]
fn invalid_config_fails_validate_gate() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(&cfg, "this: [is invalid yaml").unwrap();

    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![PromotionGate {
            validate: Some(ValidateGateOptions {
                deep: false,
                exhaustive: false,
            }),
            ..Default::default()
        }],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(!result.all_passed);
    assert!(result.gates[0].message.contains("failed"));
}

/// Test: PromotionResult counts are correct.
#[test]
fn promotion_result_counts() {
    let result = PromotionResult {
        from: "dev".into(),
        to: "staging".into(),
        gates: vec![
            GateResult {
                gate_type: "validate".into(),
                passed: true,
                message: "ok".into(),
            },
            GateResult {
                gate_type: "policy".into(),
                passed: true,
                message: "ok".into(),
            },
            GateResult {
                gate_type: "script".into(),
                passed: false,
                message: "fail".into(),
            },
        ],
        all_passed: false,
        auto_approve: false,
    };
    assert_eq!(result.passed_count(), 2);
    assert_eq!(result.failed_count(), 1);
}
