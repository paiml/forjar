//! Tests: FJ-1429 stack dependency graph.

#![allow(unused_imports)]
use super::stack_dep_graph::*;
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
    fn test_stack_graph_single() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_config(
            dir.path(),
            "app",
            r#"
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
"#,
        );
        let result = cmd_stack_graph(&[f1], true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stack_graph_independent() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_config(
            dir.path(),
            "net",
            r#"
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
"#,
        );
        let f2 = write_config(
            dir.path(),
            "storage",
            r#"
version: "1.0"
name: storage
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  mnt:
    type: file
    machine: local
    path: /etc/fstab.d/data
    content: "data"
"#,
        );
        let result = cmd_stack_graph(&[f1, f2], true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stack_node_serde() {
        let node = StackNode {
            name: "net".to_string(),
            path: "net.yaml".to_string(),
            resources: 3,
            dependencies: vec![],
            dependents: vec!["app".to_string()],
        };
        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"name\":\"net\""));
    }

    #[test]
    fn test_stack_graph_report_serde() {
        let report = StackGraphReport {
            nodes: vec![],
            total_stacks: 0,
            total_resources: 0,
            has_cycles: false,
            parallel_groups: vec![],
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"has_cycles\":false"));
    }

    #[test]
    fn test_stack_graph_two_stacks() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_config(
            dir.path(),
            "base",
            r#"
version: "1.0"
name: base
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
        let f2 = write_config(
            dir.path(),
            "overlay",
            r#"
version: "1.0"
name: overlay
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /etc/overlay.conf
    content: "overlay"
"#,
        );
        let result = cmd_stack_graph(&[f1, f2], false);
        assert!(result.is_ok());
    }
}
