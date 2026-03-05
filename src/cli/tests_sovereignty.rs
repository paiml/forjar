//! Tests: FJ-1414 data sovereignty tagging.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::sovereignty::*;
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
    fn test_fj1414_sovereignty_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_sovereignty(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1414_sovereignty_tagged() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: tagged
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  db:
    type: file
    machine: m
    path: /etc/db.conf
    tags: [jurisdiction:EU, classification:PII, residency:eu-west-1]
  web:
    type: service
    machine: m
    name: nginx
    tags: [jurisdiction:US]
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_sovereignty(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1414_sovereignty_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: json\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_sovereignty(&file, &state_dir, true).unwrap();
    }

    #[test]
    fn test_fj1414_sovereignty_with_state() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: state\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(state_dir.join("m.lock.yaml"), "resources: {}\n").unwrap();
        cmd_sovereignty(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1414_sovereignty_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        dispatch(
            Commands::Sovereignty(SovereigntyArgs {
                file,
                state_dir,
                json: false,
            }),
            0,
            true,
        )
        .unwrap();
    }
}
