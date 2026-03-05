use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// FJ-811: Show max dependency chain depth per resource.
pub(crate) fn cmd_graph_dependency_depth_per_resource(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let names: Vec<&str> = config.resources.keys().map(|s| s.as_str()).collect();
    let name_idx: std::collections::HashMap<&str, usize> =
        names.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let n = names.len();
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    for (name, resource) in &config.resources {
        if let Some(&from) = name_idx.get(name.as_str()) {
            for dep in &resource.depends_on {
                if let Some(&to) = name_idx.get(dep.as_str()) {
                    adj[from].push(to);
                }
            }
        }
    }
    let mut depths: Vec<(String, usize)> = names
        .iter()
        .map(|&name| {
            let idx = name_idx[name];
            let depth = compute_max_depth(idx, &adj, &mut vec![None; n]);
            (name.to_string(), depth)
        })
        .collect();
    depths.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if json {
        let items: Vec<String> = depths
            .iter()
            .map(|(r, d)| format!("{{\"resource\":\"{r}\",\"depth\":{d}}}"))
            .collect();
        println!("{{\"dependency_depths\":[{}]}}", items.join(","));
    } else if depths.is_empty() {
        println!("No resources.");
    } else {
        println!("Dependency depth per resource:");
        for (r, d) in &depths {
            println!("  {r} — depth {d}");
        }
    }
    Ok(())
}

fn compute_max_depth(node: usize, adj: &[Vec<usize>], cache: &mut [Option<usize>]) -> usize {
    if let Some(d) = cache[node] {
        return d;
    }
    cache[node] = Some(0); // cycle guard
    let d = adj[node]
        .iter()
        .map(|&next| 1 + compute_max_depth(next, adj, cache))
        .max()
        .unwrap_or(0);
    cache[node] = Some(d);
    d
}

/// FJ-815: Fan-in count per resource (how many depend on it).
pub(crate) fn cmd_graph_resource_fanin(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let mut fanin: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for name in config.resources.keys() {
        fanin.insert(name.clone(), 0);
    }
    for resource in config.resources.values() {
        for dep in &resource.depends_on {
            *fanin.entry(dep.clone()).or_default() += 1;
        }
    }
    let mut sorted: Vec<(String, usize)> = fanin.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if json {
        let items: Vec<String> = sorted
            .iter()
            .map(|(r, c)| format!("{{\"resource\":\"{r}\",\"fanin\":{c}}}"))
            .collect();
        println!("{{\"resource_fanin\":[{}]}}", items.join(","));
    } else if sorted.is_empty() {
        println!("No resources.");
    } else {
        println!("Fan-in per resource:");
        for (r, c) in &sorted {
            println!("  {r} — {c} dependents");
        }
    }
    Ok(())
}

/// FJ-819: Detect disconnected subgraphs in the DAG.
pub(crate) fn cmd_graph_isolated_subgraphs(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let components = find_connected_components(&config);
    if json {
        let items: Vec<String> = components
            .iter()
            .map(|c| {
                let members: Vec<String> = c.iter().map(|s| format!("\"{s}\"")).collect();
                format!("[{}]", members.join(","))
            })
            .collect();
        println!(
            "{{\"subgraphs\":[{}],\"count\":{}}}",
            items.join(","),
            components.len()
        );
    } else if components.len() <= 1 {
        println!("Graph is fully connected ({} component).", components.len());
    } else {
        println!("Isolated subgraphs ({}):", components.len());
        for (i, c) in components.iter().enumerate() {
            println!("  Subgraph {}: {}", i + 1, c.join(", "));
        }
    }
    Ok(())
}

fn build_undirected_adj(config: &types::ForjarConfig) -> (Vec<&String>, Vec<Vec<usize>>) {
    let names: Vec<&String> = config.resources.keys().collect();
    let idx: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut adj = vec![vec![]; names.len()];
    for (name, resource) in &config.resources {
        let from = idx[name.as_str()];
        for dep in &resource.depends_on {
            if let Some(&to) = idx.get(dep.as_str()) {
                adj[from].push(to);
                adj[to].push(from);
            }
        }
    }
    (names, adj)
}

