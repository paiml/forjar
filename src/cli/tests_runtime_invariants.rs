//! Tests: FJ-1421 runtime invariant monitors.

#![allow(unused_imports)]
use super::helpers::*;
use super::runtime_invariants::*;
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
    fn test_invariants_basic() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
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
    tags: [infra]
"#,
        );
        let _ = cmd_invariants(&p, &state, false);
    }

    #[test]
    fn test_invariants_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
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
    tags: [config]
"#,
        );
        let _ = cmd_invariants(&p, &state, true);
    }

    #[test]
    fn test_invariants_policy_require_tags() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
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
policies:
  - type: require
    message: "all resources must be tagged"
    field: tags
"#,
        );
        let result = cmd_invariants(&p, &state, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_invariants_deny_condition() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
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
  task:
    type: task
    machine: local
    command: "evil-cmd"
    tags: [bad]
policies:
  - type: deny
    message: "no evil commands"
    condition_field: command
    condition_value: "evil-cmd"
"#,
        );
        let result = cmd_invariants(&p, &state, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_invariants_service_has_name() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
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
  web:
    type: service
    machine: local
    name: nginx
    command: "nginx"
    tags: [web]
"#,
        );
        let _ = cmd_invariants(&p, &state, true);
    }

    #[test]
    fn test_invariant_status_display() {
        assert_eq!(format!("{}", InvariantStatus::Satisfied), "SATISFIED");
        assert_eq!(format!("{}", InvariantStatus::Violated), "VIOLATED");
        assert_eq!(format!("{}", InvariantStatus::Unknown), "UNKNOWN");
    }
}
