//! FJ-3505/3509: Promotion gates and event logging falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-3505: Gate evaluation (validate, policy, coverage, script)
//!   - PromotionResult: passed_count, failed_count, all_passed
//!   - evaluate_gates with mixed pass/fail
//!   - Unknown gate type
//! - FJ-3509: Promotion event logging
//!   - log_promotion success event
//!   - log_promotion_failure event
//!   - log_rollback event
//!   - Event append (multiple events)
//!   - ProvenanceEvent serde roundtrip
//!
//! Usage: cargo test --test falsification_promotion_events

use forjar::core::promotion::{evaluate_gates, GateResult, PromotionResult};
use forjar::core::promotion_events::{
    log_promotion, log_promotion_failure, log_rollback, PromotionParams,
};
use forjar::core::types::environment::{
    CoverageGateOptions, PolicyGateOptions, PromotionConfig, PromotionGate, ValidateGateOptions,
};
use forjar::core::types::ProvenanceEvent;

// ============================================================================
// Helpers
// ============================================================================

fn write_valid_config(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("forjar.yaml");
    std::fs::write(
        &path,
        r#"
version: "1.0"
name: test-promo
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
    )
    .unwrap();
    path
}

fn make_state_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    (dir, state_dir)
}

// ============================================================================
// FJ-3505: GateResult fields
// ============================================================================

#[test]
fn gate_result_fields() {
    let gr = GateResult {
        gate_type: "validate".into(),
        passed: true,
        message: "ok".into(),
    };
    assert_eq!(gr.gate_type, "validate");
    assert!(gr.passed);
    assert_eq!(gr.message, "ok");
}

#[test]
fn gate_result_failure() {
    let gr = GateResult {
        gate_type: "script".into(),
        passed: false,
        message: "exit 1".into(),
    };
    assert!(!gr.passed);
}

// ============================================================================
// FJ-3505: PromotionResult counting
// ============================================================================

#[test]
fn promotion_result_all_passed() {
    let pr = PromotionResult {
        from: "dev".into(),
        to: "staging".into(),
        gates: vec![
            GateResult {
                gate_type: "validate".into(),
                passed: true,
                message: "ok".into(),
            },
            GateResult {
                gate_type: "script".into(),
                passed: true,
                message: "ok".into(),
            },
        ],
        all_passed: true,
        auto_approve: false,
    };
    assert_eq!(pr.passed_count(), 2);
    assert_eq!(pr.failed_count(), 0);
    assert!(pr.all_passed);
}

#[test]
fn promotion_result_mixed() {
    let pr = PromotionResult {
        from: "dev".into(),
        to: "prod".into(),
        gates: vec![
            GateResult {
                gate_type: "validate".into(),
                passed: true,
                message: "ok".into(),
            },
            GateResult {
                gate_type: "script".into(),
                passed: false,
                message: "fail".into(),
            },
            GateResult {
                gate_type: "coverage".into(),
                passed: false,
                message: "low".into(),
            },
        ],
        all_passed: false,
        auto_approve: true,
    };
    assert_eq!(pr.passed_count(), 1);
    assert_eq!(pr.failed_count(), 2);
    assert!(!pr.all_passed);
    assert!(pr.auto_approve);
}

#[test]
fn promotion_result_empty_gates() {
    let pr = PromotionResult {
        from: "dev".into(),
        to: "staging".into(),
        gates: vec![],
        all_passed: true,
        auto_approve: false,
    };
    assert_eq!(pr.passed_count(), 0);
    assert_eq!(pr.failed_count(), 0);
}

// ============================================================================
// FJ-3505: evaluate_gates — validate gate passes
// ============================================================================

#[test]
fn evaluate_gates_validate_passes() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_valid_config(dir.path());
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
    assert_eq!(result.from, "dev");
    assert_eq!(result.to, "staging");
    assert!(
        result.all_passed,
        "validate gate should pass: {:?}",
        result.gates
    );
    assert_eq!(result.passed_count(), 1);
}

// ============================================================================
// FJ-3505: evaluate_gates — validate gate fails on bad config
// ============================================================================

#[test]
fn evaluate_gates_validate_fails_bad_config() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(&cfg, "this is: not [valid: yaml").unwrap();

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
    assert_eq!(result.failed_count(), 1);
    assert_eq!(result.gates[0].gate_type, "validate");
}

// ============================================================================
// FJ-3505: evaluate_gates — script gate
// ============================================================================

#[test]
fn evaluate_gates_script_passes() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_valid_config(dir.path());
    let promotion = PromotionConfig {
        from: "staging".into(),
        gates: vec![PromotionGate {
            script: Some("true".into()),
            ..Default::default()
        }],
        auto_approve: true,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "prod", &promotion);
    assert!(result.all_passed);
    assert!(result.auto_approve);
}

