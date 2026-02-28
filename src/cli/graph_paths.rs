//! Graph path analysis — dependency chains, bottlenecks, critical paths, histograms.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::collections::HashSet;
use std::path::Path;

/// FJ-823: Full dependency chain from root to leaf for a resource.
pub(crate) fn cmd_graph_resource_dependency_chain(
    file: &Path, target: &str, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    if !config.resources.contains_key(target) {
        return Err(format!("Resource '{}' not found", target));
    }
    let chain = collect_dep_chain(&config, target);
    if json {
        let items: Vec<String> = chain.iter().map(|s| format!("\"{}\"", s)).collect();
        println!("{{\"resource\":\"{}\",\"chain\":[{}]}}", target, items.join(","));
    } else if chain.is_empty() {
        println!("{} has no dependencies.", target);
    } else {
        println!("Dependency chain for {}:", target);
        for (i, dep) in chain.iter().enumerate() { println!("  {} {}", "  ".repeat(i), dep); }
    }
    Ok(())
}

fn collect_dep_chain(config: &types::ForjarConfig, target: &str) -> Vec<String> {
    let mut chain = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = vec![target.to_string()];
    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) { continue; }
        if let Some(resource) = config.resources.get(&current) {
            for dep in &resource.depends_on {
                chain.push(dep.clone());
                stack.push(dep.clone());
            }
        }
    }
    chain.sort();
    chain.dedup();
    chain
}

/// FJ-827: Resources with highest fan-in AND fan-out (bottlenecks).
pub(crate) fn cmd_graph_bottleneck_resources(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let bottlenecks = find_bottlenecks(&config);
    if json {
        let items: Vec<String> = bottlenecks.iter()
            .map(|(r, fi, fo)| format!("{{\"resource\":\"{}\",\"fanin\":{},\"fanout\":{}}}", r, fi, fo))
            .collect();
        println!("{{\"bottleneck_resources\":[{}]}}", items.join(","));
    } else if bottlenecks.is_empty() {
        println!("No bottleneck resources found.");
    } else {
        println!("Bottleneck resources (high fan-in + fan-out):");
        for (r, fi, fo) in &bottlenecks { println!("  {} — fan-in: {}, fan-out: {}", r, fi, fo); }
    }
    Ok(())
}

fn find_bottlenecks(config: &types::ForjarConfig) -> Vec<(String, usize, usize)> {
    let mut fanin: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut fanout: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for (name, resource) in &config.resources {
        fanout.insert(name.as_str(), resource.depends_on.len());
        for dep in &resource.depends_on {
            *fanin.entry(dep.as_str()).or_default() += 1;
        }
    }
    let mut results: Vec<(String, usize, usize)> = config.resources.keys()
        .filter_map(|name| {
            let fi = fanin.get(name.as_str()).copied().unwrap_or(0);
            let fo = fanout.get(name.as_str()).copied().unwrap_or(0);
            if fi > 0 && fo > 0 { Some((name.clone(), fi, fo)) } else { None }
        })
        .collect();
    results.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    results
}

/// FJ-831: Longest weighted path through the DAG (critical dependency path).
pub(crate) fn cmd_graph_critical_dependency_path(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let path = find_critical_dep_path(&config);
    if json {
        let items: Vec<String> = path.iter().map(|s| format!("\"{}\"", s)).collect();
        println!("{{\"critical_dependency_path\":[{}],\"length\":{}}}", items.join(","), path.len());
    } else if path.is_empty() {
        println!("No dependencies (empty or trivial graph).");
    } else {
        println!("Critical dependency path (length {}):", path.len());
        println!("  {}", path.join(" → "));
    }
    Ok(())
}

fn find_critical_dep_path(config: &types::ForjarConfig) -> Vec<String> {
    let names: Vec<&str> = config.resources.keys().map(|s| s.as_str()).collect();
    let idx: std::collections::HashMap<&str, usize> = names.iter()
        .enumerate().map(|(i, n)| (*n, i)).collect();
    let n = names.len();
    let mut adj = vec![vec![]; n];
    for (name, resource) in &config.resources {
        let from = idx[name.as_str()];
        for dep in &resource.depends_on {
            if let Some(&to) = idx.get(dep.as_str()) { adj[from].push(to); }
        }
    }
    let mut best_path: Vec<String> = Vec::new();
    for i in 0..n {
        let path = longest_path_from(i, &adj, &mut vec![false; n]);
        if path.len() > best_path.len() {
            best_path = path.iter().map(|&idx| names[idx].to_string()).collect();
        }
    }
    best_path
}

fn longest_path_from(start: usize, adj: &[Vec<usize>], visited: &mut [bool]) -> Vec<usize> {
    visited[start] = true;
    let mut best = vec![start];
    for &next in &adj[start] {
        if visited[next] { continue; }
        let sub = longest_path_from(next, adj, visited);
        if sub.len() + 1 > best.len() {
            let mut path = vec![start];
            path.extend(sub);
            best = path;
        }
    }
    visited[start] = false;
    best
}

/// FJ-835: Histogram of dependency depths across all resources.
pub(crate) fn cmd_graph_resource_depth_histogram(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let histogram = build_depth_histogram(&config);
    if json {
        let items: Vec<String> = histogram.iter()
            .map(|(d, c)| format!("{{\"depth\":{},\"count\":{}}}", d, c))
            .collect();
        println!("{{\"depth_histogram\":[{}]}}", items.join(","));
    } else if histogram.is_empty() {
        println!("No resources.");
    } else {
        println!("Resource depth histogram:");
        for (d, c) in &histogram {
            let bar = "#".repeat(*c);
            println!("  depth {} — {} {}", d, c, bar);
        }
    }
    Ok(())
}

fn build_depth_histogram(config: &types::ForjarConfig) -> Vec<(usize, usize)> {
    let names: Vec<&str> = config.resources.keys().map(|s| s.as_str()).collect();
    let idx: std::collections::HashMap<&str, usize> = names.iter()
        .enumerate().map(|(i, n)| (*n, i)).collect();
    let n = names.len();
    let mut adj = vec![vec![]; n];
    for (name, resource) in &config.resources {
        let from = idx[name.as_str()];
        for dep in &resource.depends_on {
            if let Some(&to) = idx.get(dep.as_str()) { adj[from].push(to); }
        }
    }
    let mut cache = vec![None; n];
    let mut counts: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for i in 0..n {
        let d = compute_depth(i, &adj, &mut cache);
        *counts.entry(d).or_default() += 1;
    }
    let mut hist: Vec<(usize, usize)> = counts.into_iter().collect();
    hist.sort();
    hist
}

fn compute_depth(node: usize, adj: &[Vec<usize>], cache: &mut [Option<usize>]) -> usize {
    if let Some(d) = cache[node] { return d; }
    cache[node] = Some(0);
    let d = adj[node].iter()
        .map(|&next| 1 + compute_depth(next, adj, cache))
        .max().unwrap_or(0);
    cache[node] = Some(d);
    d
}
