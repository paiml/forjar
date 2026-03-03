//! Dependency Quality graph commands (Phase 101: FJ-1071/FJ-1074, Phase 107: FJ-1119/FJ-1122).
#![allow(dead_code)]

use crate::core::types;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Build adjacency list from config (resource -> list of dependents).
pub(super) fn build_adjacency(config: &types::ForjarConfig) -> BTreeMap<String, Vec<String>> {
    let mut adj: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in config.resources.keys() {
        adj.entry(name.clone()).or_default();
    }
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if config.resources.contains_key(dep) {
                adj.entry(dep.clone()).or_default().push(name.clone());
            }
        }
    }
    // Sort dependents for deterministic output
    for deps in adj.values_mut() {
        deps.sort();
    }
    adj
}

/// Compute the longest path starting from a given node using DFS with memoization.
pub(super) fn longest_path_from(
    node: &str,
    adj: &BTreeMap<String, Vec<String>>,
    memo: &mut BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    if let Some(cached) = memo.get(node) {
        return cached.clone();
    }
    let mut best: Vec<String> = Vec::new();
    if let Some(neighbors) = adj.get(node) {
        for neighbor in neighbors {
            let sub = longest_path_from(neighbor, adj, memo);
            if sub.len() > best.len() {
                best = sub;
            }
        }
    }
    let mut path = vec![node.to_string()];
    path.extend(best);
    memo.insert(node.to_string(), path.clone());
    path
}

/// Find the critical path (longest dependency chain) in the graph.
pub(super) fn find_critical_path(config: &types::ForjarConfig) -> Vec<String> {
    let adj = build_adjacency(config);
    let mut memo: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut best_path: Vec<String> = Vec::new();
    let mut roots: Vec<&String> = config.resources.keys().collect();
    roots.sort();
    for node in roots {
        let path = longest_path_from(node, &adj, &mut memo);
        if path.len() > best_path.len() {
            best_path = path;
        }
    }
    best_path
}

pub(super) fn print_critical_path_json(path: &[String]) {
    let names: Vec<String> = path.iter().map(|n| format!("\"{}\"", n)).collect();
    println!(
        "{{\"critical_path\":[{}],\"length\":{}}}",
        names.join(","),
        path.len()
    );
}

pub(super) fn print_critical_path_text(path: &[String]) {
    println!("Critical path highlight:");
    if path.is_empty() {
        println!("  (no resources)");
        return;
    }
    println!("  {} (length={})", path.join(" -> "), path.len());
}

/// FJ-1071: Highlight the longest dependency chain in the graph.
pub(crate) fn cmd_graph_resource_dependency_critical_path_highlight(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;
    if cfg.resources.is_empty() {
        if json {
            println!("{{\"critical_path\":[],\"length\":0}}");
        } else {
            println!("Critical path highlight:");
            println!("  (no resources)");
        }
        return Ok(());
    }
    let path = find_critical_path(&cfg);
    if json {
        print_critical_path_json(&path);
    } else {
        print_critical_path_text(&path);
    }
    Ok(())
}

pub(super) struct BottleneckInfo {
    pub(super) name: String,
    pub(super) fan_in: usize,
    pub(super) dependents: Vec<String>,
}

/// Compute fan-in (number of resources that depend on each resource).
pub(super) fn compute_fan_in(config: &types::ForjarConfig) -> Vec<BottleneckInfo> {
    let mut fan_in_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in config.resources.keys() {
        fan_in_map.entry(name.clone()).or_default();
    }
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if config.resources.contains_key(dep) {
                fan_in_map
                    .entry(dep.clone())
                    .or_default()
                    .push(name.clone());
            }
        }
    }
    let mut results: Vec<BottleneckInfo> = fan_in_map
        .into_iter()
        .filter(|(_, deps)| deps.len() >= 2)
        .map(|(name, mut deps)| {
            deps.sort();
            let fan_in = deps.len();
            BottleneckInfo {
                name,
                fan_in,
                dependents: deps,
            }
        })
        .collect();
    results.sort_by(|a, b| b.fan_in.cmp(&a.fan_in).then(a.name.cmp(&b.name)));
    results
}

pub(super) fn print_bottleneck_json(bottlenecks: &[BottleneckInfo]) {
    let items: Vec<String> = bottlenecks
        .iter()
        .map(|b| {
            let deps: Vec<String> = b.dependents.iter().map(|d| format!("\"{}\"", d)).collect();
            format!(
                "{{\"name\":\"{}\",\"fan_in\":{},\"dependents\":[{}]}}",
                b.name,
                b.fan_in,
                deps.join(",")
            )
        })
        .collect();
    println!("{{\"bottlenecks\":[{}]}}", items.join(","));
}

pub(super) fn print_bottleneck_text(bottlenecks: &[BottleneckInfo]) {
    println!("Bottleneck detection:");
    if bottlenecks.is_empty() {
        println!("  (no bottlenecks detected)");
        return;
    }
    for b in bottlenecks {
        println!(
            "  {} (fan-in={}): depended on by {}",
            b.name,
            b.fan_in,
            b.dependents.join(", ")
        );
    }
}

/// FJ-1074: Identify resources with high fan-in that create bottlenecks.
pub(crate) fn cmd_graph_resource_dependency_bottleneck_detection(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;
    if cfg.resources.is_empty() {
        if json {
            println!("{{\"bottlenecks\":[]}}");
        } else {
            println!("Bottleneck detection:");
            println!("  (no bottlenecks detected)");
        }
        return Ok(());
    }
    let bottlenecks = compute_fan_in(&cfg);
    if json {
        print_bottleneck_json(&bottlenecks);
    } else {
        print_bottleneck_text(&bottlenecks);
    }
    Ok(())
}

pub(super) fn print_critical_path_p107_text(path: &[String]) {
    if path.is_empty() {
        println!("Critical path: no dependencies found");
        return;
    }
    println!("Critical path (length {}):", path.len());
    for (i, name) in path.iter().enumerate() {
        println!("  {}. {}", i + 1, name);
    }
}

pub(super) fn print_critical_path_p107_json(path: &[String]) {
    let names: Vec<String> = path.iter().map(|n| format!("\"{}\"", n)).collect();
    println!(
        "{{\"resource_dependency_critical_path\":{{\"length\":{},\"path\":[{}]}}}}",
        path.len(),
        names.join(",")
    );
}

/// FJ-1119: Find the longest dependency chain (critical path) via DFS.
pub(crate) fn cmd_graph_resource_dependency_critical_path(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| format!("read: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&raw).map_err(|e| format!("parse: {e}"))?;
    let path = find_critical_path(&config);
    if json {
        print_critical_path_p107_json(&path);
    } else {
        print_critical_path_p107_text(&path);
    }
    Ok(())
}

/// Build undirected adjacency from config dependencies.
pub(super) fn build_undirected_adj(config: &types::ForjarConfig) -> HashMap<String, Vec<String>> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for name in config.resources.keys() {
        adj.entry(name.clone()).or_default();
    }
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if config.resources.contains_key(dep) {
                adj.entry(name.clone()).or_default().push(dep.clone());
                adj.entry(dep.clone()).or_default().push(name.clone());
            }
        }
    }
    adj
}

pub(super) use super::graph_quality_b::*;
