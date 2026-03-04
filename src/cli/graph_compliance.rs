//! Phase 98 — Compliance Automation & Drift Intelligence: graph commands.

use crate::core::types;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

// ============================================================================
// FJ-1047: Resource dependency risk score
// ============================================================================

fn type_weight(rt: &types::ResourceType) -> usize {
    match rt {
        types::ResourceType::Service => 3,
        types::ResourceType::Package => 2,
        types::ResourceType::File => 1,
        _ => 1,
    }
}

fn compute_fan_in(config: &types::ForjarConfig) -> HashMap<String, usize> {
    let mut fan_in: HashMap<String, usize> = HashMap::new();
    for resource in config.resources.values() {
        for dep in &resource.depends_on {
            *fan_in.entry(dep.clone()).or_default() += 1;
        }
    }
    fan_in
}

fn compute_depths(config: &types::ForjarConfig) -> HashMap<String, usize> {
    let mut depths: HashMap<String, usize> = HashMap::new();
    let mut visited: HashMap<String, bool> = HashMap::new();
    for name in config.resources.keys() {
        compute_depth_recursive(name, config, &mut depths, &mut visited);
    }
    depths
}

fn compute_depth_recursive(
    name: &str,
    config: &types::ForjarConfig,
    depths: &mut HashMap<String, usize>,
    visited: &mut HashMap<String, bool>,
) -> usize {
    if let Some(&d) = depths.get(name) {
        return d;
    }
    if visited.get(name) == Some(&true) {
        return 0; // cycle guard
    }
    visited.insert(name.to_string(), true);
    let resource = match config.resources.get(name) {
        Some(r) => r,
        None => {
            depths.insert(name.to_string(), 0);
            return 0;
        }
    };
    let max_dep = resource
        .depends_on
        .iter()
        .map(|dep| compute_depth_recursive(dep, config, depths, visited))
        .max()
        .unwrap_or(0);
    let depth = max_dep + 1;
    depths.insert(name.to_string(), depth);
    depth
}

fn compute_risk_scores(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let fan_in = compute_fan_in(config);
    let depths = compute_depths(config);
    let mut scores: Vec<(String, usize)> = config
        .resources
        .iter()
        .map(|(name, resource)| {
            let weight = type_weight(&resource.resource_type);
            let depth = depths.get(name).copied().unwrap_or(0);
            let fi = fan_in.get(name).copied().unwrap_or(0);
            let score = weight + depth + fi;
            (name.clone(), score)
        })
        .collect();
    scores.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    scores
}

fn print_risk_scores_json(scores: &[(String, usize)]) {
    let items: Vec<String> = scores
        .iter()
        .map(|(r, s)| format!("{{\"resource\":\"{r}\",\"risk_score\":{s}}}"))
        .collect();
    println!(
        "{{\"resource_dependency_risk_scores\":[{}]}}",
        items.join(",")
    );
}

fn print_risk_scores_text(scores: &[(String, usize)]) {
    if scores.is_empty() {
        println!("No resources to score.");
        return;
    }
    println!("Resource dependency risk scores (type_weight + depth + fan_in):");
    for (r, s) in scores {
        println!("  {r} — score {s}");
    }
}

/// FJ-1047: Compute risk score per resource based on type weight, dependency depth, and fan-in.
pub(crate) fn cmd_graph_resource_dependency_risk_score(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"resource_dependency_risk_scores\":[]}}");
        } else {
            println!("No resources to score.");
        }
        return Ok(());
    }
    let scores = compute_risk_scores(&config);
    if json {
        print_risk_scores_json(&scores);
    } else {
        print_risk_scores_text(&scores);
    }
    Ok(())
}

// ============================================================================
// FJ-1050: Resource dependency layering
// ============================================================================

fn classify_layer(rt: &types::ResourceType) -> &'static str {
    match rt {
        types::ResourceType::Package | types::ResourceType::Mount => "infra",
        types::ResourceType::Service | types::ResourceType::Cron => "app",
        types::ResourceType::File => "config",
        _ => "other",
    }
}

fn build_layer_map(config: &types::ForjarConfig) -> BTreeMap<&'static str, Vec<String>> {
    let mut layers: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for (name, resource) in &config.resources {
        let layer = classify_layer(&resource.resource_type);
        layers.entry(layer).or_default().push(name.clone());
    }
    for members in layers.values_mut() {
        members.sort();
    }
    layers
}

fn print_layering_json(layers: &BTreeMap<&'static str, Vec<String>>) {
    let entries: Vec<String> = layers
        .iter()
        .map(|(layer, members)| {
            let names: Vec<String> = members.iter().map(|n| format!("\"{n}\"")).collect();
            format!("\"{}\":[{}]", layer, names.join(","))
        })
        .collect();
    println!(
        "{{\"resource_dependency_layering\":{{{}}}}}",
        entries.join(",")
    );
}

fn print_layering_text(layers: &BTreeMap<&'static str, Vec<String>>) {
    if layers.is_empty() {
        println!("No resources to layer.");
        return;
    }
    println!("Resource dependency layering:");
    for (layer, members) in layers {
        println!("  [{}] {}", layer, members.join(", "));
    }
}

/// FJ-1050: Assign resources to semantic layers (infra, app, config, other) based on type.
pub(crate) fn cmd_graph_resource_dependency_layering(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"resource_dependency_layering\":{{}}}}");
        } else {
            println!("No resources to layer.");
        }
        return Ok(());
    }
    let layers = build_layer_map(&config);
    if json {
        print_layering_json(&layers);
    } else {
        print_layering_text(&layers);
    }
    Ok(())
}
