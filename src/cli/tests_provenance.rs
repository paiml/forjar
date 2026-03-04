//! Tests: FJ-1404 SLSA provenance attestation.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::provenance::*;
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
    fn test_fj1404_provenance_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: prov-test
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
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_provenance(&file, &state_dir, None, false).unwrap();
    }

    #[test]
    fn test_fj1404_provenance_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: prov-json
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
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_provenance(&file, &state_dir, None, true).unwrap();
    }

    #[test]
    fn test_fj1404_provenance_with_state_files() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: state-test\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        // Write a fake lock file
        std::fs::write(
            state_dir.join("m1.lock.yaml"),
            "resources: {}\nlast_apply: \"2024-01-01T00:00:00Z\"\n",
        )
        .unwrap();
        cmd_provenance(&file, &state_dir, None, false).unwrap();
    }

    #[test]
    fn test_fj1404_provenance_machine_filter() {
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
    path: /tmp/db.txt
    content: "db"
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_provenance(&file, &state_dir, Some("web"), false).unwrap();
    }

    #[test]
    fn test_fj1404_provenance_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_provenance(&file, &state_dir, None, false).unwrap();
    }

    #[test]
    fn test_fj1404_provenance_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        dispatch(
            Commands::Provenance(ProvenanceArgs {
                file,
                state_dir,
                machine: None,
                json: true,
            }),
            false,
            true,
        )
        .unwrap();
    }
}
