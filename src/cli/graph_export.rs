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


/// FJ-759: Show disconnected subgraphs (connected components).
pub(crate) fn cmd_graph_connected_components(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let components = find_connected_components(&cfg);
    if json {
        let items: Vec<String> = components.iter()
            .map(|c| format!("{:?}", c))
            .collect();
        println!("{{\"connected_components\":[{}],\"count\":{}}}", items.join(","), components.len());
    } else if components.is_empty() {
        println!("No resources (empty graph).");
    } else {
        println!("Connected components ({}):", components.len());
        for (i, comp) in components.iter().enumerate() {
            println!("  Component {} ({} resources): {}", i + 1, comp.len(), comp.join(", "));
        }
    }
    Ok(())
}

/// Build undirected adjacency list from config dependencies.
fn build_undirected_graph<'a>(cfg: &'a types::ForjarConfig) -> std::collections::HashMap<&'a str, Vec<&'a str>> {
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
fn collect_dfs_component<'a>(
    start: &'a str,
    adj: &std::collections::HashMap<&str, Vec<&'a str>>,
    visited: &mut std::collections::HashSet<&'a str>,
) -> Vec<String> {
    let mut comp = Vec::new();
    let mut stack = vec![start];
    while let Some(n) = stack.pop() {
        if visited.contains(n) { continue; }
        visited.insert(n);
        comp.push(n.to_string());
        if let Some(neighbors) = adj.get(n) {
            for &next in neighbors {
                if !visited.contains(next) { stack.push(next); }
            }
        }
    }
    comp.sort();
    comp
}

/// Find connected components using undirected DFS.
fn find_connected_components(cfg: &types::ForjarConfig) -> Vec<Vec<String>> {
    let adj = build_undirected_graph(cfg);
    let mut visited = std::collections::HashSet::new();
    let mut components = Vec::new();
    let mut names: Vec<&str> = cfg.resources.keys().map(|k| k.as_str()).collect();
    names.sort();
    for name in names {
        if visited.contains(name) { continue; }
        components.push(collect_dfs_component(name, &adj, &mut visited));
    }
    components
}


/// FJ-763: Output graph as adjacency matrix.
pub(crate) fn cmd_graph_adjacency_matrix(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let (names, matrix) = build_adjacency_matrix(&cfg);
    if json {
        let rows: Vec<String> = matrix.iter()
            .map(|row| format!("[{}]", row.iter().map(|&v| if v { "1" } else { "0" }).collect::<Vec<_>>().join(",")))
            .collect();
        let labels: Vec<String> = names.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"labels\":[{}],\"matrix\":[{}]}}", labels.join(","), rows.join(","));
    } else if names.is_empty() {
        println!("No resources (empty graph).");
    } else {
        print_adjacency_table(&names, &matrix);
    }
    Ok(())
}

/// Build NxN adjacency matrix from config dependencies.
fn build_adjacency_matrix(cfg: &types::ForjarConfig) -> (Vec<String>, Vec<Vec<bool>>) {
    let mut names: Vec<String> = cfg.resources.keys().cloned().collect();
    names.sort();
    let idx: std::collections::HashMap<&str, usize> = names.iter().enumerate()
        .map(|(i, n)| (n.as_str(), i)).collect();
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
fn print_adjacency_table(names: &[String], matrix: &[Vec<bool>]) {
    let max_len = names.iter().map(|n| n.len()).max().unwrap_or(0);
    print!("{:width$} ", "", width = max_len);
    for n in names { print!("{} ", &n[..1]); }
    println!();
    for (i, name) in names.iter().enumerate() {
        print!("{:width$} ", name, width = max_len);
        for j in 0..names.len() {
            print!("{} ", if matrix[i][j] { "1" } else { "." });
        }
        println!();
    }
}


/// FJ-767: Show longest dependency chain length.
pub(crate) fn cmd_graph_longest_path(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let (length, chain) = find_longest_chain(&cfg);
    if json {
        let items: Vec<String> = chain.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"longest_path_length\":{},\"chain\":[{}]}}", length, items.join(","));
    } else if length == 0 {
        println!("No dependency chains (all resources independent).");
    } else {
        println!("Longest dependency chain ({} edges): {}", length, chain.join(" → "));
    }
    Ok(())
}

