use crate::core::types;
use std::path::Path;

/// FJ-967: Depth of each resource in topological ordering.
pub(crate) fn cmd_graph_resource_dependency_topological_depth(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut depths: Vec<(String, usize)> = config
        .resources
        .keys()
        .map(|name| {
            let d = topo_depth(&config, name, &mut std::collections::HashMap::new());
            (name.clone(), d)
        })
        .collect();
    depths.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let max = depths.first().map(|(_, d)| *d).unwrap_or(0);
    if json {
        let items: Vec<String> = depths
            .iter()
            .map(|(n, d)| format!("{{\"resource\":\"{n}\",\"depth\":{d}}}"))
            .collect();
        println!(
            "{{\"max_depth\":{},\"resources\":[{}]}}",
            max,
            items.join(",")
        );
    } else if depths.is_empty() {
        println!("No resources found.");
    } else {
        println!("Topological depth (max: {max}):");
        for (n, d) in &depths {
            println!("  {n} — depth {d}");
        }
    }
    Ok(())
}

fn topo_depth(
    config: &types::ForjarConfig,
    name: &str,
    cache: &mut std::collections::HashMap<String, usize>,
) -> usize {
    if let Some(&d) = cache.get(name) {
        return d;
    }
    let res = match config.resources.get(name) {
        Some(r) => r,
        None => return 0,
    };
    if res.depends_on.is_empty() {
        cache.insert(name.to_string(), 0);
        return 0;
    }
    let max_dep = res
        .depends_on
        .iter()
        .map(|dep| topo_depth(config, dep, cache))
        .max()
        .unwrap_or(0);
    let depth = max_dep + 1;
    cache.insert(name.to_string(), depth);
    depth
}

/// FJ-971: Identify dependency edges most likely to cause cascading failures.
pub(crate) fn cmd_graph_resource_dependency_weak_links(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut in_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for res in config.resources.values() {
        for dep in &res.depends_on {
            *in_counts.entry(dep.as_str()).or_insert(0) += 1;
        }
    }
    let mut weak_links: Vec<(String, String, usize)> = Vec::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            let dependents = in_counts.get(dep.as_str()).copied().unwrap_or(0);
            if dependents > 1 {
                weak_links.push((name.clone(), dep.clone(), dependents));
            }
        }
    }
    weak_links.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    if json {
        let items: Vec<String> = weak_links
            .iter()
            .map(|(from, to, d)| {
                format!(
                    "{{\"from\":\"{from}\",\"to\":\"{to}\",\"dependents\":{d}}}"
                )
            })
            .collect();
        println!("{{\"weak_links\":[{}]}}", items.join(","));
    } else if weak_links.is_empty() {
        println!("No weak links found (no shared dependencies).");
    } else {
        println!("Weak links (shared dependencies, cascading risk):");
        for (from, to, d) in &weak_links {
            println!("  {from} → {to} ({d} dependents)");
        }
    }
    Ok(())
}

/// FJ-975: Find minimum edge cut set that disconnects the dependency graph.
pub(crate) fn cmd_graph_resource_dependency_minimum_cut(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<&str> = config.resources.keys().map(|s| s.as_str()).collect();
    let idx: std::collections::HashMap<&str, usize> =
        names.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let n = names.len();
    let (adj, edge_count) = build_undirected_adj(&config, &idx, n);
    let bridges = find_bridge_edges(&config, &idx, &adj, n);
    if json {
        let items: Vec<String> = bridges
            .iter()
            .map(|(a, b)| format!("{{\"from\":\"{a}\",\"to\":\"{b}\"}}"))
            .collect();
        println!(
            "{{\"minimum_cut_edges\":[{}],\"total_edges\":{}}}",
            items.join(","),
            edge_count
        );
    } else if bridges.is_empty() {
        println!("No bridge edges found — graph has no single-edge cut points.");
    } else {
        println!("Minimum cut edges (bridges):");
        for (a, b) in &bridges {
            println!("  {a} → {b}");
        }
    }
    Ok(())
}

fn build_undirected_adj(
    config: &types::ForjarConfig,
    idx: &std::collections::HashMap<&str, usize>,
    n: usize,
) -> (Vec<Vec<bool>>, usize) {
    let mut adj = vec![vec![false; n]; n];
    let mut edge_count = 0usize;
    for (name, res) in &config.resources {
        let u = match idx.get(name.as_str()) {
            Some(&u) => u,
            None => continue,
        };
        for dep in &res.depends_on {
            let v = match idx.get(dep.as_str()) {
                Some(&v) => v,
                None => continue,
            };
            if !adj[u][v] {
                edge_count += 1;
            }
            adj[u][v] = true;
            adj[v][u] = true;
        }
    }
    (adj, edge_count)
}

