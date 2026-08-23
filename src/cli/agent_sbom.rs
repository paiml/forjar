//! FJ-1408: Agent SBOM generation.
//!
//! Extends standard SBOM with agent-specific components: MCP servers,
//! model resources, GPU configurations, tool registrations.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

struct AgentComponent {
    name: String,
    component_type: String,
    version: String,
    machine: String,
}

pub(crate) fn cmd_agent_sbom(file: &Path, state_dir: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let components = collect_agent_components(&config, state_dir);

    if json {
        print_agent_sbom_json(&components, &config.name);
    } else {
        print_agent_sbom_text(&components, &config.name);
    }

    Ok(())
}

/// The machine label an SBOM component carries: the single target, or every
/// target joined by commas.
fn agent_machine_label(target: &types::MachineTarget) -> String {
    match target {
        types::MachineTarget::Single(m) => m.clone(),
        types::MachineTarget::Multiple(ms) => ms.join(","),
    }
}

fn agent_component(
    name: String,
    component_type: &str,
    version: String,
    machine: String,
) -> AgentComponent {
    AgentComponent {
        name,
        component_type: component_type.to_string(),
        version,
        machine,
    }
}

fn model_component(id: &str, resource: &types::Resource, machine: &str) -> AgentComponent {
    let version = resource
        .version
        .clone()
        .unwrap_or_else(|| "latest".to_string());
    agent_component(id.to_string(), "model", version, machine.to_string())
}

fn gpu_component(id: &str, resource: &types::Resource, machine: &str) -> AgentComponent {
    let backend = resource
        .gpu_backend
        .clone()
        .unwrap_or_else(|| "nvidia".to_string());
    agent_component(id.to_string(), "gpu-runtime", backend, machine.to_string())
}

/// A service contributes only when it looks like an agent.
fn service_component(
    id: &str,
    resource: &types::Resource,
    machine: &str,
) -> Option<AgentComponent> {
    if !is_agent_service(id, resource) {
        return None;
    }
    let version = resource
        .version
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    Some(agent_component(
        id.to_string(),
        "agent-service",
        version,
        machine.to_string(),
    ))
}

/// A container contributes only when it looks like an agent.
fn container_component(
    id: &str,
    resource: &types::Resource,
    machine: &str,
) -> Option<AgentComponent> {
    if !is_agent_container(id, resource) {
        return None;
    }
    let img = resource
        .image
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    Some(agent_component(
        id.to_string(),
        "agent-container",
        img,
        machine.to_string(),
    ))
}

/// The SBOM component a resource contributes by virtue of its type, if any.
fn typed_agent_component(
    id: &str,
    resource: &types::Resource,
    machine: &str,
) -> Option<AgentComponent> {
    match resource.resource_type {
        types::ResourceType::Model => Some(model_component(id, resource, machine)),
        types::ResourceType::Gpu => Some(gpu_component(id, resource, machine)),
        types::ResourceType::Service => service_component(id, resource, machine),
        types::ResourceType::Docker => container_component(id, resource, machine),
        _ => None,
    }
}

/// The `mcp-tool` component a resource contributes when its tags mark it as
/// part of the MCP surface.
fn mcp_agent_component(
    id: &str,
    resource: &types::Resource,
    machine: String,
) -> Option<AgentComponent> {
    if resource
        .tags
        .iter()
        .any(|t| t.contains("mcp") || t.contains("pforge"))
    {
        Some(agent_component(
            format!("{id}-mcp"),
            "mcp-tool",
            "registered".to_string(),
            machine,
        ))
    } else {
        None
    }
}

fn collect_agent_components(config: &types::ForjarConfig, state_dir: &Path) -> Vec<AgentComponent> {
    // GH-91: state_dir not yet used for SBOM state inspection
    let _ = state_dir;
    let mut components = Vec::new();

    for (id, resource) in &config.resources {
        let machine = agent_machine_label(&resource.machine);

        if let Some(component) = typed_agent_component(id, resource, &machine) {
            components.push(component);
        }

        // Check for MCP-related tags
        if let Some(component) = mcp_agent_component(id, resource, machine) {
            components.push(component);
        }
    }

    components.sort_by(|a, b| {
        a.component_type
            .cmp(&b.component_type)
            .then(a.name.cmp(&b.name))
    });
    components
}

fn is_agent_service(id: &str, resource: &types::Resource) -> bool {
    let keywords = ["mcp", "agent", "pforge", "inference", "llm"];
    keywords.iter().any(|k| id.contains(k))
        || resource
            .tags
            .iter()
            .any(|t| keywords.iter().any(|k| t.contains(k)))
}

fn is_agent_container(id: &str, resource: &types::Resource) -> bool {
    let keywords = ["mcp", "agent", "inference", "llm", "model"];
    keywords.iter().any(|k| id.contains(k))
        || resource
            .image
            .as_ref()
            .is_some_and(|img| keywords.iter().any(|k| img.contains(k)))
}

fn print_agent_sbom_json(components: &[AgentComponent], name: &str) {
    let items: Vec<String> = components
        .iter()
        .map(|c| {
            format!(
                r#"{{"name":"{}","type":"{}","version":"{}","machine":"{}"}}"#,
                c.name, c.component_type, c.version, c.machine
            )
        })
        .collect();

    println!(
        r#"{{"stack":"{}","agent_components":[{}],"total":{}}}"#,
        name,
        items.join(","),
        components.len()
    );
}

fn print_agent_sbom_text(components: &[AgentComponent], name: &str) {
    println!("{}\n", bold("Agent SBOM"));
    println!("  Stack: {}", bold(name));
    println!("  Components: {}\n", components.len());

    if components.is_empty() {
        println!("  (no agent components detected)");
        return;
    }

    let mut current_type = "";
    for c in components {
        if c.component_type != current_type {
            current_type = &c.component_type;
            println!("  {}:", bold(current_type));
        }
        println!(
            "    {} {} ({}, {})",
            green("*"),
            c.name,
            dim(&c.version),
            dim(&c.machine)
        );
    }
}
