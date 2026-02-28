//! Graph scoring — impact, stability, fanout, weight, bottleneck, clustering.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// FJ-847: Impact score based on dependents + depth.
pub(crate) fn cmd_graph_resource_impact_score(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let scores = compute_impact_scores(&config);
    if json {
        let items: Vec<String> = scores.iter()
            .map(|(r, s)| format!("{{\"resource\":\"{}\",\"impact_score\":{}}}", r, s))
            .collect();
        println!("{{\"resource_impact_scores\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No resources.");
    } else {
        println!("Resource impact scores (dependents + depth):");
        for (r, s) in &scores { println!("  {} — score {}", r, s); }
    }
    Ok(())
}

fn compute_impact_scores(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut fanin: HashMap<&str, usize> = HashMap::new();
    for resource in config.resources.values() {
        for dep in &resource.depends_on {
            *fanin.entry(dep.as_str()).or_default() += 1;
        }
    }
    let mut results: Vec<(String, usize)> = config.resources.keys()
        .map(|name| {
            let dependents = fanin.get(name.as_str()).copied().unwrap_or(0);
            let depth = config.resources[name].depends_on.len();
            (name.clone(), dependents * 2 + depth)
        })
        .collect();
    results.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    results
}

/// FJ-851: Stability score based on status history (simulated from resource config).
pub(crate) fn cmd_graph_resource_stability_score(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let scores = compute_stability_scores(&config);
    if json {
        let items: Vec<String> = scores.iter()
            .map(|(r, s)| format!("{{\"resource\":\"{}\",\"stability_score\":{}}}", r, s))
            .collect();
        println!("{{\"resource_stability_scores\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No resources.");
    } else {
        println!("Resource stability scores (higher = more stable):");
        for (r, s) in &scores { println!("  {} — score {}", r, s); }
    }
    Ok(())
}

fn compute_stability_scores(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut results: Vec<(String, usize)> = config.resources.keys()
        .map(|name| {
            let r = &config.resources[name];
            let mut score = 10usize;
            score = score.saturating_sub(r.depends_on.len());
            if r.state.is_some() { score += 2; }
            if r.content.is_some() { score += 1; }
            (name.clone(), score)
        })
        .collect();
    results.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    results
}

/// FJ-855: Fan-out count per resource.
pub(crate) fn cmd_graph_resource_dependency_fanout(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let fanouts = compute_fanouts(&config);
    if json {
        let items: Vec<String> = fanouts.iter()
            .map(|(r, f)| format!("{{\"resource\":\"{}\",\"fanout\":{}}}", r, f))
            .collect();
        println!("{{\"resource_dependency_fanout\":[{}]}}", items.join(","));
    } else if fanouts.is_empty() {
        println!("No resources.");
    } else {
        println!("Resource dependency fan-out:");
        for (r, f) in &fanouts { println!("  {} — {} dependents", r, f); }
    }
    Ok(())
}

fn compute_fanouts(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut fanin: HashMap<&str, usize> = HashMap::new();
    for resource in config.resources.values() {
        for dep in &resource.depends_on {
            *fanin.entry(dep.as_str()).or_default() += 1;
        }
    }
    let mut results: Vec<(String, usize)> = config.resources.keys()
        .map(|name| {
            let count = fanin.get(name.as_str()).copied().unwrap_or(0);
            (name.clone(), count)
        })
        .collect();
    results.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    results
}

/// FJ-859: Weighted edges based on resource coupling.
pub(crate) fn cmd_graph_resource_dependency_weight(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let weights = compute_dependency_weights(&config);
    if json {
        let items: Vec<String> = weights.iter()
            .map(|(a, b, w)| format!("{{\"from\":\"{}\",\"to\":\"{}\",\"weight\":{}}}", a, b, w))
            .collect();
        println!("{{\"dependency_weights\":[{}]}}", items.join(","));
    } else if weights.is_empty() {
        println!("No dependency edges.");
    } else {
        println!("Dependency edge weights:");
        for (a, b, w) in &weights { println!("  {} → {} — weight {}", a, b, w); }
    }
    Ok(())
}

