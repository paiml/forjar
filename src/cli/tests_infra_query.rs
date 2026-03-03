//! Tests: FJ-1430 infrastructure query.

#![allow(unused_imports)]
use super::infra_query::*;
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
    fn test_query_all() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let p = write_config(dir.path(), r#"
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
"#);
        let filter = QueryFilter { pattern: None, resource_type: None, machine: None, tag: None };
        let result = cmd_query(&p, &state, &filter, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_by_type() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let p = write_config(dir.path(), r#"
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
  cfg:
    type: file
    machine: local
    path: /tmp/test
    content: "hello"
"#);
        let filter = QueryFilter { pattern: None, resource_type: Some("package".to_string()), machine: None, tag: None };
        let result = cmd_query(&p, &state, &filter, true, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_by_tag() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let p = write_config(dir.path(), r#"
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
    tags: [web]
"#);
        let filter = QueryFilter { pattern: None, resource_type: None, machine: None, tag: Some("web".to_string()) };
        let result = cmd_query(&p, &state, &filter, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_match_serde() {
        let m = QueryMatch {
            resource_id: "pkg".to_string(),
            resource_type: "Package".to_string(),
            machine: vec!["local".to_string()],
            tags: vec!["web".to_string()],
            status: "pending".to_string(),
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"status\":\"pending\""));
    }
}
