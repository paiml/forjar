//! Structure analysis.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use std::collections::HashMap;


/// Build bidirectional adjacency map from config.
fn build_bidirectional_adj(
    config: &crate::core::types::ForjarConfig,
) -> std::collections::HashMap<String, std::collections::HashSet<String>> {
    let mut adj: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();
    for name in config.resources.keys() {
        adj.entry(name.clone()).or_default();
    }
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            adj.entry(name.clone()).or_default().insert(dep.clone());
            adj.entry(dep.clone()).or_default().insert(name.clone());
        }
    }
    adj
}

/// Find connected components via BFS on bidirectional adjacency.
fn find_connected_components(
    config: &crate::core::types::ForjarConfig,
    adj: &std::collections::HashMap<String, std::collections::HashSet<String>>,
) -> Vec<Vec<String>> {
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut clusters: Vec<Vec<String>> = Vec::new();

    for name in config.resources.keys() {
        if visited.contains(name) {
            continue;
        }
        let cluster = bfs_component(name, adj, &mut visited);
        clusters.push(cluster);
    }
    clusters.sort_by_key(|b| std::cmp::Reverse(b.len()));
    clusters
}

/// BFS from a starting node to find all connected nodes.
fn bfs_component(
    start: &str,
    adj: &std::collections::HashMap<String, std::collections::HashSet<String>>,
    visited: &mut std::collections::HashSet<String>,
) -> Vec<String> {
    let mut cluster = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start.to_string());
    visited.insert(start.to_string());
    while let Some(current) = queue.pop_front() {
        cluster.push(current.clone());
        if let Some(neighbors) = adj.get(&current) {
            for n in neighbors {
                if !visited.contains(n) {
                    visited.insert(n.clone());
                    queue.push_back(n.clone());
                }
            }
        }
    }
    cluster.sort();
    cluster
}

