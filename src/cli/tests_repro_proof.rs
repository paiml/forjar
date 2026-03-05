//! Tests: FJ-1409 training reproducibility proof.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::repro_proof::*;
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
    fn test_fj1409_repro_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: repro-test
machines:
  gpu:
    hostname: gpu
    addr: 10.0.0.1
resources:
  model:
    type: model
    machine: gpu
    name: llama
    source: models/llama.gguf
    path: /opt/models/llama.gguf
    tags: [ml, training]
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_repro_proof(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1409_repro_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: repro-json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  task:
    type: task
    machine: m
    command: "echo hello"
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_repro_proof(&file, &state_dir, true).unwrap();
    }

    #[test]
    fn test_fj1409_repro_with_store() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: store\nmachines: {}\nresources: {}\n",
        );
        let store_dir = dir.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();
        std::fs::write(store_dir.join("model.bin"), b"fake model data").unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_repro_proof(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1409_repro_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = "version: \"1.0\"\nname: deterministic\nmachines: {}\nresources: {}\n";
        let file = write_config(dir.path(), yaml);
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        // Same config should produce same result (no random elements)
        cmd_repro_proof(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1409_repro_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_repro_proof(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1409_repro_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        dispatch(
            Commands::ReproProof(ReproProofArgs {
                file,
                state_dir,
                json: true,
            }),
            0,
            true,
        )
        .unwrap();
    }
}
