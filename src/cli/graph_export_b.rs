use super::graph_export::*;
use super::helpers::*;
use crate::core::types;
use std::path::Path;

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
pub(super) fn find_longest_chain(cfg: &types::ForjarConfig) -> (usize, Vec<String>) {
    let names: Vec<&str> = cfg.resources.keys().map(|k| k.as_str()).collect();
    let n = names.len();
    if n == 0 {
        return (0, Vec::new());
    }
    let idx: std::collections::HashMap<&str, usize> =
        names.iter().enumerate().map(|(i, &n)| (n, i)).collect();
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
        let items: Vec<String> = degrees
            .iter()
            .map(|(n, d)| format!("{{\"resource\":\"{n}\",\"in_degree\":{d}}}"))
            .collect();
        println!("{{\"in_degrees\":[{}]}}", items.join(","));
    } else if degrees.is_empty() {
        println!("No resources.");
    } else {
        println!("In-degree (dependents) per resource:");
        for (name, deg) in &degrees {
            println!("  {name} — {deg}");
        }
    }
    Ok(())
}

/// Compute in-degree for each resource (how many others depend on it).
pub(crate) fn compute_in_degrees(cfg: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut deg: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for name in cfg.resources.keys() {
        deg.insert(name.clone(), 0);
    }
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
    let mut degrees: Vec<(String, usize)> = cfg
        .resources
        .iter()
        .map(|(n, r)| (n.clone(), r.depends_on.len()))
        .collect();
    degrees.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    if json {
        let items: Vec<String> = degrees
            .iter()
            .map(|(n, d)| format!("{{\"resource\":\"{n}\",\"out_degree\":{d}}}"))
            .collect();
        println!("{{\"out_degrees\":[{}]}}", items.join(","));
    } else if degrees.is_empty() {
        println!("No resources.");
    } else {
        println!("Out-degree (dependencies) per resource:");
        for (name, deg) in &degrees {
            println!("  {name} — {deg}");
        }
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
        println!(
            "{{\"nodes\":{n},\"edges\":{edges},\"max_edges\":{max_edges},\"density\":{density:.4}}}"
        );
    } else {
        println!(
            "Graph density: {density:.4} ({edges} edges / {max_edges} max, {n} nodes)"
        );
    }
    Ok(())
}

/// FJ-783: Output resources in valid topological execution order.
pub(crate) fn cmd_graph_topological_sort(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let order = topological_sort_resources(&cfg);
    if json {
        let items: Vec<String> = order.iter().map(|n| format!("\"{n}\"")).collect();
        println!("{{\"topological_order\":[{}]}}", items.join(","));
    } else if order.is_empty() {
        println!("No resources (empty graph).");
    } else {
        println!("Topological execution order ({} resources):", order.len());
        for (i, name) in order.iter().enumerate() {
            println!("  {}. {}", i + 1, name);
        }
    }
    Ok(())
}

/// Build in-degree map and dependents adjacency for Kahn's algorithm.
fn build_kahn_graph(
    cfg: &types::ForjarConfig,
) -> (
    std::collections::HashMap<&str, usize>,
    std::collections::HashMap<&str, Vec<&str>>,
) {
    let mut in_deg: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut dependents: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for name in cfg.resources.keys() {
        in_deg.insert(name.as_str(), 0);
    }
    for (name, resource) in &cfg.resources {
        for dep in &resource.depends_on {
            if cfg.resources.contains_key(dep) {
                *in_deg.entry(name.as_str()).or_default() += 1;
                dependents
                    .entry(dep.as_str())
                    .or_default()
                    .push(name.as_str());
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
    let mut queue: Vec<&str> = in_deg
        .iter()
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
                    if *deg == 0 {
                        next.push(d);
                    }
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
        let items: Vec<String> = chain.iter().map(|n| format!("\"{n}\"")).collect();
        println!(
            "{{\"critical_path_length\":{},\"resources\":[{}]}}",
            length,
            items.join(",")
        );
    } else if length == 0 {
        println!("No dependency chains (all resources independent).");
    } else {
        println!(
            "Critical path ({} edges, {} resources):",
            length,
            chain.len()
        );
        for (i, name) in chain.iter().enumerate() {
            println!("  {}. {}", i + 1, name);
        }
    }
    Ok(())
}

/// FJ-791: Show sink resources (nothing depends on them).
pub(crate) fn cmd_graph_sink_resources(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let degrees = compute_in_degrees(&cfg);
    let mut sinks: Vec<String> = degrees
        .iter()
        .filter(|(_, d)| *d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    sinks.sort();
    if json {
        let items: Vec<String> = sinks.iter().map(|n| format!("\"{n}\"")).collect();
        println!("{{\"sink_resources\":[{}]}}", items.join(","));
    } else if sinks.is_empty() {
        println!("No sink resources (all have dependents).");
    } else {
        println!("Sink resources ({} with no dependents):", sinks.len());
        for name in &sinks {
            println!("  {name}");
        }
    }
    Ok(())
}

// FJ-795, FJ-799, FJ-803 moved to graph_advanced.rs
