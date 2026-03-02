//! Advanced graph analysis — bipartite, SCC, CSV export.

use super::graph_export::{build_adjacency_matrix, build_undirected_graph, compute_in_degrees};
use super::helpers::*;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-795: Check if dependency graph is bipartite.
pub(crate) fn cmd_graph_bipartite_check(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let is_bip = check_bipartite(&cfg);
    if json {
        println!("{{\"is_bipartite\":{}}}", is_bip);
    } else if is_bip {
        println!("The dependency graph is bipartite.");
    } else {
        println!("The dependency graph is NOT bipartite (contains odd-length cycle).");
    }
    Ok(())
}

/// Check bipartite using 2-coloring BFS on undirected graph.
fn check_bipartite(cfg: &types::ForjarConfig) -> bool {
    let adj = build_undirected_graph(cfg);
    let mut color: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    for &start in adj.keys() {
        if color.contains_key(start) {
            continue;
        }
        color.insert(start, false);
        if !bfs_2color(start, &adj, &mut color) {
            return false;
        }
    }
    true
}

/// BFS 2-coloring from a start node. Returns false if odd cycle found.
fn bfs_2color<'a>(
    start: &'a str,
    adj: &std::collections::HashMap<&str, Vec<&'a str>>,
    color: &mut std::collections::HashMap<&'a str, bool>,
) -> bool {
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start);
    while let Some(n) = queue.pop_front() {
        let c = color[n];
        if let Some(neighbors) = adj.get(n) {
            for &next in neighbors {
                if let Some(&nc) = color.get(next) {
                    if nc == c {
                        return false;
                    }
                } else {
                    color.insert(next, !c);
                    queue.push_back(next);
                }
            }
        }
    }
    true
}

/// FJ-799: Find strongly connected components using Tarjan's algorithm.
pub(crate) fn cmd_graph_strongly_connected(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let sccs = tarjan_scc(&cfg);
    let nontrivial: Vec<&Vec<String>> = sccs.iter().filter(|c| c.len() > 1).collect();
    if json {
        let items: Vec<String> = sccs.iter().map(|c| format!("{:?}", c)).collect();
        println!(
            "{{\"strongly_connected_components\":[{}],\"total\":{},\"nontrivial\":{}}}",
            items.join(","),
            sccs.len(),
            nontrivial.len()
        );
    } else if sccs.is_empty() {
        println!("No resources (empty graph).");
    } else {
        println!(
            "Strongly connected components ({}, {} nontrivial):",
            sccs.len(),
            nontrivial.len()
        );
        for (i, comp) in sccs.iter().enumerate() {
            let marker = if comp.len() > 1 { " [CYCLE]" } else { "" };
            println!(
                "  SCC {} ({} nodes{}): {}",
                i + 1,
                comp.len(),
                marker,
                comp.join(", ")
            );
        }
    }
    Ok(())
}

/// Mutable state for Tarjan's SCC algorithm.
struct TarjanState<'a> {
    counter: usize,
    indices: Vec<usize>,
    lowlinks: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    result: Vec<Vec<String>>,
    names: &'a [&'a str],
}

/// Tarjan's SCC algorithm — recursive with state struct.
fn tarjan_scc(cfg: &types::ForjarConfig) -> Vec<Vec<String>> {
    let names: Vec<&str> = cfg.resources.keys().map(|k| k.as_str()).collect();
    let idx_map: std::collections::HashMap<&str, usize> =
        names.iter().enumerate().map(|(i, &n)| (n, i)).collect();
    let adj = build_directed_adj(cfg, &idx_map);
    let n = names.len();
    let mut st = TarjanState {
        counter: 0,
        indices: vec![usize::MAX; n],
        lowlinks: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        result: Vec::new(),
        names: &names,
    };
    for i in 0..n {
        if st.indices[i] == usize::MAX {
            tarjan_visit(i, &adj, &mut st);
        }
    }
    st.result.iter_mut().for_each(|c| c.sort());
    st.result.sort_by(|a, b| a[0].cmp(&b[0]));
    st.result
}

/// Build directed adjacency list (node index → vec of neighbor indices).
fn build_directed_adj(
    cfg: &types::ForjarConfig,
    idx: &std::collections::HashMap<&str, usize>,
) -> Vec<Vec<usize>> {
    let n = idx.len();
    let mut adj = vec![Vec::new(); n];
    for (name, resource) in &cfg.resources {
        if let Some(&from) = idx.get(name.as_str()) {
            for dep in &resource.depends_on {
                if let Some(&to) = idx.get(dep.as_str()) {
                    adj[from].push(to);
                }
            }
        }
    }
    adj
}

