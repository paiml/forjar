//! Graph intelligence extensions — fan-out, fan-in, path count, articulation points.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-943: Maximum outgoing edges per node (fan-out bottleneck).
pub(crate) fn cmd_graph_resource_dependency_fan_out(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut fan_outs: Vec<(String, usize)> = config
        .resources
        .iter()
        .map(|(name, res)| (name.clone(), res.depends_on.len()))
        .collect();
    fan_outs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let max = fan_outs.first().map(|(_, c)| *c).unwrap_or(0);
    if json {
        let items: Vec<String> = fan_outs
            .iter()
            .map(|(n, c)| format!("{{\"resource\":\"{}\",\"fan_out\":{}}}", n, c))
            .collect();
        println!(
            "{{\"max_fan_out\":{},\"resources\":[{}]}}",
            max,
            items.join(",")
        );
    } else if fan_outs.is_empty() {
        println!("No resources found.");
    } else {
        println!("Fan-out analysis (max: {}):", max);
        for (n, c) in &fan_outs {
            println!("  {} — {} outgoing", n, c);
        }
    }
    Ok(())
}
/// FJ-947: Maximum incoming edges per node (convergence point).
pub(crate) fn cmd_graph_resource_dependency_fan_in(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut in_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for name in config.resources.keys() {
        in_counts.insert(name.clone(), 0);
    }
    for res in config.resources.values() {
        for dep in &res.depends_on {
            *in_counts.entry(dep.clone()).or_insert(0) += 1;
        }
    }
    let mut fan_ins: Vec<(String, usize)> = in_counts.into_iter().collect();
    fan_ins.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let max = fan_ins.first().map(|(_, c)| *c).unwrap_or(0);
    if json {
        let items: Vec<String> = fan_ins
            .iter()
            .map(|(n, c)| format!("{{\"resource\":\"{}\",\"fan_in\":{}}}", n, c))
            .collect();
        println!(
            "{{\"max_fan_in\":{},\"resources\":[{}]}}",
            max,
            items.join(",")
        );
    } else if fan_ins.is_empty() {
        println!("No resources found.");
    } else {
        println!("Fan-in analysis (max: {}):", max);
        for (n, c) in &fan_ins {
            println!("  {} — {} incoming", n, c);
        }
    }
    Ok(())
}
/// FJ-951: Count of distinct dependency paths between all pairs.
pub(crate) fn cmd_graph_resource_dependency_path_count(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<&String> = config.resources.keys().collect();
    let n = names.len();
    let mut total_paths = 0usize;
    for i in 0..n {
        for j in 0..n {
            if i != j {
                total_paths += count_paths_between(&config, names[i], names[j]);
            }
        }
    }
    if json {
        println!(
            "{{\"total_dependency_paths\":{},\"nodes\":{}}}",
            total_paths, n
        );
    } else {
        println!("Total dependency paths: {} ({} nodes)", total_paths, n);
    }
    Ok(())
}
fn count_paths_between(config: &types::ForjarConfig, from: &str, to: &str) -> usize {
    if from == to {
        return 1;
    }
    let res = match config.resources.get(from) {
        Some(r) => r,
        None => return 0,
    };
    let mut count = 0;
    for dep in &res.depends_on {
        count += count_paths_between(config, dep, to);
    }
    count
}

/// FJ-955: Identify articulation points whose removal disconnects graph.
pub(crate) fn cmd_graph_resource_dependency_articulation_points(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let n = names.len();
    let mut points = Vec::new();
    let base_components = count_components_undirected(&config, &names, None);
    for i in 0..n {
        let removed = count_components_undirected(&config, &names, Some(&names[i]));
        if removed > base_components {
            points.push(names[i].clone());
        }
    }
    points.sort();
    if json {
        let items: Vec<String> = points.iter().map(|p| format!("\"{}\"", p)).collect();
        println!("{{\"articulation_points\":[{}]}}", items.join(","));
    } else if points.is_empty() {
        println!("No articulation points found.");
    } else {
        println!("Articulation points:");
        for p in &points {
            println!("  {}", p);
        }
    }
    Ok(())
}

