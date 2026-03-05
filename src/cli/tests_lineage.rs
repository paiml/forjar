//! Tests: FJ-1405 Merkle DAG configuration lineage.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::lineage::*;
use super::test_fixtures::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
        let file = dir.join("forjar.yaml");
        std::fs::write(&file, yaml).unwrap();
        file
    }

    #[test]
    fn test_fj1405_lineage_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: lineage-test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  base:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  conf:
    type: file
    machine: m1
    path: /etc/test.conf
    content: "hello"
    depends_on: [base]
"#,
        );
        cmd_lineage(&file, false).unwrap();
    }

    #[test]
    fn test_fj1405_lineage_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: lineage-json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  task:
    type: task
    machine: m
    command: "echo ok"
"#,
        );
        cmd_lineage(&file, true).unwrap();
    }

    #[test]
    fn test_fj1405_lineage_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        cmd_lineage(&file, false).unwrap();
    }

    #[test]
    fn test_fj1405_lineage_deep_chain() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: deep-chain
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: task
    machine: m
    command: "echo a"
  b:
    type: task
    machine: m
    command: "echo b"
    depends_on: [a]
  c:
    type: task
    machine: m
    command: "echo c"
    depends_on: [b]
  d:
    type: task
    machine: m
    command: "echo d"
    depends_on: [c]
"#,
        );
        cmd_lineage(&file, false).unwrap();
    }

    #[test]
    fn test_fj1405_lineage_diamond() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: diamond
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  root:
    type: task
    machine: m
    command: "echo root"
  left:
    type: task
    machine: m
    command: "echo left"
    depends_on: [root]
  right:
    type: task
    machine: m
    command: "echo right"
    depends_on: [root]
  bottom:
    type: task
    machine: m
    command: "echo bottom"
    depends_on: [left, right]
"#,
        );
        cmd_lineage(&file, true).unwrap();
    }

    #[test]
    fn test_fj1405_lineage_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        dispatch(
            Commands::Lineage(LineageArgs {
                file,
                json: true,
            }),
            0,
            true,
        )
        .unwrap();
    }
}