/// Recursive Tarjan visit for a single node.
fn tarjan_visit(v: usize, adj: &[Vec<usize>], st: &mut TarjanState<'_>) {
    st.indices[v] = st.counter;
    st.lowlinks[v] = st.counter;
    st.counter += 1;
    st.stack.push(v);
    st.on_stack[v] = true;
    for &w in &adj[v] {
        if st.indices[w] == usize::MAX {
            tarjan_visit(w, adj, st);
            st.lowlinks[v] = st.lowlinks[v].min(st.lowlinks[w]);
        } else if st.on_stack[w] {
            st.lowlinks[v] = st.lowlinks[v].min(st.indices[w]);
        }
    }
    if st.lowlinks[v] == st.indices[v] {
        let mut comp = Vec::new();
        while let Some(w) = st.stack.pop() {
            st.on_stack[w] = false;
            comp.push(st.names[w].to_string());
            if w == v {
                break;
            }
        }
        st.result.push(comp);
    }
}

/// FJ-803: Export dependency graph as CSV adjacency matrix.
pub(crate) fn cmd_graph_dependency_matrix_csv(file: &Path, json: bool) -> Result<(), String> {
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
        let labels: Vec<String> = names.iter().map(|n| format!("\"{}\"", n)).collect();
        println!(
            "{{\"labels\":[{}],\"matrix\":[{}]}}",
            labels.join(","),
            rows.join(",")
        );
    } else if names.is_empty() {
        println!("No resources (empty graph).");
    } else {
        // CSV header
        print!(",");
        println!("{}", names.join(","));
        // CSV rows
        for (i, name) in names.iter().enumerate() {
            print!("{}", name);
            for j in 0..names.len() {
                print!(",{}", if matrix[i][j] { 1 } else { 0 });
            }
            println!();
        }
    }
    Ok(())
}

/// FJ-807: Assign weights to edges by dependency criticality.
pub(crate) fn cmd_graph_resource_weight(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let in_deg_vec = compute_in_degrees(&config);
    let in_deg: std::collections::HashMap<&str, usize> =
        in_deg_vec.iter().map(|(n, d)| (n.as_str(), *d)).collect();
    let mut weights: Vec<(&str, &str, usize)> = Vec::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            let dep_fan = in_deg.get(dep.as_str()).copied().unwrap_or(0);
            let w = dep_fan + 1;
            weights.push((name.as_str(), dep.as_str(), w));
        }
    }
    weights.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(b.0)));
    if json {
        let items: Vec<String> = weights
            .iter()
            .map(|(from, to, w)| {
                format!(
                    "{{\"from\":\"{}\",\"to\":\"{}\",\"weight\":{}}}",
                    from, to, w
                )
            })
            .collect();
        println!("{{\"weighted_edges\":[{}]}}", items.join(","));
    } else if weights.is_empty() {
        println!("No dependency edges.");
    } else {
        println!("Weighted dependency edges ({}):", weights.len());
        for (from, to, w) in &weights {
            println!("  {} -> {} (weight: {})", from, to, w);
        }
    }
    Ok(())
}

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
            .map(|(r, d)| format!("{{\"resource\":\"{}\",\"depth\":{}}}", r, d))
            .collect();
        println!("{{\"dependency_depths\":[{}]}}", items.join(","));
    } else if depths.is_empty() {
        println!("No resources.");
    } else {
        println!("Dependency depth per resource:");
        for (r, d) in &depths {
            println!("  {} — depth {}", r, d);
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
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
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
            .map(|(r, c)| format!("{{\"resource\":\"{}\",\"fanin\":{}}}", r, c))
            .collect();
        println!("{{\"resource_fanin\":[{}]}}", items.join(","));
    } else if sorted.is_empty() {
        println!("No resources.");
    } else {
        println!("Fan-in per resource:");
        for (r, c) in &sorted {
            println!("  {} — {} dependents", r, c);
        }
    }
    Ok(())
}

/// FJ-819: Detect disconnected subgraphs in the DAG.
pub(crate) fn cmd_graph_isolated_subgraphs(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let components = find_connected_components(&config);
    if json {
        let items: Vec<String> = components
            .iter()
            .map(|c| {
                let members: Vec<String> = c.iter().map(|s| format!("\"{}\"", s)).collect();
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
            .map(|(n, l)| format!("{{\"resource\":\"{}\",\"critical_path_length\":{}}}", n, l))
            .collect();
        println!("{{\"critical_path_lengths\":[{}]}}", items.join(","));
    } else if paths.is_empty() {
        println!("No resources to analyze.");
    } else {
        println!("Critical path lengths (longest chain to root):");
        for (n, l) in &paths {
            println!("  {} — {}", n, l);
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
            .map(|(n, s)| format!("{{\"resource\":\"{}\",\"redundancy_score\":{:.2}}}", n, s))
            .collect();
        println!("{{\"redundancy_scores\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No resources to analyze.");
    } else {
        println!("Redundancy scores (higher = more redundant paths):");
        for (n, s) in &scores {
            println!("  {} — {:.2}", n, s);
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