/// Print clusters as JSON.
fn print_clusters_json(clusters: &[Vec<String>]) {
    print!("{{\"clusters\":[");
    for (i, cluster) in clusters.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        let items: Vec<_> = cluster.iter().map(|c| format!(r#""{}""#, c)).collect();
        print!(
            "{{\"size\":{},\"resources\":[{}]}}",
            cluster.len(),
            items.join(",")
        );
    }
    println!("]}}");
}

/// Print clusters as text.
fn print_clusters_text(clusters: &[Vec<String>]) {
    println!("Resource clusters ({}):", clusters.len());
    for (i, cluster) in clusters.iter().enumerate() {
        println!(
            "  Cluster {} ({} resources): {}",
            i + 1,
            cluster.len(),
            cluster.join(", ")
        );
    }
}

/// Compute dependency depth for each resource via iterative relaxation.
fn compute_dependency_depths(
    config: &crate::core::types::ForjarConfig,
) -> std::collections::HashMap<String, usize> {
    let mut depths: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for name in config.resources.keys() {
        depths.insert(name.clone(), 0);
    }

    let mut changed = true;
    while changed {
        changed = false;
        for (name, resource) in &config.resources {
            for dep in &resource.depends_on {
                let dep_depth = *depths.get(dep).unwrap_or(&0);
                let current = *depths.get(name).unwrap_or(&0);
                if dep_depth + 1 > current {
                    depths.insert(name.clone(), dep_depth + 1);
                    changed = true;
                }
            }
        }
    }
    depths
}

/// Print dependency depths as JSON.
fn print_depths_json(sorted: &[(String, usize)], max_depth: usize) {
    print!("{{\"max_depth\":{},\"resources\":[", max_depth);
    for (i, (name, depth)) in sorted.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!(r#"{{"name":"{}","depth":{}}}"#, name, depth);
    }
    println!("]}}");
}

/// Print dependency depths as text.
fn print_depths_text(sorted: &[(String, usize)], max_depth: usize) {
    println!("Max dependency depth: {}", max_depth);
    for (name, depth) in sorted {
        let bar = "#".repeat(*depth);
        println!("  {} [{}] {}", name, depth, bar);
    }
}

/// FJ-684: Identify tightly-coupled resource clusters
pub(crate) fn cmd_graph_resource_clusters(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

    let adj = build_bidirectional_adj(&config);
    let clusters = find_connected_components(&config, &adj);

    if json {
        print_clusters_json(&clusters);
    } else {
        print_clusters_text(&clusters);
    }
    Ok(())
}


/// FJ-574: Show graph colored/grouped by resource type.
pub(crate) fn cmd_graph_resource_types(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let mut by_type: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, res) in &config.resources {
        let type_name = format!("{:?}", res.resource_type);
        by_type.entry(type_name).or_default().push(name.clone());
    }

    // Sort keys and values
    let mut sorted_types: Vec<(String, Vec<String>)> = by_type.into_iter().collect();
    sorted_types.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, resources) in &mut sorted_types {
        resources.sort();
    }

    if json {
        let items: Vec<String> = sorted_types
            .iter()
            .map(|(t, rs)| {
                let r_items: Vec<String> = rs.iter().map(|r| format!(r#""{}""#, r)).collect();
                format!(
                    r#"{{"type":"{}","resources":[{}],"count":{}}}"#,
                    t,
                    r_items.join(","),
                    rs.len()
                )
            })
            .collect();
        println!(r#"{{"resource_types":[{}]}}"#, items.join(","));
    } else {
        println!("Resources by type:");
        for (type_name, resources) in &sorted_types {
            println!("  {} ({}):", type_name, resources.len());
            for r in resources {
                println!("    {}", r);
            }
        }
    }
    Ok(())
}


/// FJ-614: Show resource age based on last apply timestamp.
pub(crate) fn cmd_graph_resource_age(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let resource_count = config.resources.len();

    // Without state, we can only show resource definitions and their static age info
    if json {
        let items: Vec<String> = config
            .resources
            .keys()
            .map(|name| {
                let rtype = config
                    .resources
                    .get(name)
                    .map(|r| format!("{:?}", r.resource_type))
                    .unwrap_or_else(|| "unknown".to_string());
                format!(
                    r#"{{"resource":"{}","type":"{}","age":"unknown"}}"#,
                    name, rtype
                )
            })
            .collect();
        println!(
            r#"{{"resources":[{}],"total":{}}}"#,
            items.join(","),
            resource_count
        );
    } else if resource_count == 0 {
        println!("No resources found");
    } else {
        println!("Resource age ({} resources):", resource_count);
        println!("  (Run with --state-dir to show actual ages from lock files)");
        for name in config.resources.keys() {
            let rtype = config
                .resources
                .get(name)
                .map(|r| format!("{:?}", r.resource_type))
                .unwrap_or_else(|| "unknown".to_string());
            println!("  {} ({}) — age unknown", name, rtype);
        }
    }
    Ok(())
}


/// FJ-654: Find orphan resources (no dependents or dependencies)
pub(crate) fn cmd_graph_orphan_detection(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

    // Build sets: who depends on whom, who is depended upon
    let mut has_deps: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut is_depended_on: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (name, resource) in &config.resources {
        if !resource.depends_on.is_empty() {
            has_deps.insert(name.clone());
            for dep in &resource.depends_on {
                is_depended_on.insert(dep.clone());
            }
        }
    }

    let mut orphans = Vec::new();
    for name in config.resources.keys() {
        if !has_deps.contains(name) && !is_depended_on.contains(name) {
            orphans.push(name.clone());
        }
    }
    orphans.sort();

    if json {
        print!("{{\"orphans\":[");
        for (i, name) in orphans.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(r#""{}""#, name);
        }
        println!("]}}");
    } else if orphans.is_empty() {
        println!("No orphan resources found — all resources are connected");
    } else {
        println!("Orphan resources ({}):", orphans.len());
        for name in &orphans {
            println!("  - {}", name);
        }
    }
    Ok(())
}


/// FJ-644: Show max dependency depth per resource
pub(crate) fn cmd_graph_dependency_depth(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

    let depths = compute_dependency_depths(&config);

    let mut sorted: Vec<_> = depths.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let max_depth = sorted.first().map(|(_, d)| *d).unwrap_or(0);

    if json {
        print_depths_json(&sorted, max_depth);
    } else {
        print_depths_text(&sorted, max_depth);
    }
    Ok(())
}

