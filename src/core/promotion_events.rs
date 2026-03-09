//! FJ-3509: Promotion event logging.
//!
//! Appends structured events to events.jsonl when promotions
//! succeed, fail, or trigger rollback.

use crate::core::types::ProvenanceEvent;
use crate::tripwire::eventlog;
use std::path::Path;

/// Parameters for logging a promotion event.
pub struct PromotionParams<'a> {
    /// State directory path.
    pub state_dir: &'a Path,
    /// Target environment name.
    pub target_env: &'a str,
    /// Source environment.
    pub source: &'a str,
    /// Target environment.
    pub target: &'a str,
    /// Number of quality gates passed.
    pub gates_passed: u32,
    /// Total quality gates.
    pub gates_total: u32,
    /// Rollout strategy used (if any).
    pub rollout_strategy: Option<&'a str>,
}

/// Log a successful promotion event.
pub fn log_promotion(params: &PromotionParams<'_>) -> Result<(), String> {
    let event = ProvenanceEvent::PromotionCompleted {
        source: params.source.to_string(),
        target: params.target.to_string(),
        success: true,
        gates_passed: params.gates_passed,
        gates_total: params.gates_total,
        rollout_strategy: params.rollout_strategy.map(String::from),
    };
    eventlog::append_event(params.state_dir, params.target_env, event)
}

/// Log a failed promotion event.
pub fn log_promotion_failure(params: &PromotionParams<'_>) -> Result<(), String> {
    let event = ProvenanceEvent::PromotionCompleted {
        source: params.source.to_string(),
        target: params.target.to_string(),
        success: false,
        gates_passed: params.gates_passed,
        gates_total: params.gates_total,
        rollout_strategy: None,
    };
    eventlog::append_event(params.state_dir, params.target_env, event)
}

/// Log a rollback triggered by health check failure.
pub fn log_rollback(
    state_dir: &Path,
    environment: &str,
    failed_step: usize,
    reason: &str,
) -> Result<(), String> {
    let event = ProvenanceEvent::RollbackTriggered {
        environment: environment.to_string(),
        failed_step,
        reason: reason.to_string(),
    };
    eventlog::append_event(state_dir, environment, event)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_promotion_success() {
        let dir = tempfile::tempdir().unwrap();
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
        let result = log_promotion(&params);
        assert!(result.is_ok());

        // Verify event was written
        let log_path = state_dir.join("staging").join("events.jsonl");
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("promotion_completed"));
        assert!(content.contains("\"success\":true"));
        assert!(content.contains("\"source\":\"dev\""));
        assert!(content.contains("\"target\":\"staging\""));
        assert!(content.contains("\"gates_passed\":3"));
        assert!(content.contains("canary"));
    }

    #[test]
    fn log_promotion_fail() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let params = PromotionParams {
            state_dir: &state_dir,
            target_env: "prod",
            source: "staging",
            target: "prod",
            gates_passed: 1,
            gates_total: 3,
            rollout_strategy: None,
        };
        let result = log_promotion_failure(&params);
        assert!(result.is_ok());

        let log_path = state_dir.join("prod").join("events.jsonl");
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("\"success\":false"));
        assert!(content.contains("\"gates_passed\":1"));
    }

    #[test]
    fn log_rollback_event() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = log_rollback(&state_dir, "prod", 0, "canary health check failed");
        assert!(result.is_ok());

        let log_path = state_dir.join("prod").join("events.jsonl");
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("rollback_triggered"));
        assert!(content.contains("canary health check failed"));
        assert!(content.contains("\"failed_step\":0"));
    }

    #[test]
    fn multiple_events_append() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let p1 = PromotionParams {
            state_dir: &state_dir,
            target_env: "staging",
            source: "dev",
            target: "staging",
            gates_passed: 2,
            gates_total: 2,
            rollout_strategy: None,
        };
        log_promotion(&p1).unwrap();
        let p2 = PromotionParams {
            state_dir: &state_dir,
            target_env: "staging",
            source: "dev",
            target: "staging",
            gates_passed: 3,
            gates_total: 3,
            rollout_strategy: None,
        };
        log_promotion(&p2).unwrap();

        let log_path = state_dir.join("staging").join("events.jsonl");
        let content = std::fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 events appended");
    }

    #[test]
    fn event_serialization_roundtrip() {
        let event = ProvenanceEvent::PromotionCompleted {
            source: "dev".into(),
            target: "staging".into(),
            success: true,
            gates_passed: 4,
            gates_total: 4,
            rollout_strategy: Some("percentage".into()),
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ProvenanceEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
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
    fn rollback_event_serialization() {
        let event = ProvenanceEvent::RollbackTriggered {
            environment: "prod".into(),
            failed_step: 2,
            reason: "timeout".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("rollback_triggered"));
        let deserialized: ProvenanceEvent = serde_json::from_str(&json).unwrap();
        match deserialized {
            ProvenanceEvent::RollbackTriggered {
                environment,
                failed_step,
                reason,
            } => {
                assert_eq!(environment, "prod");
                assert_eq!(failed_step, 2);
                assert_eq!(reason, "timeout");
            }
            _ => panic!("wrong variant"),
        }
    }
}
