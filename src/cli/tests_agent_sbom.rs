//! Tests: FJ-1408 agent SBOM generation.

#![allow(unused_imports)]
use super::agent_sbom::*;
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
    fn test_fj1408_agent_sbom_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: agent-stack
machines:
  gpu:
    hostname: gpu
    addr: 10.0.0.1
resources:
  llama:
    type: model
    machine: gpu
    name: llama
    source: models/llama.gguf
    path: /opt/models/llama.gguf
  gpu-runtime:
    type: gpu
    machine: gpu
    driver_version: "550.0"
  mcp-server:
    type: service
    machine: gpu
    name: mcp-server
    tags: [mcp, pforge]
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_agent_sbom(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1408_agent_sbom_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: agent-json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  inference-svc:
    type: service
    machine: m
    name: inference-server
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_agent_sbom(&file, &state_dir, true).unwrap();
    }

    #[test]
    fn test_fj1408_agent_sbom_no_agents() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: no-agents
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [nginx]
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_agent_sbom(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1408_agent_sbom_docker_agent() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: docker-agent
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  llm-container:
    type: docker
    machine: m
    name: llm-inference
    image: ghcr.io/company/llm-inference:latest
"#,
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_agent_sbom(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1408_agent_sbom_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        cmd_agent_sbom(&file, &state_dir, false).unwrap();
    }

    #[test]
    fn test_fj1408_agent_sbom_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        dispatch(
            Commands::AgentSbom(AgentSbomArgs {
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