fn compute_dependency_weights(config: &types::ForjarConfig) -> Vec<(String, String, usize)> {
    let mut edges = Vec::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            let mut weight = 1usize;
            if let Some(dep_resource) = config.resources.get(dep) {
                let ma: HashSet<String> = resource.machine.to_vec().into_iter().collect();
                let mb: HashSet<String> = dep_resource.machine.to_vec().into_iter().collect();
                if !ma.is_disjoint(&mb) { weight += 1; }
            }
            edges.push((name.clone(), dep.clone(), weight));
        }
    }
    edges.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    edges
}

/// FJ-863: Identify bottleneck resources (high fan-in + fan-out).
pub(crate) fn cmd_graph_resource_dependency_bottleneck(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let bottlenecks = find_dependency_bottlenecks(&config);
    if json {
        let items: Vec<String> = bottlenecks.iter()
            .map(|(n, fi, fo)| format!("{{\"resource\":\"{}\",\"fan_in\":{},\"fan_out\":{}}}", n, fi, fo)).collect();
        println!("{{\"bottlenecks\":[{}]}}", items.join(","));
    } else if bottlenecks.is_empty() {
        println!("No dependency bottlenecks found.");
    } else {
        println!("Dependency bottlenecks (fan-in + fan-out):");
        for (n, fi, fo) in &bottlenecks { println!("  {} — fan-in {}, fan-out {}", n, fi, fo); }
    }
    Ok(())
}

fn find_dependency_bottlenecks(config: &types::ForjarConfig) -> Vec<(String, usize, usize)> {
    let mut fan_in: HashMap<String, usize> = HashMap::new();
    let mut fan_out: HashMap<String, usize> = HashMap::new();
    for (name, resource) in &config.resources {
        fan_out.insert(name.clone(), resource.depends_on.len());
        for dep in &resource.depends_on {
            *fan_in.entry(dep.clone()).or_default() += 1;
        }
    }
    let mut results: Vec<(String, usize, usize)> = config.resources.keys()
        .map(|n| (n.clone(), *fan_in.get(n).unwrap_or(&0), *fan_out.get(n).unwrap_or(&0)))
        .filter(|(_, fi, fo)| *fi > 0 || *fo > 0)
        .collect();
    results.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    results
}

/// FJ-867: Cluster resources by type and show interconnections.
pub(crate) fn cmd_graph_resource_type_clustering(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let clusters = build_type_clusters(&config);
    if json {
        let items: Vec<String> = clusters.iter()
            .map(|(t, rs)| {
                let names: Vec<String> = rs.iter().map(|r| format!("\"{}\"", r)).collect();
                format!("{{\"type\":\"{}\",\"resources\":[{}]}}", t, names.join(","))
            }).collect();
        println!("{{\"type_clusters\":[{}]}}", items.join(","));
    } else if clusters.is_empty() {
        println!("No resources to cluster.");
    } else {
        println!("Resource type clusters:");
        for (t, rs) in &clusters { println!("  {} — {} resources: {}", t, rs.len(), rs.join(", ")); }
    }
    Ok(())
}

fn build_type_clusters(config: &types::ForjarConfig) -> Vec<(String, Vec<String>)> {
    let mut clusters: HashMap<String, Vec<String>> = HashMap::new();
    for (name, resource) in &config.resources {
        let type_str = format!("{:?}", resource.resource_type);
        clusters.entry(type_str).or_default().push(name.clone());
    }
    for v in clusters.values_mut() { v.sort(); }
    let mut result: Vec<(String, Vec<String>)> = clusters.into_iter().collect();
    result.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    result
}

/// FJ-871: Identify near-cycle patterns (resources that depend on each other indirectly).
pub(crate) fn cmd_graph_resource_dependency_cycle_risk(
    file: &std::path::Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let risks = find_cycle_risks(&config);
    if json {
        let items: Vec<String> = risks.iter()
            .map(|(a, b, depth)| format!("{{\"from\":\"{}\",\"to\":\"{}\",\"mutual_depth\":{}}}", a, b, depth)).collect();
        println!("{{\"cycle_risks\":[{}]}}", items.join(","));
    } else if risks.is_empty() {
        println!("No dependency cycle risks found.");
    } else {
        println!("Dependency cycle risks:");
        for (a, b, depth) in &risks { println!("  {} ↔ {} — mutual depth {}", a, b, depth); }
    }
    Ok(())
}

