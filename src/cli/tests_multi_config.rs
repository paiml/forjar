//! Tests: FJ-1428 multi-config apply ordering.

#![allow(unused_imports)]
use super::multi_config::*;
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
    fn test_multi_config_single() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_config(
            dir.path(),
            "network",
            r#"
version: "1.0"
name: network
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
        let result = cmd_multi_config(&[f1], true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multi_config_two_independent() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_config(
            dir.path(),
            "network",
            r#"
version: "1.0"
name: network
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
            "compute",
            r#"
version: "1.0"
name: compute
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: local
    path: /etc/app.conf
    content: "app"
"#,
        );
        let result = cmd_multi_config(&[f1, f2], true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_multi_config_plan_serde() {
        let plan = MultiConfigPlan {
            configs: vec![ConfigNode {
                name: "test".to_string(),
                path: "/tmp/test.yaml".to_string(),
                resources: 2,
                machines: vec!["local".to_string()],
                depends_on: vec![],
            }],
            execution_order: vec![vec!["test".to_string()]],
            total_configs: 1,
            total_resources: 2,
        };
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("\"total_configs\":1"));
    }

    #[test]
    fn test_build_stack_deps() {
        let configs = vec![
            ConfigNode {
                name: "net".to_string(),
                path: "net.yaml".to_string(),
                resources: 1,
                machines: vec![],
                depends_on: vec![],
            },
            ConfigNode {
                name: "app".to_string(),
                path: "app.yaml".to_string(),
                resources: 1,
                machines: vec![],
                depends_on: vec!["net".to_string()],
            },
        ];
        let deps = build_stack_deps(&configs);
        assert!(deps["net"].is_empty());
        assert_eq!(deps["app"], vec!["net".to_string()]);
    }
}
