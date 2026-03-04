use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// FJ-694: Show execution order with timing estimates
/// FJ-694: Show resource fan-out metrics (how many resources depend on each)
pub(crate) fn cmd_graph_fan_out(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut fan_out: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for name in cfg.resources.keys() {
        fan_out.entry(name.clone()).or_default();
    }
    for (name, resource) in &cfg.resources {
        for dep in &resource.depends_on {
            fan_out.entry(dep.clone()).or_default().push(name.clone());
        }
    }
    let mut sorted: Vec<_> = fan_out.into_iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    if json {
        let entries: Vec<String> = sorted
            .iter()
            .map(|(name, dependents)| {
                let deps: Vec<String> = dependents.iter().map(|d| format!("\"{d}\"")).collect();
                format!(
                    "{{\"resource\":\"{}\",\"fan_out\":{},\"dependents\":[{}]}}",
                    name,
                    dependents.len(),
                    deps.join(",")
                )
            })
            .collect();
        println!("{{\"fan_out\":[{}]}}", entries.join(","));
    } else {
        println!("Resource fan-out (dependents count):");
        for (name, dependents) in &sorted {
            if dependents.is_empty() {
                println!("  {name} — 0 dependents (leaf)");
            } else {
                println!(
                    "  {} — {} dependent(s): {}",
                    name,
                    dependents.len(),
                    dependents.join(", ")
                );
            }
        }
    }
    Ok(())
}

/// FJ-724: Show depth-first traversal order
pub(crate) fn cmd_graph_depth_first(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut order: Vec<String> = Vec::new();
    fn dfs(
        name: &str,
        cfg: &types::ForjarConfig,
        visited: &mut std::collections::HashSet<String>,
        order: &mut Vec<String>,
    ) {
        if visited.contains(name) {
            return;
        }
        visited.insert(name.to_string());
        if let Some(resource) = cfg.resources.get(name) {
            for dep in &resource.depends_on {
                dfs(dep, cfg, visited, order);
            }
        }
        order.push(name.to_string());
    }
    let mut names: Vec<String> = cfg.resources.keys().cloned().collect();
    names.sort();
    for name in &names {
        dfs(name, &cfg, &mut visited, &mut order);
    }
    if json {
        let entries: Vec<String> = order
            .iter()
            .enumerate()
            .map(|(i, n)| format!("{{\"step\":{},\"resource\":\"{}\"}}", i + 1, n))
            .collect();
        println!("{{\"depth_first_order\":[{}]}}", entries.join(","));
    } else {
        println!("Depth-first traversal ({} resources):", order.len());
        for (i, name) in order.iter().enumerate() {
            println!("  {}. {}", i + 1, name);
        }
    }
    Ok(())
}

/// BFS topological sort: returns resources in breadth-first order.
fn bfs_topological(cfg: &types::ForjarConfig) -> Vec<String> {
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut dependents: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, resource) in &cfg.resources {
        in_degree.entry(name.clone()).or_insert(0);
        for dep in &resource.depends_on {
            dependents
                .entry(dep.clone())
                .or_default()
                .push(name.clone());
            *in_degree.entry(name.clone()).or_default() += 1;
        }
    }
    let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    let mut roots: Vec<String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    roots.sort();
    for r in roots {
        queue.push_back(r);
    }
    let mut order: Vec<String> = Vec::new();
    while let Some(node) = queue.pop_front() {
        order.push(node.clone());
        let mut next: Vec<String> = dependents.get(&node).cloned().unwrap_or_default();
        next.sort();
        for dep in next {
            if let Some(d) = in_degree.get_mut(&dep) {
                *d -= 1;
                if *d == 0 {
                    queue.push_back(dep);
                }
            }
        }
    }
    order
}

