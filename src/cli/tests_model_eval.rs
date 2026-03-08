//! Tests: FJ-1416 model evaluation pipeline.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::model_eval::*;
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
    fn test_fj1416_eval_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        cmd_model_eval(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1416_eval_with_model() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: eval
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
"#,
        );
        cmd_model_eval(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1416_eval_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: json\nmachines: {}\nresources: {}\n",
        );
        cmd_model_eval(&file, None, true).unwrap();
    }

    #[test]
    fn test_fj1416_eval_with_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        // Create output artifacts
        std::fs::write(dir.path().join("metrics.json"), r#"{"accuracy": 0.95}"#).unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: eval-arts
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  eval-run:
    type: task
    machine: gpu
    command: "python eval.py"
    tags: [eval, ml]
    output_artifacts:
      - metrics.json
"#,
        );
        cmd_model_eval(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1416_eval_with_filter() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: filter
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  eval-a:
    type: model
    machine: gpu
    name: model-a
  eval-b:
    type: model
    machine: gpu
    name: model-b
"#,
        );
        cmd_model_eval(&file, Some("eval-a"), false).unwrap();
    }

    #[test]
    fn test_fj1416_eval_json_with_model() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: json-model
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  model:
    type: model
    machine: gpu
    name: bert
    command: "python eval.py"
    completion_check: "test -f results.json"
"#,
        );
        cmd_model_eval(&file, None, true).unwrap();
    }

    #[test]
    fn test_fj1416_eval_missing_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: missing
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  eval:
    type: task
    machine: gpu
    command: "python eval.py"
    tags: [eval]
    output_artifacts:
      - nonexistent_results.json
"#,
        );
        let result = cmd_model_eval(&file, None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj1416_eval_benchmark_tag() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: benchmark
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  bench:
    type: task
    machine: m
    command: "cargo bench"
    tags: [benchmark]
"#,
        );
        cmd_model_eval(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1416_eval_resource_group() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: eval-group
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  quality-gate:
    type: task
    machine: m
    command: "pytest"
    resource_group: evaluation
"#,
        );
        cmd_model_eval(&file, None, true).unwrap();
    }

    #[test]
    fn test_fj1416_eval_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        dispatch(
            Commands::ModelEval(ModelEvalArgs {
                file,
                resource: None,
                json: false,
            }),
            0,
            true,
        )
        .unwrap();
    }
}
