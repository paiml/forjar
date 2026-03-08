//! Tests: FJ-1420 fault injection testing.

#![allow(unused_imports)]
use super::fault_inject::*;
use super::helpers::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
        let p = dir.join("forjar.yaml");
        std::fs::write(&p, yaml).unwrap();
        p
    }

    #[test]
    fn test_fault_inject_basic() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
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
"#,
        );
        let result = cmd_fault_inject(&p, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fault_inject_json() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /etc/test.conf
    content: "hello"
"#,
        );
        let result = cmd_fault_inject(&p, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fault_inject_remote() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  remote:
    hostname: srv
    addr: 10.0.0.1
    user: deploy
resources:
  svc:
    type: service
    machine: remote
    name: nginx
    command: "nginx -g 'daemon off;'"
"#,
        );
        let result = cmd_fault_inject(&p, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fault_inject_with_deps() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [nginx]
  svc:
    type: service
    machine: local
    name: nginx
    command: "nginx"
    depends_on: [pkg]
"#,
        );
        let result = cmd_fault_inject(&p, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fault_inject_resource_filter() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: local
    path: /tmp/a
    content: "a"
  b:
    type: file
    machine: local
    path: /tmp/b
    content: "b"
"#,
        );
        let result = cmd_fault_inject(&p, Some("a"), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fault_inject_timeout_scenario() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  deploy:
    type: task
    machine: local
    command: "deploy.sh"
    timeout: 60
    completion_check: "test -f /tmp/deployed"
"#,
        );
        let result = cmd_fault_inject(&p, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fault_inject_artifacts_scenario() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  build:
    type: file
    machine: local
    path: /tmp/output
    content: "data"
    output_artifacts:
      - app.bin
"#,
        );
        let result = cmd_fault_inject(&p, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fault_inject_task_no_idempotency() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  run:
    type: task
    machine: local
    command: "echo hello"
"#,
        );
        // Task without completion_check or content fails idempotency check
        let result = cmd_fault_inject(&p, None, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_fault_inject_sudo_permission() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  system:
    type: file
    machine: local
    path: /usr/local/bin/app
    content: "binary"
    sudo: true
"#,
        );
        let result = cmd_fault_inject(&p, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fault_scenario_serde() {
        let s = FaultScenario {
            name: "test".to_string(),
            category: "transport".to_string(),
            target_resource: "pkg".to_string(),
            description: "test desc".to_string(),
            expected_behavior: "should fail".to_string(),
            passed: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"passed\":true"));
    }
}
