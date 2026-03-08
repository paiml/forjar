//! Graph export and root analysis.

use super::helpers::*;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-751: Show root resources (no dependencies).
pub(crate) fn cmd_graph_root_resources(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut roots: Vec<String> = cfg
        .resources
        .iter()
        .filter(|(_, r)| r.depends_on.is_empty())
        .map(|(n, _)| n.clone())
        .collect();
    roots.sort();
    if json {
        let items: Vec<String> = roots.iter().map(|n| format!("\"{n}\"")).collect();
        println!("{{\"root_resources\":[{}]}}", items.join(","));
    } else if roots.is_empty() {
        println!("No root resources found (all have dependencies).");
    } else {
        println!("Root resources ({} with no dependencies):", roots.len());
        for name in &roots {
            println!("  {name}");
        }
    }
    Ok(())
}

/// FJ-755: Output graph as edge list (source→target pairs).
pub(crate) fn cmd_graph_edge_list(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let edges = collect_edges(&cfg);
    if json {
        let items: Vec<String> = edges
            .iter()
            .map(|(s, t)| format!("{{\"source\":\"{s}\",\"target\":\"{t}\"}}"))
            .collect();
        println!("{{\"edges\":[{}]}}", items.join(","));
    } else if edges.is_empty() {
        println!("No edges (no dependencies).");
    } else {
        println!("Edge list ({} edges):", edges.len());
        for (source, target) in &edges {
            println!("  {source} → {target}");
        }
    }
    Ok(())
}

/// Collect all dependency edges from config.
pub(super) fn collect_edges(cfg: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut edges: Vec<(String, String)> = Vec::new();
    for (name, resource) in &cfg.resources {
        for dep in &resource.depends_on {
            edges.push((dep.clone(), name.clone()));
        }
    }
    edges.sort();
    edges
}

/// FJ-759: Show disconnected subgraphs (connected components).
pub(crate) fn cmd_graph_connected_components(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let components = find_connected_components(&cfg);
    if json {
        let items: Vec<String> = components.iter().map(|c| format!("{c:?}")).collect();
        println!(
            "{{\"connected_components\":[{}],\"count\":{}}}",
            items.join(","),
            components.len()
        );
    } else if components.is_empty() {
        println!("No resources (empty graph).");
    } else {
        println!("Connected components ({}):", components.len());
        for (i, comp) in components.iter().enumerate() {
            println!(
                "  Component {} ({} resources): {}",
                i + 1,
                comp.len(),
                comp.join(", ")
            );
        }
    }
    Ok(())
}

/// Build undirected adjacency list from config dependencies.
pub(crate) fn build_undirected_graph(
    cfg: &types::ForjarConfig,
) -> std::collections::HashMap<&str, Vec<&str>> {
    let mut adj: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for (name, resource) in &cfg.resources {
        adj.entry(name.as_str()).or_default();
        for dep in &resource.depends_on {
            adj.entry(name.as_str()).or_default().push(dep.as_str());
            adj.entry(dep.as_str()).or_default().push(name.as_str());
        }
    }
    adj
}

/// DFS from a start node, collecting all reachable nodes into a component.
pub(super) fn collect_dfs_component<'a>(
    start: &'a str,
    adj: &std::collections::HashMap<&str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
) -> Vec<String> {
    let mut comp = Vec::new();
    let mut stack = vec![start];
    while let Some(n) = stack.pop() {
        if visited.contains(n) {
            continue;
        }
        visited.insert(n);
        comp.push(n.to_string());
        if let Some(neighbors) = adj.get(n) {
            for &next in neighbors {
                if !visited.contains(next) {
                    stack.push(next);
                }
            }
        }
    }
    comp.sort();
    comp
}

