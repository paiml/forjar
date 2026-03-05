//! Tests: FJ-1412 training checkpoint management.

#![allow(unused_imports)]
use super::checkpoint::*;
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
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
    fn test_fj1412_checkpoint_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        cmd_checkpoint(&file, None, false, 5, false).unwrap();
    }

    #[test]
    fn test_fj1412_checkpoint_with_model() {
        let dir = tempfile::tempdir().unwrap();
        // Create output artifact
        std::fs::write(dir.path().join("checkpoint-1.pt"), b"model weights").unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: training
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  llm:
    type: model
    machine: gpu
    name: llama
    source: /models/llama.gguf
    format: gguf
    output_artifacts:
      - checkpoint-1.pt
"#,
        );
        cmd_checkpoint(&file, None, false, 5, false).unwrap();
    }

    #[test]
    fn test_fj1412_checkpoint_gc() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ckpt-1.pt"), b"old").unwrap();
        std::fs::write(dir.path().join("ckpt-2.pt"), b"newer").unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: gc
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  model:
    type: model
    machine: gpu
    name: llama
    source: /models/llama.gguf
    format: gguf
    output_artifacts:
      - ckpt-1.pt
      - ckpt-2.pt
"#,
        );
        cmd_checkpoint(&file, None, true, 1, false).unwrap();
    }

    #[test]
    fn test_fj1412_checkpoint_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: json\nmachines: {}\nresources: {}\n",
        );
        cmd_checkpoint(&file, None, false, 5, true).unwrap();
    }

    #[test]
    fn test_fj1412_checkpoint_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        dispatch(
            Commands::Checkpoint(CheckpointArgs {
                file,
                machine: None,
                gc: false,
                keep: 5,
                json: false,
            }),
            0,
            true,
        )
        .unwrap();
    }
}
