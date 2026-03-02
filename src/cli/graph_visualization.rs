//! Graph visualization.

use super::helpers::*;
use crate::core::{resolver, types};
use std::path::Path;

/// Print pruned graph in DOT format.
fn print_prune_dot(
    remaining: &[&String],
    config: &types::ForjarConfig,
    pruned: &std::collections::HashSet<String>,
) {
    println!("digraph {{");
    println!("  rankdir=LR;");
    for name in remaining {
        println!("  \"{}\";", name);
        if let Some(res) = config.resources.get(*name) {
            for dep in &res.depends_on {
                if !pruned.contains(dep) {
                    println!("  \"{}\" -> \"{}\";", name, dep);
                }
            }
        }
    }
    println!("}}");
}

/// Print pruned graph in Mermaid format.
fn print_prune_mermaid(
    remaining: &[&String],
    config: &types::ForjarConfig,
    pruned: &std::collections::HashSet<String>,
) {
    println!("graph LR");
    for name in remaining {
        if let Some(res) = config.resources.get(*name) {
            for dep in &res.depends_on {
                if !pruned.contains(dep) {
                    println!("  {} --> {}", name, dep);
                }
            }
        }
    }
}

// ── FJ-454: graph --prune ──

pub(crate) fn cmd_graph_prune(file: &Path, format: &str, resource: &str) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;

    // Collect subtree of resource to prune (resource + all transitive dependents)
    let mut pruned = std::collections::HashSet::new();
    pruned.insert(resource.to_string());

    // Find all resources that transitively depend on the pruned resource
    let mut changed = true;
    while changed {
        changed = false;
        for name in &order {
            if pruned.contains(name) {
                continue;
            }
            if let Some(res) = config.resources.get(name) {
                for dep in &res.depends_on {
                    if pruned.contains(dep) {
                        pruned.insert(name.clone());
                        changed = true;
                        break;
                    }
                }
            }
        }
    }

    let remaining: Vec<&String> = order.iter().filter(|n| !pruned.contains(*n)).collect();

    if format == "dot" {
        print_prune_dot(&remaining, &config, &pruned);
    } else {
        print_prune_mermaid(&remaining, &config, &pruned);
    }

    println!(
        "\n{} Pruned {} and {} dependent(s)",
        dim("──"),
        resource,
        pruned.len() - 1
    );
    Ok(())
}

// ── FJ-464: graph --layers ──

