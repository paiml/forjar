//! Graph intelligence — centrality, bridge detection, advanced graph analytics.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::collections::HashSet;
use std::path::Path;

/// FJ-911: Betweenness centrality score for critical resources.
pub(crate) fn cmd_graph_resource_dependency_centrality_score(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let scores = compute_centrality_scores(&config);
    if json {
        let items: Vec<String> = scores
            .iter()
            .map(|(n, s)| format!("{{\"resource\":\"{n}\",\"centrality_score\":{s:.3}}}"))
            .collect();
        println!("{{\"centrality_scores\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No resources to analyze.");
    } else {
        println!("Betweenness centrality scores:");
        for (n, s) in &scores {
            println!("  {n} — {s:.3}");
        }
    }
    Ok(())
}

pub(super) fn compute_centrality_scores(config: &types::ForjarConfig) -> Vec<(String, f64)> {
    let names: Vec<&str> = config.resources.keys().map(|k| k.as_str()).collect();
    let n = names.len();
    if n < 2 {
        return names.iter().map(|&n| (n.to_string(), 0.0)).collect();
    }
    let idx: std::collections::HashMap<&str, usize> =
        names.iter().enumerate().map(|(i, &n)| (n, i)).collect();
    let mut adj = vec![vec![]; n];
    for (name, res) in &config.resources {
        if let Some(&from) = idx.get(name.as_str()) {
            for dep in &res.depends_on {
                if let Some(&to) = idx.get(dep.as_str()) {
                    adj[from].push(to);
                }
            }
        }
    }
    let mut centrality = vec![0.0f64; n];
    for s in 0..n {
        let paths = bfs_shortest_paths(s, &adj, n);
        accumulate_centrality(s, &paths, &mut centrality, n);
    }
    let max_c = centrality.iter().cloned().fold(0.0f64, f64::max);
    if max_c > 0.0 {
        centrality.iter_mut().for_each(|c| *c /= max_c);
    }
    let mut result: Vec<(String, f64)> = names
        .iter()
        .enumerate()
        .map(|(i, &nm)| (nm.to_string(), centrality[i]))
        .collect();
    result.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    result
}

pub(super) fn bfs_shortest_paths(
    src: usize,
    adj: &[Vec<usize>],
    n: usize,
) -> Vec<(Vec<usize>, usize)> {
    let mut dist = vec![usize::MAX; n];
    let mut sigma = vec![0usize; n];
    let mut pred: Vec<Vec<usize>> = vec![vec![]; n];
    dist[src] = 0;
    sigma[src] = 1;
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(src);
    while let Some(v) = queue.pop_front() {
        for &w in &adj[v] {
            if dist[w] == usize::MAX {
                dist[w] = dist[v] + 1;
                queue.push_back(w);
            }
            if dist[w] == dist[v] + 1 {
                sigma[w] += sigma[v];
                pred[w].push(v);
            }
        }
    }
    pred.into_iter().zip(sigma).collect()
}

pub(super) fn accumulate_centrality(
    src: usize,
    paths: &[(Vec<usize>, usize)],
    centrality: &mut [f64],
    n: usize,
) {
    let mut delta = vec![0.0f64; n];
    let mut order: Vec<usize> = (0..n).filter(|&i| paths[i].1 > 0).collect();
    order.sort_by(|&a, &b| {
        let da = paths[a].0.first().map(|p| paths[*p].1).unwrap_or(0);
        let db = paths[b].0.first().map(|p| paths[*p].1).unwrap_or(0);
        db.cmp(&da)
    });
    for &w in &order {
        if w == src {
            continue;
        }
        let (ref preds, sigma_w) = paths[w];
        if sigma_w == 0 {
            continue;
        }
        for &v in preds {
            let sigma_v = paths[v].1;
            if sigma_v > 0 {
                delta[v] += (sigma_v as f64 / sigma_w as f64) * (1.0 + delta[w]);
            }
        }
        centrality[w] += delta[w];
    }
}