fn dfs_component(start: usize, adj: &[Vec<usize>], visited: &mut [bool]) -> Vec<usize> {
    let mut stack = vec![start];
    let mut comp = Vec::new();
    while let Some(node) = stack.pop() {
        if visited[node] {
            continue;
        }
        visited[node] = true;
        comp.push(node);
        for &next in &adj[node] {
            if !visited[next] {
                stack.push(next);
            }
        }
    }
    comp
}

fn find_connected_components(config: &types::ForjarConfig) -> Vec<Vec<String>> {
    let (names, adj) = build_undirected_adj(config);
    let mut visited = vec![false; names.len()];
    let mut components = Vec::new();
    for start in 0..names.len() {
        if visited[start] {
            continue;
        }
        let indices = dfs_component(start, &adj, &mut visited);
        let mut comp: Vec<String> = indices.iter().map(|&i| names[i].clone()).collect();
        comp.sort();
        components.push(comp);
    }
    components.sort_by_key(|c| std::cmp::Reverse(c.len()));
    components
}

/// FJ-903: Critical path length through dependency graph.
pub(crate) fn cmd_graph_resource_dependency_critical_path_length(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let paths = compute_critical_path_lengths(&config);
    if json {
        let items: Vec<String> = paths
            .iter()
            .map(|(n, l)| format!("{{\"resource\":\"{n}\",\"critical_path_length\":{l}}}"))
            .collect();
        println!("{{\"critical_path_lengths\":[{}]}}", items.join(","));
    } else if paths.is_empty() {
        println!("No resources to analyze.");
    } else {
        println!("Critical path lengths (longest chain to root):");
        for (n, l) in &paths {
            println!("  {n} — {l}");
        }
    }
    Ok(())
}

fn compute_critical_path_lengths(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut lengths: Vec<(String, usize)> = config
        .resources
        .iter()
        .map(|(name, _)| {
            let depth = compute_path_depth(name, config, &mut std::collections::HashSet::new());
            (name.clone(), depth)
        })
        .collect();
    lengths.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    lengths
}

fn compute_path_depth(
    name: &str,
    config: &types::ForjarConfig,
    visited: &mut std::collections::HashSet<String>,
) -> usize {
    if visited.contains(name) {
        return 0;
    }
    visited.insert(name.to_string());
    let res = match config.resources.get(name) {
        Some(r) => r,
        None => return 0,
    };
    let max_dep = res
        .depends_on
        .iter()
        .map(|d| compute_path_depth(d, config, visited))
        .max()
        .unwrap_or(0);
    max_dep + 1
}

/// FJ-907: Redundancy score for resources with fallbacks.
pub(crate) fn cmd_graph_resource_dependency_redundancy_score(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let scores = compute_redundancy_scores(&config);
    if json {
        let items: Vec<String> = scores
            .iter()
            .map(|(n, s)| format!("{{\"resource\":\"{n}\",\"redundancy_score\":{s:.2}}}"))
            .collect();
        println!("{{\"redundancy_scores\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No resources to analyze.");
    } else {
        println!("Redundancy scores (higher = more redundant paths):");
        for (n, s) in &scores {
            println!("  {n} — {s:.2}");
        }
    }
    Ok(())
}

fn compute_redundancy_scores(config: &types::ForjarConfig) -> Vec<(String, f64)> {
    let mut scores: Vec<(String, f64)> = config
        .resources
        .iter()
        .map(|(name, _)| {
            let dependents = config
                .resources
                .values()
                .filter(|r| r.depends_on.contains(name))
                .count();
            let score: f64 = if dependents > 1 {
                1.0 - (1.0 / dependents as f64)
            } else {
                0.0
            };
            (name.clone(), score)
        })
        .collect();
    scores.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    scores
}