/// Relax edges to compute longest distances and predecessors.
fn relax_dag_edges(
    cfg: &types::ForjarConfig, names: &[&str],
    idx: &std::collections::HashMap<&str, usize>,
    dist: &mut [usize], prev: &mut [usize],
) {
    let mut order: Vec<usize> = (0..names.len()).collect();
    order.sort_by_key(|&i| {
        cfg.resources.get(names[i]).map(|r| r.depends_on.len()).unwrap_or(0)
    });
    for &u in &order {
        if let Some(resource) = cfg.resources.get(names[u]) {
            for dep in &resource.depends_on {
                if let Some(&v) = idx.get(dep.as_str()) {
                    if dist[v] + 1 > dist[u] { dist[u] = dist[v] + 1; prev[u] = v; }
                }
            }
        }
    }
}

/// Reconstruct chain from predecessor array.
fn reconstruct_chain(names: &[&str], prev: &[usize], start: usize) -> Vec<String> {
    let mut chain = vec![names[start].to_string()];
    let mut cur = start;
    while prev[cur] != usize::MAX {
        cur = prev[cur];
        chain.push(names[cur].to_string());
    }
    chain.reverse();
    chain
}

/// Find longest path in DAG using topological-order DP.
fn find_longest_chain(cfg: &types::ForjarConfig) -> (usize, Vec<String>) {
    let names: Vec<&str> = cfg.resources.keys().map(|k| k.as_str()).collect();
    let n = names.len();
    if n == 0 { return (0, Vec::new()); }
    let idx: std::collections::HashMap<&str, usize> = names.iter().enumerate()
        .map(|(i, &n)| (n, i)).collect();
    let mut dist = vec![0usize; n];
    let mut prev = vec![usize::MAX; n];
    relax_dag_edges(cfg, &names, &idx, &mut dist, &mut prev);
    let best = (0..n).max_by_key(|&i| dist[i]).unwrap_or(0);
    (dist[best], reconstruct_chain(&names, &prev, best))
}


/// FJ-771: Show in-degree (number of dependents) per resource.
pub(crate) fn cmd_graph_in_degree(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let degrees = compute_in_degrees(&cfg);
    if json {
        let items: Vec<String> = degrees.iter()
            .map(|(n, d)| format!("{{\"resource\":\"{}\",\"in_degree\":{}}}", n, d))
            .collect();
        println!("{{\"in_degrees\":[{}]}}", items.join(","));
    } else if degrees.is_empty() {
        println!("No resources.");
    } else {
        println!("In-degree (dependents) per resource:");
        for (name, deg) in &degrees { println!("  {} — {}", name, deg); }
    }
    Ok(())
}

/// Compute in-degree for each resource (how many others depend on it).
fn compute_in_degrees(cfg: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut deg: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for name in cfg.resources.keys() { deg.insert(name.clone(), 0); }
    for resource in cfg.resources.values() {
        for dep in &resource.depends_on {
            *deg.entry(dep.clone()).or_default() += 1;
        }
    }
    let mut result: Vec<(String, usize)> = deg.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    result
}


/// FJ-775: Show out-degree (number of dependencies) per resource.
pub(crate) fn cmd_graph_out_degree(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut degrees: Vec<(String, usize)> = cfg.resources.iter()
        .map(|(n, r)| (n.clone(), r.depends_on.len()))
        .collect();
    degrees.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if json {
        let items: Vec<String> = degrees.iter()
            .map(|(n, d)| format!("{{\"resource\":\"{}\",\"out_degree\":{}}}", n, d))
            .collect();
        println!("{{\"out_degrees\":[{}]}}", items.join(","));
    } else if degrees.is_empty() {
        println!("No resources.");
    } else {
        println!("Out-degree (dependencies) per resource:");
        for (name, deg) in &degrees { println!("  {} — {}", name, deg); }
    }
    Ok(())
}


