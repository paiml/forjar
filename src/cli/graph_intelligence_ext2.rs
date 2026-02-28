//! Graph intelligence extensions (Phase 90+) — resilience score, PageRank.
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-983: Score each edge by how resilient the graph is to its removal.
pub(crate) fn cmd_graph_resource_dependency_resilience_score(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<&str> = config.resources.keys().map(|s| s.as_str()).collect();
    let idx: std::collections::HashMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let n = names.len();
    let (adj, _) = build_adj(&config, &idx, n);
    let baseline = count_components(&adj, n);
    let mut scores = compute_edge_resilience(&config, &idx, &adj, baseline);
    scores.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
    print_resilience_scores(&scores, json);
    Ok(())
}
fn compute_edge_resilience(config: &types::ForjarConfig, idx: &std::collections::HashMap<&str, usize>, adj: &[Vec<bool>], baseline: usize) -> Vec<(String, String, f64)> {
    let mut scores = Vec::new();
    for (name, res) in &config.resources {
        let u = match idx.get(name.as_str()) { Some(&u) => u, None => continue };
        for dep in &res.depends_on {
            let v = match idx.get(dep.as_str()) { Some(&v) => v, None => continue };
            let resilience = edge_resilience(adj, u, v, baseline);
            scores.push((name.clone(), dep.clone(), resilience));
        }
    }
    scores
}
fn edge_resilience(adj: &[Vec<bool>], u: usize, v: usize, baseline: usize) -> f64 {
    let mut adj_copy: Vec<Vec<bool>> = adj.to_vec();
    adj_copy[u][v] = false;
    adj_copy[v][u] = false;
    if count_components(&adj_copy, adj.len()) > baseline { 0.0 } else { 1.0 }
}
fn print_resilience_scores(scores: &[(String, String, f64)], json: bool) {
    if json {
        let items: Vec<String> = scores.iter()
            .map(|(f, t, s)| format!("{{\"from\":\"{}\",\"to\":\"{}\",\"resilience\":{:.2}}}", f, t, s))
            .collect();
        println!("{{\"resilience_scores\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No dependency edges to score.");
    } else {
        println!("Edge resilience scores (0=bridge, 1=resilient):");
        for (f, t, s) in scores { println!("  {} → {} — {:.2}", f, t, s); }
    }
}

