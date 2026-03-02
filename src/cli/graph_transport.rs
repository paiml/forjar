//! Phase 96 — Transport Diagnostics & Recipe Governance: graph commands.

use crate::core::types;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

type GroupMap = BTreeMap<String, Vec<String>>;

fn classify_resources(config: &types::ForjarConfig) -> (GroupMap, GroupMap) {
    let mut recipe_groups: GroupMap = BTreeMap::new();
    let mut type_groups: GroupMap = BTreeMap::new();
    for (name, resource) in &config.resources {
        let key = resource.recipe.as_ref().or(resource.source.as_ref());
        if let Some(origin) = key {
            recipe_groups
                .entry(origin.clone())
                .or_default()
                .push(name.clone());
        } else {
            type_groups
                .entry(resource.resource_type.to_string())
                .or_default()
                .push(name.clone());
        }
    }
    for v in recipe_groups.values_mut() {
        v.sort();
    }
    for v in type_groups.values_mut() {
        v.sort();
    }
    (recipe_groups, type_groups)
}

fn print_expansion_map_json(recipe_groups: &GroupMap, type_groups: &GroupMap) {
    let mut entries: Vec<String> = Vec::new();
    for (key, members) in recipe_groups {
        let names: Vec<String> = members.iter().map(|n| format!("\"{}\"", n)).collect();
        entries.push(format!(
            "\"{}\":{{\"origin\":\"recipe\",\"resources\":[{}]}}",
            key,
            names.join(",")
        ));
    }
    for (key, members) in type_groups {
        let names: Vec<String> = members.iter().map(|n| format!("\"{}\"", n)).collect();
        entries.push(format!(
            "\"{}\":{{\"origin\":\"type\",\"resources\":[{}]}}",
            key,
            names.join(",")
        ));
    }
    println!("{{\"recipe_expansion_map\":{{{}}}}}", entries.join(","));
}

fn print_expansion_map_text(recipe_groups: &GroupMap, type_groups: &GroupMap) {
    if !recipe_groups.is_empty() {
        println!("Recipe-originated resources:");
        for (key, members) in recipe_groups {
            println!("  {} -> {}", key, members.join(", "));
        }
    }
    if !type_groups.is_empty() {
        println!("Resources grouped by type (no recipe origin):");
        for (key, members) in type_groups {
            println!("  {} -> {}", key, members.join(", "));
        }
    }
}

/// FJ-1031: Map resources to their recipe expansion origins.
pub(crate) fn cmd_graph_resource_recipe_expansion_map(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"recipe_expansion_map\":{{}}}}");
        } else {
            println!("No resources to map.");
        }
        return Ok(());
    }
    let (recipe_groups, type_groups) = classify_resources(&config);
    if json {
        print_expansion_map_json(&recipe_groups, &type_groups);
    } else {
        print_expansion_map_text(&recipe_groups, &type_groups);
    }
    Ok(())
}

type AdjMap = HashMap<String, Vec<String>>;

fn build_forward_adjacency(config: &types::ForjarConfig) -> (AdjMap, Vec<String>) {
    let mut children_map: AdjMap = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for (name, _) in &config.resources {
        children_map.entry(name.clone()).or_default();
        in_degree.entry(name.clone()).or_insert(0);
    }
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            children_map
                .entry(dep.clone())
                .or_default()
                .push(name.clone());
            *in_degree.entry(name.clone()).or_default() += 1;
        }
    }
    for children in children_map.values_mut() {
        children.sort();
    }
    let mut roots: Vec<String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    roots.sort();
    (children_map, roots)
}

fn dfs_longest(
    node: &str,
    children_map: &AdjMap,
    memo: &mut HashMap<String, Vec<String>>,
    visiting: &mut HashSet<String>,
) -> Vec<String> {
    if let Some(cached) = memo.get(node) {
        return cached.clone();
    }
    if visiting.contains(node) {
        return vec![node.to_string()];
    }
    visiting.insert(node.to_string());
    let children = children_map.get(node).cloned().unwrap_or_default();
    let mut best: Vec<String> = Vec::new();
    for child in &children {
        let p = dfs_longest(child, children_map, memo, visiting);
        if p.len() > best.len() {
            best = p;
        }
    }
    let mut path = vec![node.to_string()];
    path.extend(best);
    memo.insert(node.to_string(), path.clone());
    visiting.remove(node);
    path
}

fn find_longest_path(children_map: &AdjMap, roots: &[String], all_names: &[String]) -> Vec<String> {
    let mut memo: HashMap<String, Vec<String>> = HashMap::new();
    let mut longest: Vec<String> = Vec::new();
    for root in roots {
        let mut visiting = HashSet::new();
        let path = dfs_longest(root, children_map, &mut memo, &mut visiting);
        if path.len() > longest.len() {
            longest = path;
        }
    }
    if longest.is_empty() {
        for name in all_names {
            let mut visiting = HashSet::new();
            let path = dfs_longest(name, children_map, &mut memo, &mut visiting);
            if path.len() > longest.len() {
                longest = path;
            }
        }
    }
    longest
}

/// FJ-1034: Compute critical chain path with resource weights.
pub(crate) fn cmd_graph_resource_dependency_critical_chain_path(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"critical_chain_path\":{{\"length\":0,\"path\":[]}}}}");
        } else {
            println!("No resources to analyze.");
        }
        return Ok(());
    }
    let (children_map, roots) = build_forward_adjacency(&config);
    let all_names: Vec<String> = config.resources.keys().cloned().collect();
    let longest = find_longest_path(&children_map, &roots, &all_names);
    let length = longest.len();
    if json {
        let items: Vec<String> = longest.iter().map(|n| format!("\"{}\"", n)).collect();
        println!(
            "{{\"critical_chain_path\":{{\"length\":{},\"path\":[{}]}}}}",
            length,
            items.join(",")
        );
    } else {
        println!("Critical chain path ({} nodes):", length);
        println!("  {}", longest.join(" -> "));
    }
    Ok(())
}
