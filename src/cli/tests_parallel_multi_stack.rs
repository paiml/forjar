//! Tests: FJ-1435 parallel multi-stack apply.

#![allow(unused_imports)]
use super::parallel_multi_stack::*;
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
    fn test_parallel_single_stack() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_config(dir.path(), "app", r#"
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
        let result = cmd_parallel_stacks(&[f1], 4, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parallel_independent_stacks() {
        let dir = tempfile::tempdir().unwrap();
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
        let f2 = write_config(dir.path(), "storage", r#"
version: "1.0"
name: storage
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  data:
    type: file
    machine: local
    path: /etc/data.conf
    content: "data"
"#);
        let result = cmd_parallel_stacks(&[f1, f2], 4, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parallel_plan_serde() {
        let plan = ParallelPlan {
            stacks: vec![StackInfo {
                name: "app".to_string(),
                path: "app.yaml".to_string(),
                resources: 1,
                dependencies: vec![],
            }],
            waves: vec![Wave {
                index: 0,
                stacks: vec!["app".to_string()],
                parallel: false,
            }],
            total_stacks: 1,
            max_parallelism: 1,
        };
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("\"max_parallelism\":1"));
    }

    #[test]
    fn test_wave_serde() {
        let wave = Wave {
            index: 0,
            stacks: vec!["a".to_string(), "b".to_string()],
            parallel: true,
        };
        let json = serde_json::to_string(&wave).unwrap();
        assert!(json.contains("\"parallel\":true"));
    }
}
