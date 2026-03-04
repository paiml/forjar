use super::graph_intelligence::*;
use crate::core::types;
use std::collections::HashSet;
use std::path::Path;

/// FJ-923: Modularity score for resource dependency communities.
pub(crate) fn cmd_graph_resource_dependency_modularity_score(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let (score, communities) = compute_modularity(&config);
    if json {
        let items: Vec<String> = communities
            .iter()
            .map(|(c, members)| {
                let ms: Vec<String> = members.iter().map(|m| format!("\"{m}\"")).collect();
                format!("{{\"community\":{},\"members\":[{}]}}", c, ms.join(","))
            })
            .collect();
        println!(
            "{{\"modularity_score\":{:.3},\"communities\":[{}]}}",
            score,
            items.join(",")
        );
    } else {
        println!("Modularity score: {score:.3}");
        if communities.is_empty() {
            println!("No communities detected.");
        } else {
            for (c, members) in &communities {
                println!("  Community {} — {}", c, members.join(", "));
            }
        }
    }
    Ok(())
}

fn compute_modularity(config: &types::ForjarConfig) -> (f64, Vec<(usize, Vec<String>)>) {
    let (names, _idx, adj) = build_undirected_index(config);
    let n = names.len();
    if n == 0 {
        return (0.0, vec![]);
    }
    let total_edges: usize = adj.iter().map(|a| a.len()).sum();
    let m = total_edges / 2;
    if m == 0 {
        return (0.0, vec![(0, names)]);
    }
    let community_id = detect_communities(&adj, n);
    let q = modularity_score(&adj, &community_id, m);
    let mut communities: std::collections::HashMap<usize, Vec<String>> =
        std::collections::HashMap::new();
    for i in 0..n {
        communities
            .entry(community_id[i])
            .or_default()
            .push(names[i].clone());
    }
    let mut result: Vec<(usize, Vec<String>)> = communities.into_iter().collect();
    result.sort_by_key(|(c, _)| *c);
    (q, result)
}

fn detect_communities(adj: &[HashSet<usize>], n: usize) -> Vec<usize> {
    let mut visited = vec![false; n];
    let mut community_id = vec![0usize; n];
    let mut cid = 0usize;
    for start in 0..n {
        if visited[start] {
            continue;
        }
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            if visited[node] {
                continue;
            }
            visited[node] = true;
            community_id[node] = cid;
            for &next in &adj[node] {
                if !visited[next] {
                    stack.push(next);
                }
            }
        }
        cid += 1;
    }
    community_id
}

fn modularity_score(adj: &[HashSet<usize>], community_id: &[usize], m: usize) -> f64 {
    let m2 = 2.0 * m as f64;
    let mut q = 0.0f64;
    for (i, cid_i) in community_id.iter().enumerate() {
        for &j in &adj[i] {
            if *cid_i == community_id[j] {
                let ki = adj[i].len() as f64;
                let kj = adj[j].len() as f64;
                q += 1.0 - (ki * kj) / m2;
            }
        }
    }
    q / m2
}

/// FJ-927: Graph diameter — longest shortest path in dependency graph.
pub(crate) fn cmd_graph_resource_dependency_diameter(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let (diameter, eccentricities) = compute_eccentricities(&config);
    if json {
        println!("{{\"diameter\":{diameter}}}");
    } else {
        println!("Graph diameter: {diameter}");
        if !eccentricities.is_empty() {
            println!("Max eccentricity resources:");
            for (n, e) in eccentricities.iter().filter(|(_, e)| *e == diameter) {
                println!("  {n} — eccentricity {e}");
            }
        }
    }
    Ok(())
}

/// FJ-931: Eccentricity (max shortest path) per resource.
pub(crate) fn cmd_graph_resource_dependency_eccentricity(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let (_diameter, eccentricities) = compute_eccentricities(&config);
    if json {
        let items: Vec<String> = eccentricities
            .iter()
            .map(|(n, e)| format!("{{\"resource\":\"{n}\",\"eccentricity\":{e}}}"))
            .collect();
        println!("{{\"eccentricities\":[{}]}}", items.join(","));
    } else if eccentricities.is_empty() {
        println!("No resources to analyze.");
    } else {
        println!("Resource eccentricities:");
        for (n, e) in &eccentricities {
            println!("  {n} — {e}");
        }
    }
    Ok(())
}

