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
    source: /models/llama.gguf
    format: gguf
"#,
        );
        cmd_cost_estimate(&file, false).unwrap();
    }

    #[test]
    fn test_fj1415_cost_task_with_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: task-cost
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  deploy:
    type: task
    machine: m
    command: "deploy.sh"
    timeout: 120
"#,
        );
        cmd_cost_estimate(&file, false).unwrap();
    }

    #[test]
    fn test_fj1415_cost_docker_resource() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: docker-cost
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  web:
    type: docker
    machine: m
    name: nginx
    image: "nginx:latest"
"#,
        );
        cmd_cost_estimate(&file, true).unwrap();
    }

    #[test]
    fn test_fj1415_cost_mount_resource() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: mount-cost
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  data-vol:
    type: mount
    machine: m
    path: /mnt/data
    source: /dev/sda1
"#,
        );
        cmd_cost_estimate(&file, false).unwrap();
    }

    #[test]
    fn test_fj1415_cost_mixed_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: full-stack
machines:
  app:
    hostname: app
    addr: 10.0.0.1
  db:
    hostname: db
    addr: 10.0.0.2
resources:
  pkg:
    type: package
    machine: app
    provider: apt
    packages: [curl]
  conf:
    type: file
    machine: app
    path: /etc/app.conf
  svc:
    type: service
    machine: app
    name: myapp
  backup:
    type: task
    machine: db
    command: "pg_dump"
"#,
        );
        cmd_cost_estimate(&file, true).unwrap();
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
            0,
            true,
        )
        .unwrap();
    }
}