fn find_cycle_risks(config: &types::ForjarConfig) -> Vec<(String, String, usize)> {
    let mut risks = Vec::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            let dep_res = match config.resources.get(dep) {
                Some(r) => r, None => continue,
            };
            if !dep_res.depends_on.contains(name) { continue; }
            let pair = if name < dep { (name.clone(), dep.clone()) } else { (dep.clone(), name.clone()) };
            let already_found = risks.iter().any(|(a, b, _): &(String, String, usize)| a == &pair.0 && b == &pair.1);
            if !already_found { risks.push((pair.0, pair.1, 1)); }
        }
    }
    risks.sort_by(|a, b| a.0.cmp(&b.0));
    risks
}

/// FJ-875: Calculate blast radius of resource changes.
pub(crate) fn cmd_graph_resource_impact_radius_analysis(
    file: &std::path::Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let radii = compute_impact_radii(&config);
    if json {
        let items: Vec<String> = radii.iter()
            .map(|(n, r)| format!("{{\"resource\":\"{}\",\"impact_radius\":{}}}", n, r)).collect();
        println!("{{\"impact_radii\":[{}]}}", items.join(","));
    } else if radii.is_empty() {
        println!("No resources found.");
    } else {
        println!("Resource impact radius (blast radius):");
        for (n, r) in &radii { println!("  {} — impact radius {}", n, r); }
    }
    Ok(())
}

fn compute_impact_radii(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut radii = Vec::new();
    for name in config.resources.keys() {
        let radius = count_transitive_dependents(config, name);
        radii.push((name.clone(), radius));
    }
    radii.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    radii
}

fn count_transitive_dependents(config: &types::ForjarConfig, target: &str) -> usize {
    let mut visited: Vec<String> = Vec::new();
    let mut queue: Vec<String> = vec![target.to_string()];
    while let Some(current) = queue.pop() {
        for (name, resource) in &config.resources {
            if resource.depends_on.contains(&current) && !visited.contains(name) {
                visited.push(name.clone());
                queue.push(name.clone());
            }
        }
    }
    visited.len()
}

/// FJ-879: Show dependency graph with health status overlay from state.
pub(crate) fn cmd_graph_resource_dependency_health_map(file: &Path, json: bool) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| e.to_string())?;
    let mut nodes: Vec<(String, Vec<String>)> = Vec::new();
    for (name, resource) in &config.resources {
        nodes.push((name.clone(), resource.depends_on.clone()));
    }
    nodes.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = nodes.iter().map(|(n, deps)| {
            let d: Vec<String> = deps.iter().map(|d| format!("\"{}\"", d)).collect();
            format!("{{\"resource\":\"{}\",\"depends_on\":[{}],\"health\":\"unknown\"}}", n, d.join(","))
        }).collect();
        println!("{{\"dependency_health_map\":[{}]}}", items.join(","));
    } else if nodes.is_empty() {
        println!("No resources found for dependency health map.");
    } else {
        println!("Dependency health map:");
        for (name, deps) in &nodes {
            if deps.is_empty() {
                println!("  {} (no dependencies)", name);
            } else {
                println!("  {} → {}", name, deps.join(", "));
            }
        }
    }
    Ok(())
}

/// FJ-883: Show how changes propagate through dependency chains.
pub(crate) fn cmd_graph_resource_change_propagation(file: &Path, json: bool) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| e.to_string())?;
    let propagation = compute_propagation_chains(&config);
    if json {
        let items: Vec<String> = propagation.iter().map(|(n, count)| {
            format!("{{\"resource\":\"{}\",\"propagation_depth\":{}}}", n, count)
        }).collect();
        println!("{{\"change_propagation\":[{}]}}", items.join(","));
    } else if propagation.is_empty() {
        println!("No change propagation paths found.");
    } else {
        println!("Change propagation analysis (resources by impact depth):");
        for (name, depth) in &propagation { println!("  {} — propagation depth {}", name, depth); }
    }
    Ok(())
}

fn compute_propagation_chains(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut chains = Vec::new();
    for name in config.resources.keys() {
        let depth = count_transitive_dependents(config, name);
        if depth > 0 { chains.push((name.clone(), depth)); }
    }
    chains.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    chains
}