pub(crate) fn cmd_graph_layers(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;

    // Compute layer (depth) for each resource
    let mut layers: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    // Resources with no deps are layer 0
    for name in &order {
        if let Some(res) = config.resources.get(name) {
            if res.depends_on.is_empty() {
                layers.insert(name.clone(), 0);
            }
        }
    }

    // Iteratively assign layers
    let mut changed = true;
    while changed {
        changed = false;
        for name in &order {
            if layers.contains_key(name) {
                continue;
            }
            if let Some(res) = config.resources.get(name) {
                let max_dep = res
                    .depends_on
                    .iter()
                    .filter_map(|d| layers.get(d))
                    .max()
                    .copied();
                if let Some(max) = max_dep {
                    layers.insert(name.clone(), max + 1);
                    changed = true;
                }
            }
        }
    }

    // Group by layer
    let max_layer = layers.values().copied().max().unwrap_or(0);
    for layer in 0..=max_layer {
        let resources: Vec<&String> = order
            .iter()
            .filter(|n| layers.get(*n) == Some(&layer))
            .collect();
        if !resources.is_empty() {
            println!(
                "Layer {} ({}): {}",
                layer,
                resources.len(),
                resources
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    Ok(())
}

// ── FJ-474: graph --critical-resources ──

pub(crate) fn cmd_graph_critical_resources(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    // Count how many resources depend on each resource (direct + transitive)
    let mut dependent_count: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (name, res) in &config.resources {
        dependent_count.entry(name.clone()).or_insert(0);
        for dep in &res.depends_on {
            *dependent_count.entry(dep.clone()).or_insert(0) += 1;
        }
    }
    let mut ranked: Vec<(String, usize)> = dependent_count.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("Critical Resources (most dependents first)");
    println!("{}", "─".repeat(50));
    for (name, count) in &ranked {
        if *count == 0 {
            continue;
        }
        println!("  {:30} {} dependent(s)", name, count);
    }
    if ranked.iter().all(|(_, c)| *c == 0) {
        println!("  (no resources have dependents)");
    }
    Ok(())
}

/// Print weighted graph in DOT format.
fn print_weight_dot(
    config: &types::ForjarConfig,
    weights: &std::collections::HashMap<String, usize>,
) {
    println!("digraph forjar {{");
    println!("  rankdir=LR;");
    for (name, res) in &config.resources {
        let w = weights.get(name).unwrap_or(&0);
        println!("  \"{}\" [label=\"{} (w={})\"];", name, name, w);
        for dep in &res.depends_on {
            println!("  \"{}\" -> \"{}\";", name, dep);
        }
    }
    println!("}}");
}

/// Print weighted graph in Mermaid format.
fn print_weight_mermaid(
    config: &types::ForjarConfig,
    weights: &std::collections::HashMap<String, usize>,
) {
    println!("graph LR");
    for (name, res) in &config.resources {
        let w = weights.get(name).unwrap_or(&0);
        for dep in &res.depends_on {
            let dw = weights.get(dep.as_str()).unwrap_or(&0);
            println!(
                "  {}[\"{}(w={})\"] --> {}[\"{}(w={})\"]",
                name, name, w, dep, dep, dw
            );
        }
        if res.depends_on.is_empty() {
            println!("  {}[\"{}(w={})\"]", name, name, w);
        }
    }
}

// ── FJ-484: graph --weight ──

pub(crate) fn cmd_graph_weight(file: &Path, format: &str) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;
    // Weight = number of transitive dependents
    let mut weights: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for name in &order {
        weights.entry(name.clone()).or_insert(0);
        if let Some(res) = config.resources.get(name) {
            for dep in &res.depends_on {
                *weights.entry(dep.clone()).or_insert(0) += 1;
            }
        }
    }
    if format == "dot" {
        print_weight_dot(&config, &weights);
    } else {
        print_weight_mermaid(&config, &weights);
    }
    Ok(())
}

/// Print subgraph in DOT format.
fn print_subgraph_dot(
    resource: &str,
    visited: &std::collections::HashSet<String>,
    config: &types::ForjarConfig,
) {
    println!("digraph subgraph_{} {{", resource);
    println!("  rankdir=LR;");
    for name in visited {
        if let Some(res) = config.resources.get(name) {
            for dep in &res.depends_on {
                if visited.contains(dep) {
                    println!("  \"{}\" -> \"{}\";", name, dep);
                }
            }
        }
    }
    println!("}}");
}

/// Print subgraph in Mermaid format.
fn print_subgraph_mermaid(
    visited: &std::collections::HashSet<String>,
    config: &types::ForjarConfig,
) {
    println!("graph LR");
    for name in visited {
        if let Some(res) = config.resources.get(name) {
            for dep in &res.depends_on {
                if visited.contains(dep) {
                    println!("  {} --> {}", name, dep);
                }
            }
            if res.depends_on.is_empty() {
                println!("  {}", name);
            }
        }
    }
}

// ── FJ-494: graph --subgraph ──

pub(crate) fn cmd_graph_subgraph(file: &Path, format: &str, resource: &str) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    if !config.resources.contains_key(resource) {
        return Err(format!("Resource '{}' not found", resource));
    }
    // Collect transitive dependencies
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut stack = vec![resource.to_string()];
    while let Some(name) = stack.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        if let Some(res) = config.resources.get(&name) {
            for dep in &res.depends_on {
                stack.push(dep.clone());
            }
        }
    }
    if format == "dot" {
        print_subgraph_dot(resource, &visited, &config);
    } else {
        print_subgraph_mermaid(&visited, &config);
    }
    Ok(())
}
