//! Graph export and root analysis.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;
use super::helpers::*;


/// FJ-751: Show root resources (no dependencies).
pub(crate) fn cmd_graph_root_resources(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut roots: Vec<String> = cfg.resources.iter()
        .filter(|(_, r)| r.depends_on.is_empty())
        .map(|(n, _)| n.clone())
        .collect();
    roots.sort();
    if json {
        let items: Vec<String> = roots.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"root_resources\":[{}]}}", items.join(","));
    } else if roots.is_empty() {
        println!("No root resources found (all have dependencies).");
    } else {
        println!("Root resources ({} with no dependencies):", roots.len());
        for name in &roots { println!("  {}", name); }
    }
    Ok(())
}


/// FJ-755: Output graph as edge list (source→target pairs).
pub(crate) fn cmd_graph_edge_list(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let edges = collect_edges(&cfg);
    if json {
        let items: Vec<String> = edges.iter()
            .map(|(s, t)| format!("{{\"source\":\"{}\",\"target\":\"{}\"}}", s, t))
            .collect();
        println!("{{\"edges\":[{}]}}", items.join(","));
    } else if edges.is_empty() {
        println!("No edges (no dependencies).");
    } else {
        println!("Edge list ({} edges):", edges.len());
        for (source, target) in &edges {
            println!("  {} → {}", source, target);
        }
    }
    Ok(())
}

/// Collect all dependency edges from config.
fn collect_edges(cfg: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut edges: Vec<(String, String)> = Vec::new();
    for (name, resource) in &cfg.resources {
        for dep in &resource.depends_on {
            edges.push((dep.clone(), name.clone()));
        }
    }
    edges.sort();
    edges
}