fn count_components_undirected(
    config: &types::ForjarConfig,
    names: &[String],
    exclude: Option<&str>,
) -> usize {
    let active: Vec<&String> = names
        .iter()
        .filter(|n| exclude != Some(n.as_str()))
        .collect();
    let mut visited = std::collections::HashSet::new();
    let mut components = 0;
    for name in &active {
        if visited.contains(name.as_str()) {
            continue;
        }
        components += 1;
        flood_fill_component(config, name, exclude, &mut visited);
    }
    components
}

fn flood_fill_component<'a>(
    config: &'a types::ForjarConfig,
    start: &'a str,
    exclude: Option<&str>,
    visited: &mut std::collections::HashSet<&'a str>,
) {
    let mut stack = vec![start];
    while let Some(v) = stack.pop() {
        if visited.contains(v) {
            continue;
        }
        visited.insert(v);
        push_forward_neighbors(config, v, exclude, visited, &mut stack);
        push_reverse_neighbors(config, v, exclude, visited, &mut stack);
    }
}

fn push_forward_neighbors<'a>(
    config: &'a types::ForjarConfig,
    v: &str,
    exclude: Option<&str>,
    visited: &std::collections::HashSet<&str>,
    stack: &mut Vec<&'a str>,
) {
    if let Some(res) = config.resources.get(v) {
        for dep in &res.depends_on {
            if exclude != Some(dep.as_str()) && !visited.contains(dep.as_str()) {
                stack.push(dep);
            }
        }
    }
}

fn push_reverse_neighbors<'a>(
    config: &'a types::ForjarConfig,
    v: &str,
    exclude: Option<&str>,
    visited: &std::collections::HashSet<&str>,
    stack: &mut Vec<&'a str>,
) {
    for (other_name, other_res) in &config.resources {
        if exclude != Some(other_name.as_str())
            && other_res.depends_on.contains(&v.to_string())
            && !visited.contains(other_name.as_str())
        {
            stack.push(other_name);
        }
    }
}

/// FJ-959: Longest dependency path in the DAG (critical chain).
pub(crate) fn cmd_graph_resource_dependency_longest_path(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let mut longest = 0usize;
    let mut longest_path: Vec<String> = Vec::new();
    for name in &names {
        let mut path = Vec::new();
        let depth = dag_longest_from(&config, name, &mut path);
        if depth > longest {
            longest = depth;
            longest_path = path;
        }
    }
    if json {
        let path_items: Vec<String> = longest_path.iter().map(|p| format!("\"{}\"", p)).collect();
        println!(
            "{{\"longest_path_length\":{},\"path\":[{}]}}",
            longest,
            path_items.join(",")
        );
    } else if longest == 0 {
        println!("No dependency paths found.");
    } else {
        println!("Longest dependency path ({} hops):", longest);
        println!("  {}", longest_path.join(" → "));
    }
    Ok(())
}

fn dag_longest_from(config: &types::ForjarConfig, node: &str, path: &mut Vec<String>) -> usize {
    path.push(node.to_string());
    let res = match config.resources.get(node) {
        Some(r) => r,
        None => return 0,
    };
    if res.depends_on.is_empty() {
        return 0;
    }
    let mut max_depth = 0;
    let mut best_path = Vec::new();
    for dep in &res.depends_on {
        let mut sub_path = Vec::new();
        let d = dag_longest_from(config, dep, &mut sub_path);
        if d + 1 > max_depth {
            max_depth = d + 1;
            best_path = sub_path;
        }
    }
    path.extend(best_path);
    max_depth
}

/// FJ-963: Find strongly connected components in dependency graph.
pub(crate) fn cmd_graph_resource_dependency_strongly_connected(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let sccs = tarjan_scc(&config, &names);
    let non_trivial: Vec<&Vec<String>> = sccs.iter().filter(|c| c.len() > 1).collect();
    if json {
        let items: Vec<String> = non_trivial
            .iter()
            .map(|c| {
                let members: Vec<String> = c.iter().map(|n| format!("\"{}\"", n)).collect();
                format!("[{}]", members.join(","))
            })
            .collect();
        println!(
            "{{\"strongly_connected_components\":[{}],\"count\":{}}}",
            items.join(","),
            non_trivial.len()
        );
    } else if non_trivial.is_empty() {
        println!("No strongly connected components found (DAG is acyclic).");
    } else {
        println!("Strongly connected components:");
        for (i, c) in non_trivial.iter().enumerate() {
            println!("  SCC {}: {}", i + 1, c.join(", "));
        }
    }
    Ok(())
}

