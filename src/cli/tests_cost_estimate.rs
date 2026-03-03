//! Tests: FJ-1415 cost estimation.

#![allow(unused_imports)]
use super::commands::*;
use super::cost_estimate::*;
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
    fn test_fj1415_cost_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        cmd_cost_estimate(&file, false).unwrap();
    }

    #[test]
    fn test_fj1415_cost_mixed() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: mixed
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl, wget]
  conf:
    type: file
    machine: m
    path: /etc/app.conf
    content: "key=value"
  web:
    type: service
    machine: m
    name: nginx
"#,
        );
        cmd_cost_estimate(&file, false).unwrap();
    }

    #[test]
    fn test_fj1415_cost_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: json\nmachines: {}\nresources: {}\n",
        );
        cmd_cost_estimate(&file, true).unwrap();
    }

    #[test]
    fn test_fj1415_cost_gpu() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: gpu
machines:
  gpu:
    hostname: gpu
    addr: 127.0.0.1
resources:
  nvidia:
    type: gpu
    machine: gpu
    driver_version: "550.0"
  llm:
    type: model
    machine: gpu
    name: llama
    model_source: local
    model_path: /models/llama.gguf
"#,
        );
        cmd_cost_estimate(&file, false).unwrap();
    }

    #[test]
    fn test_fj1415_cost_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        dispatch(
            Commands::CostEstimate(CostEstimateArgs {
                file,
                json: false,
            }),
            false,
            true,
        )
        .unwrap();
    }
}