fn compute_eccentricities(config: &types::ForjarConfig) -> (usize, Vec<(String, usize)>) {
    let (names, _idx, adj) = build_undirected_index(config);
    let n = names.len();
    if n == 0 {
        return (0, vec![]);
    }
    let mut eccentricities = Vec::new();
    let mut diameter = 0usize;
    for (i, name) in names.iter().enumerate().take(n) {
        let max_dist = bfs_max_distance(i, &adj, n);
        eccentricities.push((name.clone(), max_dist));
        if max_dist > diameter {
            diameter = max_dist;
        }
    }
    eccentricities.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    (diameter, eccentricities)
}

fn bfs_max_distance(start: usize, adj: &[HashSet<usize>], n: usize) -> usize {
    let mut dist = vec![usize::MAX; n];
    dist[start] = 0;
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start);
    let mut max_d = 0;
    while let Some(v) = queue.pop_front() {
        for &w in &adj[v] {
            if dist[w] == usize::MAX {
                dist[w] = dist[v] + 1;
                if dist[w] > max_d {
                    max_d = dist[w];
                }
                queue.push_back(w);
            }
        }
    }
    max_d
}

/// FJ-935: Edge density ratio in dependency graph.
pub(crate) fn cmd_graph_resource_dependency_density(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let n = config.resources.len();
    let edges: usize = config.resources.values().map(|r| r.depends_on.len()).sum();
    let max_edges = if n > 1 { n * (n - 1) } else { 1 };
    let density = edges as f64 / max_edges as f64;
    if json {
        println!(
            "{{\"density\":{density:.4},\"nodes\":{n},\"edges\":{edges}}}"
        );
    } else {
        println!(
            "Graph density: {density:.4} ({n} nodes, {edges} edges)"
        );
    }
    Ok(())
}

/// FJ-939: Transitive reduction ratio for dependency simplification.
pub(crate) fn cmd_graph_resource_dependency_transitivity(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let (total, redundant) = count_transitive_edges(&config);
    let ratio = if total > 0 {
        redundant as f64 / total as f64
    } else {
        0.0
    };
    if json {
        println!(
            "{{\"total_edges\":{total},\"redundant_edges\":{redundant},\"transitivity_ratio\":{ratio:.4}}}"
        );
    } else {
        println!(
            "Transitivity: {redundant}/{total} edges redundant (ratio: {ratio:.4})"
        );
    }
    Ok(())
}

fn count_transitive_edges(config: &types::ForjarConfig) -> (usize, usize) {
    let names: Vec<&str> = config.resources.keys().map(|k| k.as_str()).collect();
    let idx: std::collections::HashMap<&str, usize> =
        names.iter().enumerate().map(|(i, n)| (*n, i)).collect();
    let n = names.len();
    let mut adj = vec![vec![false; n]; n];
    let mut total = 0;
    for (name, res) in &config.resources {
        if let Some(&from) = idx.get(name.as_str()) {
            for dep in &res.depends_on {
                if let Some(&to) = idx.get(dep.as_str()) {
                    adj[from][to] = true;
                    total += 1;
                }
            }
        }
    }
    let redundant = count_redundant_edges(&adj, n);
    (total, redundant)
}

fn count_redundant_edges(adj: &[Vec<bool>], n: usize) -> usize {
    let mut redundant = 0;
    for i in 0..n {
        for j in 0..n {
            if adj[i][j] && is_edge_redundant(adj, i, j, n) {
                redundant += 1;
            }
        }
    }
    redundant
}

fn is_edge_redundant(adj: &[Vec<bool>], from: usize, to: usize, n: usize) -> bool {
    (0..n).any(|k| k != to && adj[from][k] && reachable_without_direct(adj, k, to, n))
}

fn reachable_without_direct(adj: &[Vec<bool>], from: usize, to: usize, n: usize) -> bool {
    let mut visited = vec![false; n];
    let mut stack = vec![from];
    visited[from] = true;
    while let Some(v) = stack.pop() {
        for (w, visited_w) in visited.iter_mut().enumerate().take(n) {
            if adj[v][w] && !*visited_w {
                if w == to {
                    return true;
                }
                *visited_w = true;
                stack.push(w);
            }
        }
    }
    false
}