/// FJ-734: Show breadth-first traversal order.
pub(crate) fn cmd_graph_breadth_first(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let order = bfs_topological(&cfg);
    if json {
        let entries: Vec<String> = order
            .iter()
            .enumerate()
            .map(|(i, n)| format!("{{\"step\":{},\"resource\":\"{}\"}}", i + 1, n))
            .collect();
        println!("{{\"breadth_first_order\":[{}]}}", entries.join(","));
    } else {
        println!("Breadth-first traversal ({} resources):", order.len());
        for (i, name) in order.iter().enumerate() {
            println!("  {}. {}", i + 1, name);
        }
    }
    Ok(())
}

/// Compute in-degree and out-degree for each resource.
fn compute_degrees(cfg: &types::ForjarConfig) -> Vec<(String, usize, usize)> {
    let mut in_deg: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut out_deg: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (name, resource) in &cfg.resources {
        out_deg.insert(name.clone(), resource.depends_on.len());
        in_deg.entry(name.clone()).or_insert(0);
        for dep in &resource.depends_on {
            *in_deg.entry(dep.clone()).or_default() += 1;
        }
    }
    let mut result: Vec<(String, usize, usize)> = cfg
        .resources
        .keys()
        .map(|n| {
            (
                n.clone(),
                *in_deg.get(n).unwrap_or(&0),
                *out_deg.get(n).unwrap_or(&0),
            )
        })
        .collect();
    result.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    result
}

/// FJ-747: Show in-degree and out-degree per resource.
pub(crate) fn cmd_graph_dependency_count(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let degrees = compute_degrees(&cfg);
    if json {
        let items: Vec<String> = degrees
            .iter()
            .map(|(n, i, o)| {
                format!(
                    "{{\"resource\":\"{n}\",\"in_degree\":{i},\"out_degree\":{o}}}"
                )
            })
            .collect();
        println!("{{\"dependency_counts\":[{}]}}", items.join(","));
    } else {
        println!("Dependency counts ({} resources):", degrees.len());
        for (name, ind, outd) in &degrees {
            println!("  {name} — in:{ind} out:{outd}");
        }
    }
    Ok(())
}

/// FJ-743: Show stats for each connected component.
pub(crate) fn cmd_graph_subgraph_stats(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let components = find_components(&cfg);
    if json {
        let items: Vec<String> = components
            .iter()
            .enumerate()
            .map(|(i, c)| {
                format!(
                    "{{\"component\":{},\"nodes\":{},\"resources\":{:?}}}",
                    i + 1,
                    c.len(),
                    c
                )
            })
            .collect();
        println!("{{\"subgraph_stats\":[{}]}}", items.join(","));
    } else {
        println!("Connected components: {}", components.len());
        for (i, c) in components.iter().enumerate() {
            println!(
                "  Component {} — {} node(s): {}",
                i + 1,
                c.len(),
                c.join(", ")
            );
        }
    }
    Ok(())
}

/// Build undirected adjacency list from resource graph.
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

/// DFS to collect a single connected component starting from a node.
fn collect_component(
    start: &str,
    adj: &std::collections::HashMap<String, Vec<String>>,
    visited: &mut std::collections::HashSet<String>,
) -> Vec<String> {
    let mut component = Vec::new();
    let mut stack = vec![start.to_string()];
    while let Some(n) = stack.pop() {
        if visited.contains(&n) {
            continue;
        }
        visited.insert(n.clone());
        component.push(n.clone());
        for nb in adj.get(&n).unwrap_or(&Vec::new()) {
            stack.push(nb.clone());
        }
    }
    component.sort();
    component
}

/// Find connected components (undirected) in resource graph.
fn find_components(cfg: &types::ForjarConfig) -> Vec<Vec<String>> {
    let adj = build_undirected_adj(cfg);
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut names: Vec<String> = adj.keys().cloned().collect();
    names.sort();
    let mut components = Vec::new();
    for name in &names {
        if !visited.contains(name) {
            components.push(collect_component(name, &adj, &mut visited));
        }
    }
    components
}
