use super::graph_intelligence_ext2::*;
use crate::core::types;
use std::path::Path;

pub(super) fn bfs_reachable(out: &[Vec<usize>], start: usize, n: usize) -> usize {
    let mut visited = vec![false; n];
    visited[start] = true;
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start);
    let mut count = 0;
    while let Some(u) = queue.pop_front() {
        for &v in &out[u] {
            if !visited[v] {
                visited[v] = true;
                count += 1;
                queue.push_back(v);
            }
        }
    }
    count
}
pub(super) fn print_closure_sizes(sizes: &[(String, usize)], json: bool) {
    if json {
        let items: Vec<String> = sizes
            .iter()
            .map(|(n, s)| format!("{{\"resource\":\"{n}\",\"closure_size\":{s}}}"))
            .collect();
        println!("{{\"closure_sizes\":[{}]}}", items.join(","));
    } else if sizes.is_empty() {
        println!("No resources found.");
    } else {
        println!("Transitive closure sizes:");
        for (n, s) in sizes {
            println!("  {n} — {s} reachable");
        }
    }
}
/// FJ-999: Compute graph eccentricity for each resource node.
pub(crate) fn cmd_graph_resource_dependency_eccentricity_map(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let n = names.len();
    if n == 0 {
        if json {
            println!("{{\"eccentricities\":[]}}");
        } else {
            println!("No resources found.");
        }
        return Ok(());
    }
    let idx: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
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
            if dist[v] == usize::MAX {
                dist[v] = dist[u] + 1;
                queue.push_back(v);
            }
        }
    }
    dist.iter()
        .filter(|&&d| d != usize::MAX)
        .copied()
        .max()
        .unwrap_or(0)
}
fn print_eccentricities(eccs: &[(String, usize)], json: bool) {
    if json {
        let items: Vec<String> = eccs
            .iter()
            .map(|(n, e)| format!("{{\"resource\":\"{n}\",\"eccentricity\":{e}}}"))
            .collect();
        println!("{{\"eccentricities\":[{}]}}", items.join(","));
    } else {
        println!("Eccentricity map (max distance from each node):");
        for (n, e) in eccs {
            println!("  {n} — {e}");
        }
    }
}
/// FJ-1003: Find and display the diameter path.
pub(crate) fn cmd_graph_resource_dependency_diameter_path(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let n = names.len();
    if n == 0 {
        if json {
            println!("{{\"diameter\":0,\"path\":[]}}");
        } else {
            println!("No resources found.");
        }
        return Ok(());
    }
    let idx: std::collections::HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
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
            if dist[v] == usize::MAX {
                dist[v] = dist[u] + 1;
                prev[v] = u;
                queue.push_back(v);
            }
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
        let items: Vec<String> = path.iter().map(|n| format!("\"{n}\"")).collect();
        println!(
            "{{\"diameter\":{},\"path\":[{}]}}",
            diameter,
            items.join(",")
        );
    } else if path.is_empty() {
        println!("Graph diameter: 0 (no edges)");
    } else {
        println!("Graph diameter: {} (path: {})", diameter, path.join(" → "));
    }
}
/// FJ-1015: Score bridge edges by downstream subtree size.
pub(crate) fn cmd_graph_resource_dependency_bridge_criticality(
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
    let (adj, _) = build_adj(&config, &idx, n);
    let bridges = find_bridge_criticality(&names, &adj, n);
    print_bridge_criticality(&bridges, json);
    Ok(())
}
fn find_bridge_criticality(
    names: &[&str],
    adj: &[Vec<bool>],
    n: usize,
) -> Vec<(String, String, usize)> {
    let mut result = Vec::new();
    for i in 0..n {
        for j in 0..n {
            if !adj[i][j] {
                continue;
            }
            // Count downstream nodes reachable from j
            let mut visited = vec![false; n];
            let mut stack = vec![j];
            let mut downstream = 0;
            while let Some(node) = stack.pop() {
                if visited[node] {
                    continue;
                }
                visited[node] = true;
                downstream += 1;
                for (k, visited_k) in visited.iter().enumerate().take(n) {
                    if adj[node][k] && !visited_k {
                        stack.push(k);
                    }
                }
            }
            result.push((names[i].to_string(), names[j].to_string(), downstream));
        }
    }
    result.sort_by(|a, b| b.2.cmp(&a.2));
    result
}
fn print_bridge_criticality(bridges: &[(String, String, usize)], json: bool) {
    if json {
        let items: Vec<String> = bridges
            .iter()
            .map(|(f, t, d)| {
                format!(
                    "{{\"from\":\"{f}\",\"to\":\"{t}\",\"downstream\":{d}}}"
                )
            })
            .collect();
        println!("{{\"bridge_criticality\":[{}]}}", items.join(","));
    } else if bridges.is_empty() {
        println!("No edges found.");
    } else {
        println!("Bridge criticality (downstream subtree size):");
        for (f, t, d) in bridges {
            println!("  {f} → {t} — {d} downstream");
        }
    }
}
/// FJ-1019: Visualize conditional vs unconditional resource subgraphs.
pub(crate) fn cmd_graph_resource_dependency_conditional_subgraph(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut conditional: Vec<(String, String)> = Vec::new();
    let mut unconditional: Vec<String> = Vec::new();
    for (name, res) in &config.resources {
        if let Some(ref when_expr) = res.when {
            conditional.push((name.clone(), when_expr.clone()));
        } else {
            unconditional.push(name.clone());
        }
    }
    conditional.sort_by(|a, b| a.0.cmp(&b.0));
    unconditional.sort();
    print_conditional_subgraph(&conditional, &unconditional, json);
    Ok(())
}
fn print_conditional_subgraph(
    conditional: &[(String, String)],
    unconditional: &[String],
    json: bool,
) {
    if json {
        let cond: Vec<String> = conditional
            .iter()
            .map(|(n, w)| {
                format!(
                    "{{\"resource\":\"{}\",\"when\":\"{}\"}}",
                    n,
                    w.replace('"', "\\\"")
                )
            })
            .collect();
        let uncond: Vec<String> = unconditional.iter().map(|n| format!("\"{n}\"")).collect();
        println!(
            "{{\"conditional\":[{}],\"unconditional\":[{}]}}",
            cond.join(","),
            uncond.join(",")
        );
    } else {
        println!("Conditional resources ({}):", conditional.len());
        for (n, w) in conditional {
            println!("  {n} — when: {w}");
        }
        println!("Unconditional resources ({}):", unconditional.len());
        for n in unconditional {
            println!("  {n}");
        }
    }
}
