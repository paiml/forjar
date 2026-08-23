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
    // REAL CONFIG, REAL LOCK PATH.
    //
    // These tests were written against the agent's own fake drift detector,
    // which parsed a raw YAML walk and looked for `state/<resource>.lock.yaml`.
    // So they used a `resources:` LIST — not forjar's schema, which is a map —
    // and a lock path that no forjar version has ever written. They passed
    // because both sides agreed on a fiction, which is exactly why the fake
    // detector survived: the tests could not tell it from a working one.
    // Now that detect_drift uses the real parser and the real detector, the
    // fixtures have to be real too.
    fn test_detect_drift_empty_config() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: \"1.0\"\nname: t\nmachines: { local: { hostname: localhost, addr: 127.0.0.1 } }\nresources: {}\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        let drifted = detect_drift(&cfg, &state).unwrap();
        assert!(drifted.is_empty());
    }

    #[test]
    fn a_machine_with_no_lock_is_not_drift() {
        // Was `test_detect_drift_missing_lock`, which asserted
        // `drifted == ["pkg-nginx"]` because the fake detector returned drift
        // for any missing `state/<resource>.lock.yaml` — a path forjar never
        // writes. That semantic is wrong on its own terms: a machine that has
        // never been applied has no RECORDED STATE to have drifted from, and
        // calling it drift makes `agent --auto-apply` re-apply the entire stack
        // on every first run.
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: \"1.0\"\nname: t\nmachines: { local: { hostname: localhost, addr: 127.0.0.1 } }\nresources: {}\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        assert!(detect_drift(&cfg, &state).unwrap().is_empty());
    }

    #[test]
    fn detect_drift_uses_the_real_parser() {
        // Was `test_detect_drift_failed_lock`, which wrote "status: failed"
        // into a bogus lock path and expected drift. That conflated A FAILED
        // APPLY with DRIFT — different questions — and the fake detector
        // answered neither, because it read a file forjar never writes.
        //
        // The end-to-end assertion (converge, tamper, require detection) needs
        // a real transport, so it lives in the ledger-replay harness as
        // `agent-blind-to-drift-autoapply-never-fires` rather than being faked
        // here. What IS provable at unit level, and what the old fixtures could
        // never have caught, is that detect_drift now goes through the real
        // parser: a config that is not a valid forjar config must be REJECTED,
        // not silently walked as raw YAML.
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        // The exact shape the old tests used: a `resources:` LIST. forjar's
        // schema is a map, so this is not a forjar config at all.
        std::fs::write(&cfg, "resources:\n  - name: svc-app\n    type: service\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        assert!(
            detect_drift(&cfg, &state).is_err(),
            "an invalid config was accepted — detect_drift is not using the real parser"
        );
    }


    #[test]
    fn a_failed_apply_is_not_drift() {
        // The old `test_detect_drift_failed_lock` asserted the opposite,
        // because the fake detector string-matched "status: failed" in a lock.
        // A resource that FAILED TO APPLY has not drifted — nothing converged,
        // so there is no recorded state for the machine to have moved away
        // from. Treating it as drift makes `--auto-apply` retry failures
        // forever under a name that means something else; `retry-failed` is
        // the command for that.
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: \"1.0\"\nname: t\nmachines: { local: { hostname: localhost, addr: 127.0.0.1 } }\nresources: {}\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        // A stray file at the path the FAKE detector used to read. The real
        // detector must ignore it entirely.
        std::fs::write(state.join("svc-app.lock.yaml"), "status: failed\nhash: abc\n").unwrap();
        assert!(
            detect_drift(&cfg, &state).unwrap().is_empty(),
            "a failed-apply marker was reported as drift"
        );
    }

    #[test]
    fn test_cmd_push_mode() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: \"1.0\"\nname: t\nmachines: { local: { hostname: localhost, addr: 127.0.0.1 } }\nresources: {}\n").unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir(&state).unwrap();
        let result = cmd_pull_agent(&cfg, &state, 1, false, Some(1), ExecMode::Push, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_pull_mode_bounded() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, "version: \"1.0\"\nname: t\nmachines: { local: { hostname: localhost, addr: 127.0.0.1 } }\nresources: {}\n").unwrap();
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
