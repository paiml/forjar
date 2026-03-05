//! Tests: FJ-1451 dependency impact analysis.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::impact_analysis::*;
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
    fn test_impact_no_deps() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#,
        );
        cmd_impact(&file, "a", false).unwrap();
    }

    #[test]
    fn test_impact_chain() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: package
    machine: m
    provider: apt
    packages: [curl]
  b:
    type: file
    machine: m
    path: /tmp/b
    content: "b"
    depends_on: [a]
  c:
    type: task
    machine: m
    command: "echo done"
    depends_on: [b]
"#,
        );
        cmd_impact(&file, "a", false).unwrap();
    }

    #[test]
    fn test_impact_diamond() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  base:
    type: package
    machine: m
    provider: apt
    packages: [curl]
  left:
    type: file
    machine: m
    path: /tmp/left
    content: "l"
    depends_on: [base]
  right:
    type: file
    machine: m
    path: /tmp/right
    content: "r"
    depends_on: [base]
  top:
    type: task
    machine: m
    command: "echo done"
    depends_on: [left, right]
"#,
        );
        cmd_impact(&file, "base", false).unwrap();
    }

    #[test]
    fn test_impact_cross_machine() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web
    addr: 10.0.0.1
  db:
    hostname: db
    addr: 10.0.0.2
resources:
  db-pkg:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
  web-cfg:
    type: file
    machine: web
    path: /etc/app/db.conf
    content: "host=10.0.0.2"
    depends_on: [db-pkg]
"#,
        );
        cmd_impact(&file, "db-pkg", false).unwrap();
    }

    #[test]
    fn test_impact_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: package
    machine: m
    provider: apt
    packages: [curl]
  b:
    type: file
    machine: m
    path: /tmp/b
    content: "b"
    depends_on: [a]
"#,
        );
        cmd_impact(&file, "a", true).unwrap();
    }

    #[test]
    fn test_impact_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: test\nmachines: {}\nresources: {}\n",
        );
        let result = cmd_impact(&file, "missing", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_impact_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#,
        );
        dispatch(
            Commands::Impact(ImpactArgs {
                file,
                resource: "a".to_string(),
                json: false,
            }),
            0,
            true,
        )
        .unwrap();
    }
}
