//! Phase 102 — Resource Intelligence & Topology Insight: graph commands (FJ-1079, FJ-1082).

use crate::core::types;
use std::path::Path;

fn parse_and_validate(file: &Path) -> Result<types::ForjarConfig, String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())
}

fn build_undirected_adj(
    cfg: &types::ForjarConfig,
) -> std::collections::HashMap<String, Vec<String>> {
    let mut adj: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (name, resource) in &cfg.resources {
        adj.entry(name.clone()).or_default();
        for dep in &resource.depends_on {
            adj.entry(name.clone()).or_default().push(dep.clone());
            adj.entry(dep.clone()).or_default().push(name.clone());
        }
    }
    adj
}

/// FJ-1079: Identify clusters of tightly coupled resources using connected component analysis.
pub(crate) fn cmd_graph_resource_topology_cluster_analysis(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let adj = build_undirected_adj(&config);
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut names: Vec<String> = adj.keys().cloned().collect();
    names.sort();

    let mut clusters: Vec<Vec<String>> = Vec::new();
    for name in &names {
        if visited.contains(name) {
            continue;
        }
        let mut component: Vec<String> = Vec::new();
        let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        queue.push_back(name.clone());
        visited.insert(name.clone());
        while let Some(node) = queue.pop_front() {
            component.push(node.clone());
            if let Some(neighbors) = adj.get(&node) {
                for nb in neighbors {
                    if !visited.contains(nb) {
                        visited.insert(nb.clone());
                        queue.push_back(nb.clone());
                    }
                }
            }
        }
        component.sort();
        clusters.push(component);
    }

    if json {
        let items: Vec<serde_json::Value> = clusters
            .iter()
            .enumerate()
            .map(|(i, members)| serde_json::json!({"id": i, "members": members}))
            .collect();
        println!(
            "{}",
            serde_json::json!({"clusters": items, "count": clusters.len()})
        );
    } else {
        println!("Topology cluster analysis ({} cluster(s)):", clusters.len());
        for (i, members) in clusters.iter().enumerate() {
            println!(
                "  Cluster {} ({} member(s)): {}",
                i,
                members.len(),
                members.join(", ")
            );
        }
    }
    Ok(())
}

/// FJ-1082: Find disconnected subgraphs (islands) — resources with no dependencies in or out.
pub(crate) fn cmd_graph_resource_dependency_island_detection(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut has_outgoing: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut has_incoming: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (name, resource) in &config.resources {
        if !resource.depends_on.is_empty() {
            has_outgoing.insert(name.clone());
            for dep in &resource.depends_on {
                has_incoming.insert(dep.clone());
            }
        }
    }
    let mut islands: Vec<String> = config
        .resources
        .keys()
        .filter(|name| !has_outgoing.contains(*name) && !has_incoming.contains(*name))
        .cloned()
        .collect();
    islands.sort();

    if json {
        println!(
            "{}",
            serde_json::json!({"islands": islands, "count": islands.len()})
        );
    } else {
        println!("Dependency island detection ({} island(s)):", islands.len());
        if islands.is_empty() {
            println!("  (no islands detected)");
        } else {
            for name in &islands {
                println!("  {name}");
            }
        }
    }
    Ok(())
}
