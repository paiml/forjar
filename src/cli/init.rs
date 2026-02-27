//! Init, format, completion, schema.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::commands::*;


pub(crate) fn cmd_init(path: &Path) -> Result<(), String> {
    let config_path = path.join("forjar.yaml");
    if config_path.exists() {
        return Err(format!("{} already exists", config_path.display()));
    }

    let state_dir = path.join("state");
    std::fs::create_dir_all(&state_dir).map_err(|e| format!("cannot create state dir: {}", e))?;

    let template = r#"version: "1.0"
name: my-infrastructure
description: "Managed by forjar"

params:
  env: development

machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
  # remote-server:
  #   hostname: my-server
  #   addr: 10.0.0.1
  #   user: root
  #   ssh_key: ~/.ssh/id_ed25519

resources:
  # Example: install packages
  base-packages:
    type: package
    machine: localhost
    provider: apt
    packages: [curl, git, htop]

  # Example: manage a config file
  # app-config:
  #   type: file
  #   machine: localhost
  #   path: /etc/myapp/config.yaml
  #   content: |
  #     environment: {{params.env}}
  #     log_level: info
  #   owner: root
  #   mode: "0644"
  #   depends_on: [base-packages]

  # Example: manage a service
  # app-service:
  #   type: service
  #   machine: localhost
  #   name: myapp
  #   state: running
  #   enabled: true
  #   restart_on: [app-config]
  #   depends_on: [app-config]

policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#;
    std::fs::write(&config_path, template)
        .map_err(|e| format!("cannot write {}: {}", config_path.display(), e))?;

    println!("Initialized forjar project at {}", path.display());
    println!("  Created: {}", config_path.display());
    println!("  Created: {}/", state_dir.display());
    Ok(())
}


pub(crate) fn cmd_fmt(file: &Path, check: bool) -> Result<(), String> {
    let original = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {}", file.display(), e))?;

    // Parse into ForjarConfig to validate + normalize
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&original).map_err(|e| format!("YAML parse error: {}", e))?;

    // Re-serialize to canonical YAML
    let formatted =
        serde_yaml_ng::to_string(&config).map_err(|e| format!("YAML serialize error: {}", e))?;

    if check {
        if original.trim() != formatted.trim() {
            println!("{} is not formatted", file.display());
            return Err("file is not formatted".to_string());
        }
        println!("{} is formatted", file.display());
        return Ok(());
    }

    if original.trim() == formatted.trim() {
        println!("{} already formatted", file.display());
        return Ok(());
    }

    std::fs::write(file, &formatted)
        .map_err(|e| format!("cannot write {}: {}", file.display(), e))?;
    println!("Formatted {}", file.display());
    Ok(())
}


// FJ-253: forjar completion — shell completion generation
pub(crate) fn cmd_completion(shell: CompletionShell) -> Result<(), String> {
    use clap::CommandFactory;
    use clap_complete::{generate, Shell};

    // Build a top-level CLI command that mirrors main.rs Cli struct
    #[derive(clap::Parser)]
    #[command(name = "forjar")]
    struct CliForCompletion {
        #[command(subcommand)]
        command: Commands,
    }

    let clap_shell = match shell {
        CompletionShell::Bash => Shell::Bash,
        CompletionShell::Zsh => Shell::Zsh,
        CompletionShell::Fish => Shell::Fish,
    };

    let mut cmd = CliForCompletion::command();
    generate(clap_shell, &mut cmd, "forjar", &mut std::io::stdout());
    Ok(())
}


pub(crate) fn cmd_schema() -> Result<(), String> {
    let machine_schema = serde_json::json!({
        "type": "object",
        "required": ["hostname", "addr"],
        "properties": {
            "hostname": { "type": "string" },
            "addr": { "type": "string", "description": "IP, DNS, or 'container'" },
            "user": { "type": "string", "default": "root" },
            "arch": { "type": "string", "default": "x86_64" },
            "ssh_key": { "type": "string" },
            "roles": { "type": "array", "items": { "type": "string" } },
            "transport": { "type": "string", "enum": ["container"] },
            "cost": { "type": "integer", "default": 0 }
        }
    });

    let resource_schema = serde_json::json!({
        "type": "object",
        "required": ["type", "machine"],
        "properties": {
            "type": { "type": "string", "enum": [
                "package", "file", "service", "mount", "user",
                "docker", "cron", "network", "pepita", "model", "gpu"
            ]},
            "machine": { "type": "string" },
            "state": { "type": "string" },
            "depends_on": { "type": "array", "items": { "type": "string" } },
            "triggers": { "type": "array", "items": { "type": "string" } },
            "tags": { "type": "array", "items": { "type": "string" } },
            "when": { "type": "string" },
            "arch": { "type": "array", "items": { "type": "string" } },
            "provider": { "type": "string", "enum": ["apt", "cargo", "uv"] },
            "packages": { "type": "array", "items": { "type": "string" } },
            "path": { "type": "string" },
            "content": { "type": "string" },
            "source": { "type": "string" },
            "owner": { "type": "string" },
            "group": { "type": "string" },
            "mode": { "type": "string" },
            "name": { "type": "string" },
            "enabled": { "type": "boolean" },
            "schedule": { "type": "string" },
            "command": { "type": "string" },
            "image": { "type": "string" },
            "ports": { "type": "array", "items": { "type": "string" } },
            "volumes": { "type": "array", "items": { "type": "string" } }
        }
    });

    let policy_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "failure": { "type": "string", "enum": ["stop_on_first", "continue_independent"] },
            "parallel_machines": { "type": "boolean", "default": false },
            "parallel_resources": { "type": "boolean", "default": false },
            "tripwire": { "type": "boolean", "default": true },
            "lock_file": { "type": "boolean", "default": true },
            "ssh_retries": { "type": "integer", "default": 1, "minimum": 1, "maximum": 4 },
            "serial": { "type": "integer", "minimum": 1 },
            "max_fail_percentage": { "type": "integer", "minimum": 0, "maximum": 100 },
            "pre_apply": { "type": "string" },
            "post_apply": { "type": "string" }
        }
    });

    let schema = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Forjar Configuration",
        "description": "Schema for forjar.yaml — Rust-native Infrastructure as Code",
        "type": "object",
        "required": ["version", "name", "resources"],
        "properties": {
            "version": { "type": "string", "const": "1.0" },
            "name": { "type": "string" },
            "description": { "type": "string" },
            "params": { "type": "object", "additionalProperties": true },
            "includes": { "type": "array", "items": { "type": "string" } },
            "machines": { "type": "object", "additionalProperties": machine_schema },
            "resources": { "type": "object", "additionalProperties": resource_schema },
            "policy": policy_schema,
            "outputs": { "type": "object", "additionalProperties": {
                "type": "object",
                "properties": {
                    "value": { "type": "string" },
                    "description": { "type": "string" },
                    "sensitive": { "type": "boolean", "default": false }
                }
            }},
            "data": { "type": "object", "additionalProperties": {
                "type": "object",
                "required": ["type", "value"],
                "properties": {
                    "type": { "type": "string", "enum": ["file", "command", "dns"] },
                    "value": { "type": "string" },
                    "default": { "type": "string" }
                }
            }},
            "policies": { "type": "array", "items": {
                "type": "object",
                "required": ["name", "type", "condition"],
                "properties": {
                    "name": { "type": "string" },
                    "type": { "type": "string", "enum": ["deny", "warn", "require"] },
                    "condition": { "type": "string" },
                    "message": { "type": "string" }
                }
            }}
        }
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&schema).map_err(|e| format!("JSON error: {}", e))?
    );
    Ok(())
}

