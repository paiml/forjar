//! Tests: FJ-1407 model card generation.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::model_card::*;
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
    fn test_fj1407_model_card_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: ml-stack
machines:
  gpu:
    hostname: gpu
    addr: 10.0.0.1
resources:
  llama-model:
    type: model
    machine: gpu
    name: llama-7b
    source: models/llama-7b.gguf
    path: /opt/models/llama-7b.gguf
    tags: [ml, llm]
    resource_group: models
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_model_card(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1407_model_card_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: ml-json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  bert:
    type: model
    machine: m
    name: bert
    source: models/bert.bin
    path: /opt/models/bert.bin
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_model_card(&file, &state_dir, true).unwrap();
    }

    #[test]
    fn test_fj1407_model_card_no_models() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: no-models
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_model_card(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1407_model_card_tagged() {
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
  train-task:
    type: task
    machine: m
    command: "python train.py"
    tags: [ml, training, model]
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_model_card(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1407_model_card_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        dispatch(
            Commands::ModelCard(ModelCardArgs {
                file,
                state_dir,
                json: false,
            }),
            false,
            true,
        )
        .unwrap();
    }
}