fn tarjan_scc(config: &types::ForjarConfig, names: &[String]) -> Vec<Vec<String>> {
    let n = names.len();
    let idx_map: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut index_counter = 0usize;
    let mut stack = Vec::new();
    let mut on_stack = vec![false; n];
    let mut indices = vec![usize::MAX; n];
    let mut lowlinks = vec![0usize; n];
    let mut result = Vec::new();

    #[allow(clippy::too_many_arguments)]
    fn strongconnect(
        v: usize,
        config: &types::ForjarConfig,
        names: &[String],
        idx_map: &std::collections::HashMap<&str, usize>,
        index_counter: &mut usize,
        stack: &mut Vec<usize>,
        on_stack: &mut [bool],
        indices: &mut [usize],
        lowlinks: &mut [usize],
        result: &mut Vec<Vec<String>>,
    ) {
        indices[v] = *index_counter;
        lowlinks[v] = *index_counter;
        *index_counter += 1;
        stack.push(v);
        on_stack[v] = true;

        if let Some(res) = config.resources.get(&names[v]) {
            for dep in &res.depends_on {
                if let Some(&w) = idx_map.get(dep.as_str()) {
                    if indices[w] == usize::MAX {
                        strongconnect(
                            w,
                            config,
                            names,
                            idx_map,
                            index_counter,
                            stack,
                            on_stack,
                            indices,
                            lowlinks,
                            result,
                        );
                        lowlinks[v] = lowlinks[v].min(lowlinks[w]);
                    } else if on_stack[w] {
                        lowlinks[v] = lowlinks[v].min(indices[w]);
                    }
                }
            }
        }

        if lowlinks[v] == indices[v] {
            let mut component = Vec::new();
            while let Some(w) = stack.pop() {
                on_stack[w] = false;
                component.push(names[w].clone());
                if w == v {
                    break;
                }
            }
            component.sort();
            result.push(component);
        }
    }

    for i in 0..n {
        if indices[i] == usize::MAX {
            strongconnect(
                i,
                config,
                names,
                &idx_map,
                &mut index_counter,
                &mut stack,
                &mut on_stack,
                &mut indices,
                &mut lowlinks,
                &mut result,
            );
        }
    }
    result
}

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
            .map(|(n, d)| format!("{{\"resource\":\"{}\",\"depth\":{}}}", n, d))
            .collect();
        println!(
            "{{\"max_depth\":{},\"resources\":[{}]}}",
            max,
            items.join(",")
        );
    } else if depths.is_empty() {
        println!("No resources found.");
    } else {
        println!("Topological depth (max: {}):", max);
        for (n, d) in &depths {
            println!("  {} — depth {}", n, d);
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
                    "{{\"from\":\"{}\",\"to\":\"{}\",\"dependents\":{}}}",
                    from, to, d
                )
            })
            .collect();
        println!("{{\"weak_links\":[{}]}}", items.join(","));
    } else if weak_links.is_empty() {
        println!("No weak links found (no shared dependencies).");
    } else {
        println!("Weak links (shared dependencies, cascading risk):");
        for (from, to, d) in &weak_links {
            println!("  {} → {} ({} dependents)", from, to, d);
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
            .map(|(a, b)| format!("{{\"from\":\"{}\",\"to\":\"{}\"}}", a, b))
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
            println!("  {} → {}", a, b);
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
            .map(|(name, d)| format!("{{\"resource\":\"{}\",\"dominates\":{}}}", name, d))
            .collect();
        println!("{{\"dominator_tree\":[{}]}}", items.join(","));
    } else if dom_list.is_empty() {
        println!("No dominators found (no single points of failure).");
    } else {
        println!("Dominator tree (single points of failure):");
        for (name, d) in &dom_list {
            println!("  {} — dominates {} resources", name, d);
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
