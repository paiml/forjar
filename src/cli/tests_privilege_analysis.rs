//! Tests: FJ-1403 privilege analysis.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::privilege_analysis::*;
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
    fn test_fj1403_privilege_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: priv-test
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
    path: /etc/nginx/nginx.conf
    content: "server {}"
    depends_on: [pkg]
  user-file:
    type: file
    machine: m1
    path: /home/user/app.conf
    content: "app config"
"#,
        );
        cmd_privilege_analysis(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1403_privilege_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: priv-json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  svc:
    type: service
    machine: m
    name: nginx
  task:
    type: task
    machine: m
    command: "echo hello"
"#,
        );
        cmd_privilege_analysis(&file, None, true).unwrap();
    }

    #[test]
    fn test_fj1403_privilege_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        cmd_privilege_analysis(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1403_privilege_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: filter
machines:
  web:
    hostname: web
    addr: 1.1.1.1
  db:
    hostname: db
    addr: 2.2.2.2
resources:
  web-pkg:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
  db-file:
    type: file
    machine: db
    path: /etc/db.conf
    content: "db"
"#,
        );
        cmd_privilege_analysis(&file, Some("web"), false).unwrap();
    }

    #[test]
    fn test_fj1403_privilege_sudo_flag() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: sudo-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  custom:
    type: task
    machine: m
    command: "reboot"
    sudo: true
"#,
        );
        cmd_privilege_analysis(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1403_privilege_all_unprivileged() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: unpriv
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  task1:
    type: task
    machine: m
    command: "echo hello"
  task2:
    type: task
    machine: m
    command: "echo world"
"#,
        );
        cmd_privilege_analysis(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1403_privilege_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        dispatch(
            Commands::PrivilegeAnalysis(PrivilegeAnalysisArgs {
                file,
                machine: None,
                json: true,
            }),
            0,
            true,
        )
        .unwrap();
    }
}