fn find_bridge_edges(
    config: &types::ForjarConfig,
    idx: &std::collections::HashMap<&str, usize>,
    adj: &[Vec<bool>],
    n: usize,
) -> Vec<(String, String)> {
    let baseline = count_components(adj, n);
    let mut bridges = Vec::new();
    for (name, res) in &config.resources {
        let u = match idx.get(name.as_str()) {
            Some(&u) => u,
            None => continue,
        };
        for dep in &res.depends_on {
            let v = match idx.get(dep.as_str()) {
                Some(&v) => v,
                None => continue,
            };
            let mut adj_copy = adj.to_vec();
            adj_copy[u][v] = false;
            adj_copy[v][u] = false;
            if count_components(&adj_copy, n) > baseline {
                bridges.push((name.clone(), dep.clone()));
            }
        }
    }
    bridges
}
fn count_components(adj: &[Vec<bool>], n: usize) -> usize {
    let mut visited = vec![false; n];
    let mut components = 0;
    for i in 0..n {
        if !visited[i] {
            components += 1;
            let mut stack = vec![i];
            while let Some(node) = stack.pop() {
                if visited[node] {
                    continue;
                }
                visited[node] = true;
                for (j, &is_adj) in adj[node].iter().enumerate() {
                    if is_adj && !visited[j] {
                        stack.push(j);
                    }
                }
            }
        }
    }
    components
}
/// FJ-979: Compute dominator tree showing single points of failure.
pub(crate) fn cmd_graph_resource_dependency_dominator_tree(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let idx: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let n = names.len();
    let children = build_children_adj(&config, &idx, n);
    let roots = find_roots(&config, &names, n);
    let dom_list = compute_dominator_counts(&names, &children, &roots, n);
    if json {
        let items: Vec<String> = dom_list
            .iter()
            .map(|(name, d)| format!("{{\"resource\":\"{name}\",\"dominates\":{d}}}"))
            .collect();
        println!("{{\"dominator_tree\":[{}]}}", items.join(","));
    } else if dom_list.is_empty() {
        println!("No dominators found (no single points of failure).");
    } else {
        println!("Dominator tree (single points of failure):");
        for (name, d) in &dom_list {
            println!("  {name} — dominates {d} resources");
        }
    }
    Ok(())
}

fn build_children_adj(
    config: &types::ForjarConfig,
    idx: &std::collections::HashMap<&str, usize>,
    n: usize,
) -> Vec<Vec<usize>> {
    let mut children: Vec<Vec<usize>> = vec![vec![]; n];
    for (name, res) in &config.resources {
        let to = match idx.get(name.as_str()) {
            Some(&t) => t,
            None => continue,
        };
        for dep in &res.depends_on {
            let from = match idx.get(dep.as_str()) {
                Some(&f) => f,
                None => continue,
            };
            children[from].push(to);
        }
    }
    children
}

fn find_roots(config: &types::ForjarConfig, names: &[String], n: usize) -> Vec<usize> {
    (0..n)
        .filter(|&i| config.resources[&names[i]].depends_on.is_empty())
        .collect()
}

fn compute_dominator_counts(
    names: &[String],
    children: &[Vec<usize>],
    roots: &[usize],
    n: usize,
) -> Vec<(String, usize)> {
    let mut dominates: Vec<usize> = vec![0; n];
    for &root in roots {
        let reachable = reachable_from(root, children, n);
        for (node, &is_reachable) in reachable.iter().enumerate() {
            if node == root || !is_reachable {
                continue;
            }
            let still_reachable = roots
                .iter()
                .any(|&r| r != root && reachable_without(r, root, children, n, node));
            if !still_reachable {
                dominates[root] += 1;
            }
        }
    }
    let mut dom_list: Vec<(String, usize)> = names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), dominates[i]))
        .filter(|(_, d)| *d > 0)
        .collect();
    dom_list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    dom_list
}
fn reachable_from(start: usize, children: &[Vec<usize>], n: usize) -> Vec<bool> {
    let mut visited = vec![false; n];
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if visited[node] {
            continue;
        }
        visited[node] = true;
        for &child in &children[node] {
            stack.push(child);
        }
    }
    visited
}
fn reachable_without(
    start: usize,
    exclude: usize,
    children: &[Vec<usize>],
    n: usize,
    target: usize,
) -> bool {
    let mut visited = vec![false; n];
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if node == exclude || visited[node] {
            continue;
        }
        visited[node] = true;
        if node == target {
            return true;
        }
        for &child in &children[node] {
            stack.push(child);
        }
    }
    false
}