/// Find connected components using undirected DFS.
pub(super) fn find_connected_components(cfg: &types::ForjarConfig) -> Vec<Vec<String>> {
    let adj = build_undirected_graph(cfg);
    let mut visited = std::collections::HashSet::new();
    let mut components = Vec::new();
    let mut names: Vec<&str> = cfg.resources.keys().map(|k| k.as_str()).collect();
    names.sort();
    for name in names {
        if visited.contains(name) {
            continue;
        }
        components.push(collect_dfs_component(name, &adj, &mut visited));
    }
    components
}

/// FJ-763: Output graph as adjacency matrix.
pub(crate) fn cmd_graph_adjacency_matrix(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let (names, matrix) = build_adjacency_matrix(&cfg);
    if json {
        let rows: Vec<String> = matrix
            .iter()
            .map(|row| {
                format!(
                    "[{}]",
                    row.iter()
                        .map(|&v| if v { "1" } else { "0" })
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect();
        let labels: Vec<String> = names.iter().map(|n| format!("\"{n}\"")).collect();
        println!(
            "{{\"labels\":[{}],\"matrix\":[{}]}}",
            labels.join(","),
            rows.join(",")
        );
    } else if names.is_empty() {
        println!("No resources (empty graph).");
    } else {
        print_adjacency_table(&names, &matrix);
    }
    Ok(())
}

/// Build NxN adjacency matrix from config dependencies.
pub(crate) fn build_adjacency_matrix(cfg: &types::ForjarConfig) -> (Vec<String>, Vec<Vec<bool>>) {
    let mut names: Vec<String> = cfg.resources.keys().cloned().collect();
    names.sort();
    let idx: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let n = names.len();
    let mut matrix = vec![vec![false; n]; n];
    for (name, resource) in &cfg.resources {
        if let Some(&to) = idx.get(name.as_str()) {
            for dep in &resource.depends_on {
                if let Some(&from) = idx.get(dep.as_str()) {
                    matrix[from][to] = true;
                }
            }
        }
    }
    (names, matrix)
}

/// Print a simple text adjacency table.
pub(super) fn print_adjacency_table(names: &[String], matrix: &[Vec<bool>]) {
    let max_len = names.iter().map(|n| n.len()).max().unwrap_or(0);
    print!("{:width$} ", "", width = max_len);
    for n in names {
        print!("{} ", n.chars().next().unwrap_or(' '));
    }
    println!();
    for (i, name) in names.iter().enumerate() {
        print!("{name:max_len$} ");
        for &cell in &matrix[i] {
            print!("{} ", if cell { "1" } else { "." });
        }
        println!();
    }
}

/// FJ-767: Show longest dependency chain length.
pub(crate) fn cmd_graph_longest_path(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let (length, chain) = find_longest_chain(&cfg);
    if json {
        let items: Vec<String> = chain.iter().map(|n| format!("\"{n}\"")).collect();
        println!(
            "{{\"longest_path_length\":{},\"chain\":[{}]}}",
            length,
            items.join(",")
        );
    } else if length == 0 {
        println!("No dependency chains (all resources independent).");
    } else {
        println!(
            "Longest dependency chain ({} edges): {}",
            length,
            chain.join(" → ")
        );
    }
    Ok(())
}

/// Relax edges to compute longest distances and predecessors.
pub(super) fn relax_dag_edges(
    cfg: &types::ForjarConfig,
    names: &[&str],
    idx: &std::collections::HashMap<&str, usize>,
    dist: &mut [usize],
    prev: &mut [usize],
) {
    let mut order: Vec<usize> = (0..names.len()).collect();
    order.sort_by_key(|&i| {
        cfg.resources
            .get(names[i])
            .map(|r| r.depends_on.len())
            .unwrap_or(0)
    });
    for &u in &order {
        if let Some(resource) = cfg.resources.get(names[u]) {
            for dep in &resource.depends_on {
                if let Some(&v) = idx.get(dep.as_str()) {
                    if dist[v] + 1 > dist[u] {
                        dist[u] = dist[v] + 1;
                        prev[u] = v;
                    }
                }
            }
        }
    }
}

pub(super) use super::graph_export_b::*;
