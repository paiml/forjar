//! Tests: Core graph commands.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::graph_core::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::test_fixtures::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj131_cmd_graph_mermaid() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_simple_config(dir.path());
        // Should succeed without error
        cmd_graph(&config_path, "mermaid", None, None).unwrap();
    }

    #[test]
    fn test_fj131_cmd_graph_dot() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_simple_config(dir.path());
        cmd_graph(&config_path, "dot", None, None).unwrap();
    }

    #[test]
    fn test_fj131_cmd_graph_unknown_format() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_simple_config(dir.path());
        let err = cmd_graph(&config_path, "svg", None, None).unwrap_err();
        assert!(err.contains("unknown graph format"));
        assert!(err.contains("svg"));
    }

    #[test]
    fn test_fj131_cmd_graph_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "not valid yaml {{{{").unwrap();
        let err = cmd_graph(&config_path, "mermaid", None, None);
        assert!(err.is_err());
    }

    // ── FJ-131: cmd_diff tests ────────────────────────────────────

    #[test]
    fn test_fj132_cmd_graph_mermaid() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [nginx]
  conf:
    type: file
    machine: m
    path: /etc/nginx/nginx.conf
    content: "server {}"
    depends_on: [pkg]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_graph(&file, "mermaid", None, None).unwrap();
    }

    #[test]
    fn test_fj132_cmd_graph_dot() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_graph(&file, "dot", None, None).unwrap();
    }

    #[test]
    fn test_fj132_cmd_graph_unknown_format() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        std::fs::write(&file, yaml).unwrap();
        let result = cmd_graph(&file, "svg", None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown graph format"));
    }

    #[test]
    fn test_fj017_cmd_graph_dot_format() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  conf:
    type: file
    machine: m1
    path: /etc/test.conf
    content: "hello"
    depends_on: [pkg]
"#,
        )
        .unwrap();
        let result = cmd_graph(&config, "dot", None, None);
        assert!(result.is_ok(), "cmd_graph with dot format should succeed");
    }

    #[test]
    fn test_fj294_graph_filter_machine() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: filter-test
machines:
  web:
    hostname: web
    addr: 192.168.1.1
  db:
    hostname: db
    addr: 192.168.1.2
resources:
  web-pkg:
    type: file
    machine: web
    path: /tmp/web.txt
    content: "web"
  db-pkg:
    type: file
    machine: db
    path: /tmp/db.txt
    content: "db"
"#,
        )
        .unwrap();

        // Filter to web machine only
        let result = cmd_graph(&config, "mermaid", Some("web"), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj294_graph_filter_group() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: group-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  web-pkg:
    type: file
    machine: local
    path: /tmp/web.txt
    content: "web"
    resource_group: frontend
  api-pkg:
    type: file
    machine: local
    path: /tmp/api.txt
    content: "api"
    resource_group: backend
"#,
        )
        .unwrap();

        // Filter to frontend group only
        let result = cmd_graph(&config, "dot", None, Some("frontend"));
        assert!(result.is_ok());
    }

    // ── FJ-295: validate --json ──
}
