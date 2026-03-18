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
#![allow(dead_code)]

use forjar::core::promotion::evaluate_gates;
use forjar::core::promotion_events::{log_promotion, log_rollback, PromotionParams};
use forjar::core::types::environment::{PromotionConfig, PromotionGate, ValidateGateOptions};
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
#[test]
fn log_rollback_event() {
    let (_dir, state_dir) = make_state_dir();
    assert!(log_rollback(&state_dir, "prod", 2, "health check timeout").is_ok());

    let log_path = state_dir.join("prod").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("rollback_triggered"));
    assert!(content.contains("health check timeout"));
    assert!(content.contains("\"failed_step\":2"));
}

#[test]
fn log_rollback_step_zero() {
    let (_dir, state_dir) = make_state_dir();
    assert!(log_rollback(&state_dir, "staging", 0, "canary failed").is_ok());

    let log_path = state_dir.join("staging").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("\"failed_step\":0"));
}

// ============================================================================
// FJ-3509: Multiple events append
// ============================================================================

#[test]
fn multiple_events_append_sequentially() {
    let (_dir, state_dir) = make_state_dir();

    let params1 = PromotionParams {
        state_dir: &state_dir,
        target_env: "staging",
        source: "dev",
        target: "staging",
        gates_passed: 2,
        gates_total: 2,
        rollout_strategy: None,
    };
    log_promotion(&params1).unwrap();

    let params2 = PromotionParams {
        state_dir: &state_dir,
        target_env: "staging",
        source: "dev",
        target: "staging",
        gates_passed: 3,
        gates_total: 3,
        rollout_strategy: Some("percentage"),
    };
    log_promotion(&params2).unwrap();

    log_rollback(&state_dir, "staging", 1, "rollback reason").unwrap();

    let log_path = state_dir.join("staging").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3);
}

// ============================================================================
// FJ-3509: ProvenanceEvent serde roundtrip — promotion
// ============================================================================

#[test]
fn provenance_event_promotion_serde() {
    let event = ProvenanceEvent::PromotionCompleted {
        source: "dev".into(),
        target: "staging".into(),
        success: true,
        gates_passed: 4,
        gates_total: 4,
        rollout_strategy: Some("percentage".into()),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("promotion_completed"));
    let deser: ProvenanceEvent = serde_json::from_str(&json).unwrap();
    match deser {
        ProvenanceEvent::PromotionCompleted {
            source,
            target,
            success,
            gates_passed,
            gates_total,
            rollout_strategy,
        } => {
            assert_eq!(source, "dev");
            assert_eq!(target, "staging");
            assert!(success);
            assert_eq!(gates_passed, 4);
            assert_eq!(gates_total, 4);
            assert_eq!(rollout_strategy.as_deref(), Some("percentage"));
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn provenance_event_promotion_failure_serde() {
    let event = ProvenanceEvent::PromotionCompleted {
        source: "staging".into(),
        target: "prod".into(),
        success: false,
        gates_passed: 2,
        gates_total: 5,
        rollout_strategy: None,
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"success\":false"));
    let deser: ProvenanceEvent = serde_json::from_str(&json).unwrap();
    match deser {
        ProvenanceEvent::PromotionCompleted {
            success,
            gates_passed,
            gates_total,
            ..
        } => {
            assert!(!success);
            assert_eq!(gates_passed, 2);
            assert_eq!(gates_total, 5);
        }
        _ => panic!("wrong variant"),
    }
}

// ============================================================================
// FJ-3509: ProvenanceEvent serde roundtrip — rollback
// ============================================================================

#[test]
fn provenance_event_rollback_serde() {
    let event = ProvenanceEvent::RollbackTriggered {
        environment: "prod".into(),
        failed_step: 3,
        reason: "timeout exceeded".into(),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("rollback_triggered"));
    let deser: ProvenanceEvent = serde_json::from_str(&json).unwrap();
    match deser {
        ProvenanceEvent::RollbackTriggered {
            environment,
            failed_step,
            reason,
        } => {
            assert_eq!(environment, "prod");
            assert_eq!(failed_step, 3);
            assert_eq!(reason, "timeout exceeded");
        }
        _ => panic!("wrong variant"),
    }
}

// ============================================================================
// FJ-3509: Source/target naming preserved
// ============================================================================

#[test]
fn source_target_preserved_in_log() {
    let (_dir, state_dir) = make_state_dir();
    let params = PromotionParams {
        state_dir: &state_dir,
        target_env: "production",
        source: "staging-us-east",
        target: "production-us-east",
        gates_passed: 1,
        gates_total: 1,
        rollout_strategy: None,
    };
    log_promotion(&params).unwrap();

    let log_path = state_dir.join("production").join("events.jsonl");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("staging-us-east"));
    assert!(content.contains("production-us-east"));
}

// ============================================================================
// FJ-3505: evaluate_gates preserves auto_approve flag
// ============================================================================

#[test]
fn evaluate_gates_auto_approve_true() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_valid_config(dir.path());
    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![PromotionGate {
            script: Some("true".into()),
            ..Default::default()
        }],
        auto_approve: true,
        rollout: None,
    };
    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(result.auto_approve);
}

#[test]
fn evaluate_gates_auto_approve_false() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_valid_config(dir.path());
    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![PromotionGate {
            script: Some("true".into()),
            ..Default::default()
        }],
        auto_approve: false,
        rollout: None,
    };
    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert!(!result.auto_approve);
}

// ============================================================================
// FJ-3505: evaluate_gates — deep validate
// ============================================================================

#[test]
fn evaluate_gates_deep_validate() {
    let dir = tempfile::tempdir().unwrap();
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
