//! Tests: FJ-1434 preservation checking.

#![allow(unused_imports)]
use super::preservation_check::*;
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
    fn test_preservation_no_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), r#"
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
"#);
        let result = cmd_preservation(&p, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_preservation_path_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), r#"
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
    path: /tmp/same
    content: "version-a"
  b:
    type: file
    machine: local
    path: /tmp/same
    content: "version-b"
"#);
        let result = cmd_preservation(&p, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_preservation_single_resource() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  only:
    type: package
    machine: local
    provider: apt
    packages: [curl]
"#);
        let result = cmd_preservation(&p, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_preservation_pair_serde() {
        let pair = PreservationPair {
            resource_a: "a".to_string(),
            resource_b: "b".to_string(),
            preserved: true,
            reason: "ok".to_string(),
        };
        let json = serde_json::to_string(&pair).unwrap();
        assert!(json.contains("\"preserved\":true"));
    }

    #[test]
    fn test_preservation_package_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(dir.path(), r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  a:
    type: package
    machine: local
    provider: apt
    packages: [curl, wget]
  b:
    type: package
    machine: local
    provider: apt
    packages: [curl, vim]
"#);
        let result = cmd_preservation(&p, true);
        assert!(result.is_err());
    }
}
