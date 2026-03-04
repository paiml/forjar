//! Tests: FJ-1424 cross-machine dependency analysis.

#![allow(unused_imports)]
use super::cross_machine_deps::*;
use super::helpers::*;
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
    fn test_cross_deps_single_machine() {
        let dir = tempfile::tempdir().unwrap();
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
  a:
    type: package
    machine: local
    provider: apt
    packages: [curl]
  b:
    type: file
    machine: local
    path: /tmp/b
    content: "b"
    depends_on: [a]
"#,
        );
        let result = cmd_cross_deps(&p, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cross_deps_multi_machine() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web
    addr: 10.0.0.1
    user: deploy
  db:
    hostname: db
    addr: 10.0.0.2
    user: deploy
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
        let result = cmd_cross_deps(&p, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cross_deps_json() {
        let dir = tempfile::tempdir().unwrap();
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
  a:
    type: package
    machine: local
    provider: apt
    packages: [curl]
"#,
        );
        let result = cmd_cross_deps(&p, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cross_deps_no_deps() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 10.0.0.1
  b:
    hostname: b
    addr: 10.0.0.2
resources:
  pkg-a:
    type: package
    machine: a
    provider: apt
    packages: [curl]
  pkg-b:
    type: package
    machine: b
    provider: apt
    packages: [vim]
"#,
        );
        let result = cmd_cross_deps(&p, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cross_dep_serde() {
        let dep = CrossDep {
            from_resource: "web".to_string(),
            from_machine: "web-1".to_string(),
            to_resource: "db".to_string(),
            to_machine: "db-1".to_string(),
            dep_type: "cross-machine".to_string(),
        };
        let json = serde_json::to_string(&dep).unwrap();
        assert!(json.contains("\"dep_type\":\"cross-machine\""));
    }

    #[test]
    fn test_cross_dep_chain() {
        let dir = tempfile::tempdir().unwrap();
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
  m2:
    hostname: m2
    addr: 10.0.0.2
  m3:
    hostname: m3
    addr: 10.0.0.3
resources:
  base:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  middle:
    type: file
    machine: m2
    path: /tmp/mid
    content: "mid"
    depends_on: [base]
  top:
    type: task
    machine: m3
    command: "echo done"
    depends_on: [middle]
"#,
        );
        let result = cmd_cross_deps(&p, false);
        assert!(result.is_ok());
    }
}