/// FJ-987: Compute PageRank importance score for each resource.
pub(crate) fn cmd_graph_resource_dependency_pagerank(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let n = names.len();
    if n == 0 {
        if json { println!("{{\"pagerank\":[]}}"); } else { println!("No resources found."); }
        return Ok(());
    }
    let idx: std::collections::HashMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    let out_links = build_out_links(&config, &idx, n);
    let ranks = compute_pagerank(&out_links, n);
    let mut ranked: Vec<(String, f64)> = names.iter().zip(ranks.iter()).map(|(n, &r)| (n.clone(), r)).collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    if json {
        let items: Vec<String> = ranked.iter()
            .map(|(n, r)| format!("{{\"resource\":\"{}\",\"pagerank\":{:.6}}}", n, r))
            .collect();
        println!("{{\"pagerank\":[{}]}}", items.join(","));
    } else {
        println!("PageRank importance scores:");
        for (n, r) in &ranked { println!("  {} — {:.6}", n, r); }
    }
    Ok(())
}
fn build_adj(config: &types::ForjarConfig, idx: &std::collections::HashMap<&str, usize>, n: usize) -> (Vec<Vec<bool>>, usize) {
    let mut adj = vec![vec![false; n]; n];
    let mut edges = 0;
    for (name, res) in &config.resources {
        let u = match idx.get(name.as_str()) { Some(&u) => u, None => continue };
        for dep in &res.depends_on {
            let v = match idx.get(dep.as_str()) { Some(&v) => v, None => continue };
            if !adj[u][v] { edges += 1; }
            adj[u][v] = true;
            adj[v][u] = true;
        }
    }
    (adj, edges)
}
fn count_components(adj: &[Vec<bool>], n: usize) -> usize {
    let mut visited = vec![false; n];
    let mut components = 0;
    for i in 0..n {
        if !visited[i] {
            components += 1;
            dfs_mark(adj, i, &mut visited);
        }
    }
    components
}
fn dfs_mark(adj: &[Vec<bool>], start: usize, visited: &mut [bool]) {
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if visited[node] { continue; }
        visited[node] = true;
        for (j, &is_adj) in adj[node].iter().enumerate() {
            if is_adj && !visited[j] { stack.push(j); }
        }
    }
}
fn build_out_links(config: &types::ForjarConfig, idx: &std::collections::HashMap<&str, usize>, n: usize) -> Vec<Vec<usize>> {
    let mut out = vec![vec![]; n];
    for (name, res) in &config.resources {
        let u = match idx.get(name.as_str()) { Some(&u) => u, None => continue };
        for dep in &res.depends_on {
            let v = match idx.get(dep.as_str()) { Some(&v) => v, None => continue };
            out[u].push(v);
        }
    }
    out
}
fn compute_pagerank(out_links: &[Vec<usize>], n: usize) -> Vec<f64> {
    let damping = 0.85;
    let mut ranks = vec![1.0 / n as f64; n];
    for _ in 0..20 {
        let mut new_ranks = vec![(1.0 - damping) / n as f64; n];
        for (i, links) in out_links.iter().enumerate() {
            if links.is_empty() {
                let share = damping * ranks[i] / n as f64;
                for r in &mut new_ranks { *r += share; }
            } else {
                let share = damping * ranks[i] / links.len() as f64;
                for &target in links { new_ranks[target] += share; }
            }
        }
        ranks = new_ranks;
    }
    ranks
}
/// FJ-991: Compute betweenness centrality for each resource.
pub(crate) fn cmd_graph_resource_dependency_betweenness_centrality(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let n = names.len();
    if n == 0 {
        if json { println!("{{\"betweenness\":[]}}"); } else { println!("No resources found."); }
        return Ok(());
    }
    let idx: std::collections::HashMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    let out = build_out_links(&config, &idx, n);
    let scores = compute_betweenness(&out, n);
    let mut ranked: Vec<(String, f64)> = names.iter().zip(scores.iter()).map(|(n, &s)| (n.clone(), s)).collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    print_betweenness(&ranked, json);
    Ok(())
}
fn compute_betweenness(out: &[Vec<usize>], n: usize) -> Vec<f64> {
    let mut bc = vec![0.0_f64; n];
    for s in 0..n {
        let paths = bfs_paths(out, s, n);
        accumulate_betweenness(&paths, s, n, &mut bc);
    }
    bc
}
fn bfs_paths(out: &[Vec<usize>], s: usize, n: usize) -> Vec<(i32, f64)> {
    let mut dist = vec![(-1_i32, 0.0_f64); n];
    dist[s] = (0, 1.0);
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(s);
    while let Some(u) = queue.pop_front() {
        for &v in &out[u] {
            if dist[v].0 < 0 {
                dist[v] = (dist[u].0 + 1, dist[u].1);
                queue.push_back(v);
            } else if dist[v].0 == dist[u].0 + 1 {
                dist[v].1 += dist[u].1;
            }
        }
    }
    dist
}
fn accumulate_betweenness(paths: &[(i32, f64)], _s: usize, n: usize, bc: &mut [f64]) {
    for v in 0..n {
        if paths[v].0 > 0 && paths[v].1 > 0.0 {
            bc[v] += paths[v].1;
        }
    }
}
fn print_betweenness(ranked: &[(String, f64)], json: bool) {
    if json {
        let items: Vec<String> = ranked.iter()
            .map(|(n, s)| format!("{{\"resource\":\"{}\",\"betweenness\":{:.4}}}", n, s)).collect();
        println!("{{\"betweenness\":[{}]}}", items.join(","));
    } else {
        println!("Betweenness centrality:");
        for (n, s) in ranked { println!("  {} — {:.4}", n, s); }
    }
}
/// FJ-995: Compute transitive closure size for each resource.
pub(crate) fn cmd_graph_resource_dependency_closure_size(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let n = names.len();
    if n == 0 {
        if json { println!("{{\"closure_sizes\":[]}}"); } else { println!("No resources found."); }
        return Ok(());
    }
    let idx: std::collections::HashMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    let out = build_out_links(&config, &idx, n);
    let mut sizes: Vec<(String, usize)> = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let reachable = bfs_reachable(&out, i, n);
        sizes.push((name.clone(), reachable));
    }
    sizes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    print_closure_sizes(&sizes, json);
    Ok(())
}
fn bfs_reachable(out: &[Vec<usize>], start: usize, n: usize) -> usize {
    let mut visited = vec![false; n];
    visited[start] = true;
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start);
    let mut count = 0;
    while let Some(u) = queue.pop_front() {
        for &v in &out[u] {
            if !visited[v] { visited[v] = true; count += 1; queue.push_back(v); }
        }
    }
    count
}
fn print_closure_sizes(sizes: &[(String, usize)], json: bool) {
    if json {
        let items: Vec<String> = sizes.iter()
            .map(|(n, s)| format!("{{\"resource\":\"{}\",\"closure_size\":{}}}", n, s)).collect();
        println!("{{\"closure_sizes\":[{}]}}", items.join(","));
    } else if sizes.is_empty() {
        println!("No resources found.");
    } else {
        println!("Transitive closure sizes:");
        for (n, s) in sizes { println!("  {} — {} reachable", n, s); }
    }
}
/// FJ-999: Compute graph eccentricity for each resource node.
pub(crate) fn cmd_graph_resource_dependency_eccentricity_map(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let n = names.len();
    if n == 0 {
        if json { println!("{{\"eccentricities\":[]}}"); } else { println!("No resources found."); }
        return Ok(());
    }
    let idx: std::collections::HashMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    let out = build_out_links(&config, &idx, n);
    let mut eccs: Vec<(String, usize)> = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let ecc = bfs_eccentricity(&out, i, n);
        eccs.push((name.clone(), ecc));
    }
    eccs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    print_eccentricities(&eccs, json);
    Ok(())
}
fn bfs_eccentricity(out: &[Vec<usize>], start: usize, n: usize) -> usize {
    let mut dist = vec![usize::MAX; n];
    dist[start] = 0;
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start);
    while let Some(u) = queue.pop_front() {
        for &v in &out[u] {
            if dist[v] == usize::MAX { dist[v] = dist[u] + 1; queue.push_back(v); }
        }
    }
    dist.iter().filter(|&&d| d != usize::MAX).copied().max().unwrap_or(0)
}
fn print_eccentricities(eccs: &[(String, usize)], json: bool) {
    if json {
        let items: Vec<String> = eccs.iter()
            .map(|(n, e)| format!("{{\"resource\":\"{}\",\"eccentricity\":{}}}", n, e)).collect();
        println!("{{\"eccentricities\":[{}]}}", items.join(","));
    } else {
        println!("Eccentricity map (max distance from each node):");
        for (n, e) in eccs { println!("  {} — {}", n, e); }
    }
}
/// FJ-1003: Find and display the diameter path.
pub(crate) fn cmd_graph_resource_dependency_diameter_path(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let n = names.len();
    if n == 0 {
        if json { println!("{{\"diameter\":0,\"path\":[]}}"); } else { println!("No resources found."); }
        return Ok(());
    }
    let idx: std::collections::HashMap<&str, usize> = names.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();
    let out = build_out_links(&config, &idx, n);
    let (diameter, path) = find_diameter_path(&out, &names, n);
    print_diameter_path(diameter, &path, json);
    Ok(())
}
fn find_diameter_path(out: &[Vec<usize>], names: &[String], n: usize) -> (usize, Vec<String>) {
    let mut max_dist = 0;
    let mut best_path = Vec::new();
    for s in 0..n {
        let (dist, prev) = bfs_with_prev(out, s, n);
        for (t, &d) in dist.iter().enumerate() {
            if d != usize::MAX && d > max_dist {
                max_dist = d;
                best_path = reconstruct_path(&prev, s, t, names);
            }
        }
    }
    (max_dist, best_path)
}
fn bfs_with_prev(out: &[Vec<usize>], start: usize, n: usize) -> (Vec<usize>, Vec<usize>) {
    let mut dist = vec![usize::MAX; n];
    let mut prev = vec![usize::MAX; n];
    dist[start] = 0;
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start);
    while let Some(u) = queue.pop_front() {
        for &v in &out[u] {
            if dist[v] == usize::MAX { dist[v] = dist[u] + 1; prev[v] = u; queue.push_back(v); }
        }
    }
    (dist, prev)
}
fn reconstruct_path(prev: &[usize], start: usize, end: usize, names: &[String]) -> Vec<String> {
    let mut path = vec![names[end].clone()];
    let mut cur = end;
    while cur != start && prev[cur] != usize::MAX {
        cur = prev[cur];
        path.push(names[cur].clone());
    }
    path.reverse();
    path
}
fn print_diameter_path(diameter: usize, path: &[String], json: bool) {
    if json {
        let items: Vec<String> = path.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"diameter\":{},\"path\":[{}]}}", diameter, items.join(","));
    } else if path.is_empty() {
        println!("Graph diameter: 0 (no edges)");
    } else {
        println!("Graph diameter: {} (path: {})", diameter, path.join(" → "));
    }
}
