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

pub(crate) fn cmd_agent_sbom(
    file: &Path,
    state_dir: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let components = collect_agent_components(&config, state_dir);

    if json {
        print_agent_sbom_json(&components, &config.name);
    } else {
        print_agent_sbom_text(&components, &config.name);
    }

    Ok(())
}

fn collect_agent_components(
    config: &types::ForjarConfig,
    _state_dir: &Path,
) -> Vec<AgentComponent> {
    let mut components = Vec::new();

    for (id, resource) in &config.resources {
        let machine = match &resource.machine {
            types::MachineTarget::Single(m) => m.clone(),
            types::MachineTarget::Multiple(ms) => ms.join(","),
        };

        match resource.resource_type {
            types::ResourceType::Model => {
                components.push(AgentComponent {
                    name: id.clone(),
                    component_type: "model".to_string(),
                    version: resource.version.clone().unwrap_or_else(|| "latest".to_string()),
                    machine: machine.clone(),
                });
            }
            types::ResourceType::Gpu => {
                let backend = resource.gpu_backend.clone().unwrap_or_else(|| "nvidia".to_string());
                components.push(AgentComponent {
                    name: id.clone(),
                    component_type: "gpu-runtime".to_string(),
                    version: backend,
                    machine: machine.clone(),
                });
            }
            types::ResourceType::Service => {
                if is_agent_service(id, resource) {
                    components.push(AgentComponent {
                        name: id.clone(),
                        component_type: "agent-service".to_string(),
                        version: resource.version.clone().unwrap_or_else(|| "unknown".to_string()),
                        machine: machine.clone(),
                    });
                }
            }
            types::ResourceType::Docker => {
                if is_agent_container(id, resource) {
                    let img = resource.image.clone().unwrap_or_else(|| "unknown".to_string());
                    components.push(AgentComponent {
                        name: id.clone(),
                        component_type: "agent-container".to_string(),
                        version: img,
                        machine: machine.clone(),
                    });
                }
            }
            _ => {}
        }

        // Check for MCP-related tags
        if resource.tags.iter().any(|t| t.contains("mcp") || t.contains("pforge")) {
            components.push(AgentComponent {
                name: format!("{id}-mcp"),
                component_type: "mcp-tool".to_string(),
                version: "registered".to_string(),
                machine,
            });
        }
    }

    components.sort_by(|a, b| a.component_type.cmp(&b.component_type).then(a.name.cmp(&b.name)));
    components
}

fn is_agent_service(id: &str, resource: &types::Resource) -> bool {
    let keywords = ["mcp", "agent", "pforge", "inference", "llm"];
    keywords.iter().any(|k| id.contains(k))
        || resource.tags.iter().any(|t| keywords.iter().any(|k| t.contains(k)))
}

fn is_agent_container(id: &str, resource: &types::Resource) -> bool {
    let keywords = ["mcp", "agent", "inference", "llm", "model"];
    keywords.iter().any(|k| id.contains(k))
        || resource.image.as_ref().map_or(false, |img| {
            keywords.iter().any(|k| img.contains(k))
        })
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