/// FJ-779: Show graph density (edges / max-possible-edges).
pub(crate) fn cmd_graph_density(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let n = cfg.resources.len();
    let edges: usize = cfg.resources.values().map(|r| r.depends_on.len()).sum();
    let max_edges = if n > 1 { n * (n - 1) } else { 1 };
    let density = edges as f64 / max_edges as f64;
    if json {
        println!("{{\"nodes\":{},\"edges\":{},\"max_edges\":{},\"density\":{:.4}}}", n, edges, max_edges, density);
    } else {
        println!("Graph density: {:.4} ({} edges / {} max, {} nodes)", density, edges, max_edges, n);
    }
    Ok(())
}


/// FJ-783: Output resources in valid topological execution order.
pub(crate) fn cmd_graph_topological_sort(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let order = topological_sort_resources(&cfg);
    if json {
        let items: Vec<String> = order.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"topological_order\":[{}]}}", items.join(","));
    } else if order.is_empty() {
        println!("No resources (empty graph).");
    } else {
        println!("Topological execution order ({} resources):", order.len());
        for (i, name) in order.iter().enumerate() { println!("  {}. {}", i + 1, name); }
    }
    Ok(())
}

/// Build in-degree map and dependents adjacency for Kahn's algorithm.
fn build_kahn_graph<'a>(cfg: &'a types::ForjarConfig) -> (
    std::collections::HashMap<&'a str, usize>,
    std::collections::HashMap<&'a str, Vec<&'a str>>,
) {
    let mut in_deg: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut dependents: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for name in cfg.resources.keys() { in_deg.insert(name.as_str(), 0); }
    for (name, resource) in &cfg.resources {
        for dep in &resource.depends_on {
            if cfg.resources.contains_key(dep) {
                *in_deg.entry(name.as_str()).or_default() += 1;
                dependents.entry(dep.as_str()).or_default().push(name.as_str());
            }
        }
    }
    (in_deg, dependents)
}

/// Process Kahn's queue: pop nodes, decrement dependents, collect result.
fn kahn_process<'a>(
    in_deg: &mut std::collections::HashMap<&'a str, usize>,
    dependents: &std::collections::HashMap<&str, Vec<&'a str>>,
) -> Vec<String> {
    let mut queue: Vec<&str> = in_deg.iter()
        .filter(|(_, &d)| d == 0)
        .map(|(&n, _)| n)
        .collect();
    queue.sort();
    let mut deque: std::collections::VecDeque<&str> = queue.into_iter().collect();
    let mut result = Vec::new();
    while let Some(n) = deque.pop_front() {
        result.push(n.to_string());
        if let Some(deps) = dependents.get(n) {
            let mut next: Vec<&str> = Vec::new();
            for &d in deps {
                if let Some(deg) = in_deg.get_mut(d) {
                    *deg -= 1;
                    if *deg == 0 { next.push(d); }
                }
            }
            next.sort();
            deque.extend(next);
        }
    }
    result
}

/// Kahn's algorithm for topological sort.
fn topological_sort_resources(cfg: &types::ForjarConfig) -> Vec<String> {
    let (mut in_deg, dependents) = build_kahn_graph(cfg);
    kahn_process(&mut in_deg, &dependents)
}


/// FJ-787: Show resources on the longest dependency chain (critical path).
pub(crate) fn cmd_graph_critical_path_resources(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let (length, chain) = find_longest_chain(&cfg);
    if json {
        let items: Vec<String> = chain.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"critical_path_length\":{},\"resources\":[{}]}}", length, items.join(","));
    } else if length == 0 {
        println!("No dependency chains (all resources independent).");
    } else {
        println!("Critical path ({} edges, {} resources):", length, chain.len());
        for (i, name) in chain.iter().enumerate() { println!("  {}. {}", i + 1, name); }
    }
    Ok(())
}
