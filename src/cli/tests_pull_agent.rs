//! Tests: FJ-059+060 pull agent + hybrid push/pull.

#![allow(unused_imports)]
use super::pull_agent::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_mode_display() {
        assert_eq!(format!("{}", ExecMode::Push), "push");
        assert_eq!(format!("{}", ExecMode::Pull), "pull");
    }

    #[test]
    fn test_exec_mode_serde() {
        let push = ExecMode::Push;
        let json = serde_json::to_string(&push).unwrap();
        let round: ExecMode = serde_json::from_str(&json).unwrap();
        assert_eq!(round, ExecMode::Push);
    }

    #[test]
    fn test_pull_agent_config_serde() {
        let cfg = PullAgentConfig {
            config_file: "forjar.yaml".into(),
            state_dir: "state".into(),
            interval_secs: 30,
            auto_apply: false,
            max_iterations: Some(5),
            mode: ExecMode::Pull,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let round: PullAgentConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(round.interval_secs, 30);
        assert_eq!(round.mode, ExecMode::Pull);
    }

    #[test]
    fn test_detect_drift_missing_config() {
        let result = detect_drift(Path::new("/nonexistent/forjar.yaml"), Path::new("state"));
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_drift_empty_config() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "resources: []\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        let drifted = detect_drift(&cfg, &state).unwrap();
        assert!(drifted.is_empty());
    }

    #[test]
    fn test_detect_drift_missing_lock() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "resources:\n  - name: pkg-nginx\n    type: package\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        let drifted = detect_drift(&cfg, &state).unwrap();
        assert_eq!(drifted, vec!["pkg-nginx"]);
    }

    #[test]
    fn test_detect_drift_clean_lock() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "resources:\n  - name: pkg-nginx\n    type: package\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        std::fs::write(state.join("pkg-nginx.lock.yaml"), "status: converged\nhash: abc123\n")
            .unwrap();
        let drifted = detect_drift(&cfg, &state).unwrap();
        assert!(drifted.is_empty());
    }

    #[test]
    fn test_detect_drift_failed_lock() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "resources:\n  - name: svc-app\n    type: service\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        std::fs::write(state.join("svc-app.lock.yaml"), "status: failed\nhash: abc\n").unwrap();
        let drifted = detect_drift(&cfg, &state).unwrap();
        assert_eq!(drifted, vec!["svc-app"]);
    }

    #[test]
    fn test_cmd_push_mode() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "resources: []\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        let result = cmd_pull_agent(&cfg, &state, 1, false, Some(1), ExecMode::Push, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_pull_mode_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "resources: []\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        // Pull mode with 2 iterations, 0-second interval for test speed
        let result = cmd_pull_agent(&cfg, &state, 0, false, Some(2), ExecMode::Pull, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reconcile_result_serde() {
        let r = ReconcileResult {
            iteration: 0,
            timestamp: "now".to_string(),
            drift_detected: true,
            resources_drifted: 3,
            auto_applied: false,
            mode: ExecMode::Push,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"drift_detected\":true"));
    }

    #[test]
    fn test_agent_report_serde() {
        let report = AgentReport {
            mode: ExecMode::Pull,
            config_file: "test.yaml".to_string(),
            interval_secs: 60,
            iterations_completed: 1,
            total_drift_events: 0,
            auto_applies: 0,
            results: vec![],
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"mode\":\"Pull\""));
    }
}
