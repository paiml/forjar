//! Tests: FJ-1431 live infrastructure query.

#![allow(unused_imports)]
use super::infra_query_live::*;
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
    fn test_live_query_basic() {
        let dir = tempfile::tempdir().unwrap();
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
"#);
        let result = cmd_query_live(&p, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_live_query_filtered() {
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
        let result = cmd_query_live(&p, Some("a"), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_live_status_display() {
        assert_eq!(format!("{}", LiveStatus::Running), "RUNNING");
        assert_eq!(format!("{}", LiveStatus::Stopped), "STOPPED");
        assert_eq!(format!("{}", LiveStatus::Unreachable), "UNREACHABLE");
    }

    #[test]
    fn test_live_query_report_serde() {
        let report = LiveQueryReport {
            query: "*".to_string(),
            results: vec![],
            total: 0,
            running: 0,
            stopped: 0,
            unreachable: 0,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"total\":0"));
    }
}
