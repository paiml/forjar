//! Tests: FJ-1436 saga coordinator.

#![allow(unused_imports)]
use super::saga_coordinator::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(dir: &Path, name: &str, yaml: &str) -> std::path::PathBuf {
        let p = dir.join(format!("{name}.yaml"));
        std::fs::write(&p, yaml).unwrap();
        p
    }

    #[test]
    fn test_saga_plan_single() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let f1 = write_config(dir.path(), "net", r#"
version: "1.0"
name: net
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  fw:
    type: file
    machine: local
    path: /etc/fw.conf
    content: "allow"
"#);
        let result = cmd_saga_plan(&[f1], &state, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_saga_plan_multi() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let f1 = write_config(dir.path(), "net", r#"
version: "1.0"
name: net
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  fw:
    type: file
    machine: local
    path: /etc/fw.conf
    content: "allow"
"#);
        let f2 = write_config(dir.path(), "app", r#"
version: "1.0"
name: app
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [curl]
"#);
        let result = cmd_saga_plan(&[f1, f2], &state, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_saga_step_status_display() {
        assert_eq!(format!("{}", SagaStepStatus::Pending), "PENDING");
        assert_eq!(format!("{}", SagaStepStatus::Applied), "APPLIED");
        assert_eq!(format!("{}", SagaStepStatus::Failed), "FAILED");
        assert_eq!(format!("{}", SagaStepStatus::Compensated), "COMPENSATED");
    }

    #[test]
    fn test_create_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let stack_state = state.join("net");
        std::fs::create_dir_all(&stack_state).unwrap();
        std::fs::write(stack_state.join("state.lock.yaml"), "resources: {}\n").unwrap();
        let snap = create_snapshot(&state, "net").unwrap();
        assert!(!snap.is_empty());
    }

    #[test]
    fn test_saga_report_serde() {
        let report = SagaReport {
            steps: vec![SagaStep {
                stack_name: "net".to_string(),
                config_path: "net.yaml".to_string(),
                snapshot_path: Some("/tmp/snap".to_string()),
                status: SagaStepStatus::Pending,
                error: None,
            }],
            total: 1,
            applied: 0,
            failed: 0,
            compensated: 0,
            success: true,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"success\":true"));
    }
}