/// FJ-915: Find bridge edges whose removal disconnects the graph.
pub(crate) fn cmd_graph_resource_dependency_bridge_detection(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let bridges = detect_bridge_edges(&config);
    if json {
        let items: Vec<String> = bridges
            .iter()
            .map(|(a, b)| format!("{{\"from\":\"{a}\",\"to\":\"{b}\"}}"))
            .collect();
        println!(
            "{{\"bridge_edges\":[{}],\"count\":{}}}",
            items.join(","),
            bridges.len()
        );
    } else if bridges.is_empty() {
        println!("No bridge edges detected (graph is well-connected).");
    } else {
        println!("Bridge edges ({}):", bridges.len());
        for (a, b) in &bridges {
            println!("  {a} → {b}");
        }
    }
    Ok(())
}

pub(super) fn build_undirected_index(
    config: &types::ForjarConfig,
) -> (
    Vec<String>,
    std::collections::HashMap<String, usize>,
    Vec<HashSet<usize>>,
) {
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let n = names.len();
    let idx: std::collections::HashMap<String, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.clone(), i))
        .collect();
    let mut adj: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    for (name, res) in &config.resources {
        if let Some(&from) = idx.get(name.as_str()) {
            for dep in &res.depends_on {
                if let Some(&to) = idx.get(dep.as_str()) {
                    adj[from].insert(to);
                    adj[to].insert(from);
                }
            }
        }
    }
    (names, idx, adj)
}

pub(super) fn detect_bridge_edges(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let (names, idx, adj) = build_undirected_index(config);
    let n = names.len();
    let mut bridges = Vec::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if let (Some(&from), Some(&to)) = (idx.get(name.as_str()), idx.get(dep.as_str())) {
                if is_bridge(from, to, &adj, n) {
                    bridges.push((name.clone(), dep.clone()));
                }
            }
        }
    }
    bridges.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    bridges
}

pub(super) fn is_bridge(u: usize, v: usize, adj: &[HashSet<usize>], n: usize) -> bool {
    let before = count_reachable(u, adj, n);
    let mut adj_without: Vec<HashSet<usize>> = adj.to_vec();
    adj_without[u].remove(&v);
    adj_without[v].remove(&u);
    let after = count_reachable(u, &adj_without, n);
    after < before
}

pub(super) fn count_reachable(start: usize, adj: &[HashSet<usize>], n: usize) -> usize {
    let mut visited = vec![false; n];
    let mut stack = vec![start];
    let mut count = 0;
    while let Some(node) = stack.pop() {
        if visited[node] {
            continue;
        }
        visited[node] = true;
        count += 1;
        for &next in &adj[node] {
            if !visited[next] {
                stack.push(next);
            }
        }
    }
    count
}

/// FJ-919: Clustering coefficient per resource in dependency graph.
pub(crate) fn cmd_graph_resource_dependency_cluster_coefficient(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let coefficients = compute_cluster_coefficients(&config);
    if json {
        let items: Vec<String> = coefficients
            .iter()
            .map(|(n, c)| format!("{{\"resource\":\"{n}\",\"cluster_coefficient\":{c:.3}}}"))
            .collect();
        println!("{{\"cluster_coefficients\":[{}]}}", items.join(","));
    } else if coefficients.is_empty() {
        println!("No resources to analyze.");
    } else {
        println!("Clustering coefficients:");
        for (n, c) in &coefficients {
            println!("  {n} — {c:.3}");
        }
    }
    Ok(())
}

pub(super) fn compute_cluster_coefficients(config: &types::ForjarConfig) -> Vec<(String, f64)> {
    let (names, _idx, adj) = build_undirected_index(config);
    let n = names.len();
    let mut result = Vec::new();
    for i in 0..n {
        let neighbors: Vec<usize> = adj[i].iter().copied().collect();
        let k = neighbors.len();
        if k < 2 {
            result.push((names[i].clone(), 0.0));
            continue;
        }
        let mut triangles = 0usize;
        for a in 0..k {
            for b in (a + 1)..k {
                if adj[neighbors[a]].contains(&neighbors[b]) {
                    triangles += 1;
                }
            }
        }
        let possible = k * (k - 1) / 2;
        let cc = if possible > 0 {
            triangles as f64 / possible as f64
        } else {
            0.0
        };
        result.push((names[i].clone(), cc));
    }
    result.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    result
}

pub(super) use super::graph_intelligence_b::*;