#[test]
fn evaluate_gates_script_fails() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_valid_config(dir.path());
    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![PromotionGate {
            script: Some("false".into()),
            ..Default::default()
        }],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(!result.all_passed);
    assert_eq!(result.gates[0].gate_type, "script");
}

// ============================================================================
// FJ-3505: evaluate_gates — mixed gates
// ============================================================================

#[test]
fn evaluate_gates_mixed_pass_fail() {
    let dir = tempfile::tempdir().unwrap();
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
                script: Some("false".into()),
                ..Default::default()
            },
        ],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(!result.all_passed);
    assert_eq!(result.passed_count(), 1);
    assert_eq!(result.failed_count(), 1);
}

// ============================================================================
// FJ-3505: evaluate_gates — unknown gate
// ============================================================================

#[test]
fn evaluate_gates_unknown_type() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_valid_config(dir.path());
    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![PromotionGate::default()],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(!result.all_passed);
    assert_eq!(result.gates[0].gate_type, "unknown");
}

// ============================================================================
// FJ-3505: evaluate_gates — policy gate no policies
// ============================================================================

#[test]
fn evaluate_gates_policy_no_policies() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_valid_config(dir.path());
    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![PromotionGate {
            policy: Some(PolicyGateOptions { strict: false }),
            ..Default::default()
        }],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(
        result.all_passed,
        "policy with no policies should pass: {:?}",
        result.gates
    );
}

// ============================================================================
// FJ-3505: evaluate_gates — empty gates
// ============================================================================

#[test]
fn evaluate_gates_empty() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_valid_config(dir.path());
    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(result.all_passed);
    assert_eq!(result.gates.len(), 0);
}

// ============================================================================
// FJ-3505: PromotionGate type detection
// ============================================================================

#[test]
fn gate_type_validate() {
    let g = PromotionGate {
        validate: Some(ValidateGateOptions {
            deep: true,
            exhaustive: false,
        }),
        ..Default::default()
    };
    assert_eq!(g.gate_type(), "validate");
}

#[test]
fn gate_type_policy() {
    let g = PromotionGate {
        policy: Some(PolicyGateOptions { strict: true }),
        ..Default::default()
    };
    assert_eq!(g.gate_type(), "policy");
}

#[test]
fn gate_type_coverage() {
    let g = PromotionGate {
        coverage: Some(CoverageGateOptions { min: 80 }),
        ..Default::default()
    };
    assert_eq!(g.gate_type(), "coverage");
}

#[test]
fn gate_type_script() {
    let g = PromotionGate {
        script: Some("echo ok".into()),
        ..Default::default()
    };
    assert_eq!(g.gate_type(), "script");
}

#[test]
fn gate_type_unknown() {
    let g = PromotionGate::default();
    assert_eq!(g.gate_type(), "unknown");
}

// ============================================================================
// FJ-3509: log_promotion success
// ============================================================================

#[test]
fn log_promotion_success_event() {
    let (_dir, state_dir) = make_state_dir();
    let params = PromotionParams {
        state_dir: &state_dir,
        target_env: "staging",
        source: "dev",
        target: "staging",
        gates_passed: 3,
        gates_total: 3,
        rollout_strategy: Some("canary"),
    };
    assert!(log_promotion(&params).is_ok());

    let log_path = state_dir.join("staging").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("promotion_completed"));
    assert!(content.contains("\"success\":true"));
    assert!(content.contains("\"gates_passed\":3"));
    assert!(content.contains("canary"));
}

#[test]
fn log_promotion_success_without_rollout() {
    let (_dir, state_dir) = make_state_dir();
    let params = PromotionParams {
        state_dir: &state_dir,
        target_env: "prod",
        source: "staging",
        target: "prod",
        gates_passed: 5,
        gates_total: 5,
        rollout_strategy: None,
    };
    assert!(log_promotion(&params).is_ok());

    let log_path = state_dir.join("prod").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("\"success\":true"));
    assert!(content.contains("\"gates_passed\":5"));
}

// ============================================================================
// FJ-3509: log_promotion_failure
// ============================================================================

#[test]
fn log_promotion_failure_event() {
    let (_dir, state_dir) = make_state_dir();
    let params = PromotionParams {
        state_dir: &state_dir,
        target_env: "prod",
        source: "staging",
        target: "prod",
        gates_passed: 1,
        gates_total: 4,
        rollout_strategy: None,
    };
    assert!(log_promotion_failure(&params).is_ok());

    let log_path = state_dir.join("prod").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("\"success\":false"));
    assert!(content.contains("\"gates_passed\":1"));
    assert!(content.contains("\"gates_total\":4"));
}

// ============================================================================
// FJ-3509: log_rollback
// ============================================================================
