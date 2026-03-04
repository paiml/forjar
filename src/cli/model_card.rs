//! FJ-1407: Model card generation.
//!
//! Generates a model card (markdown or JSON) from config + state,
//! documenting model resources with metadata, training info, and lineage.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

pub(crate) fn cmd_model_card(
    file: &Path,
    state_dir: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let models = collect_model_resources(&config);

    if models.is_empty() {
        println!("No model resources found in config");
        return Ok(());
    }

    if json {
        print_model_cards_json(&models, &config.name, state_dir);
    } else {
        print_model_cards_text(&models, &config.name, state_dir);
    }

    Ok(())
}

struct ModelInfo {
    id: String,
    resource_type: String,
    machine: String,
    tags: Vec<String>,
    group: Option<String>,
    deps: Vec<String>,
    has_store: bool,
    has_output: bool,
}

fn collect_model_resources(config: &types::ForjarConfig) -> Vec<ModelInfo> {
    let mut models = Vec::new();
    for (id, resource) in &config.resources {
        let is_model = matches!(resource.resource_type, types::ResourceType::Model)
            || resource.tags.iter().any(|t| t.contains("model") || t.contains("ml"))
            || resource.resource_group.as_deref() == Some("models");

        if !is_model {
            continue;
        }

        let machine = match &resource.machine {
            types::MachineTarget::Single(m) => m.clone(),
            types::MachineTarget::Multiple(ms) => ms.join(", "),
        };

        models.push(ModelInfo {
            id: id.clone(),
            resource_type: format!("{:?}", resource.resource_type).to_lowercase(),
            machine,
            tags: resource.tags.clone(),
            group: resource.resource_group.clone(),
            deps: resource.depends_on.clone(),
            has_store: resource.store,
            has_output: !resource.output_artifacts.is_empty(),
        });
    }
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models
}

fn get_state_hash(state_dir: &Path) -> Option<String> {
    let global = state_dir.join("forjar.lock.yaml");
    if global.exists() {
        if let Ok(bytes) = std::fs::read(&global) {
            return Some(blake3::hash(&bytes).to_hex()[..16].to_string());
        }
    }
    None
}

fn print_model_cards_json(models: &[ModelInfo], name: &str, state_dir: &Path) {
    let state_hash = get_state_hash(state_dir).unwrap_or_default();
    let items: Vec<String> = models
        .iter()
        .map(|m| {
            let tags: Vec<String> = m.tags.iter().map(|t| format!("\"{t}\"")).collect();
            let deps: Vec<String> = m.deps.iter().map(|d| format!("\"{d}\"")).collect();
            format!(
                r#"{{"id":"{}","type":"{}","machine":"{}","tags":[{}],"group":{},"dependencies":[{}],"store":{},"outputs":{}}}"#,
                m.id,
                m.resource_type,
                m.machine,
                tags.join(","),
                m.group.as_ref().map(|g| format!("\"{g}\"")).unwrap_or_else(|| "null".to_string()),
                deps.join(","),
                m.has_store,
                m.has_output,
            )
        })
        .collect();

    println!(
        r#"{{"stack":"{}","state_hash":"{}","models":[{}],"total":{}}}"#,
        name,
        state_hash,
        items.join(","),
        models.len()
    );
}

fn print_model_cards_text(models: &[ModelInfo], name: &str, state_dir: &Path) {
    let state_hash = get_state_hash(state_dir);

    println!("{}\n", bold("Model Card"));
    println!("  Stack: {}", bold(name));
    if let Some(ref h) = state_hash {
        println!("  State: blake3:{h}");
    }
    println!("  Models: {}\n", models.len());

    for model in models {
        println!("  {} {}", green("*"), bold(&model.id));
        println!("    Type:    {}", model.resource_type);
        println!("    Machine: {}", model.machine);
        if !model.tags.is_empty() {
            println!("    Tags:    {}", model.tags.join(", "));
        }
        if let Some(ref g) = model.group {
            println!("    Group:   {g}");
        }
        if !model.deps.is_empty() {
            println!("    Deps:    {}", model.deps.join(", "));
        }
        if model.has_store {
            println!("    Store:   {}", green("yes"));
        }
        if model.has_output {
            println!("    Outputs: {}", green("yes"));
        }
        println!();
    }
}
