//! Graph scoring — impact, stability, fanout, weight, bottleneck, clustering.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// FJ-847: Impact score based on dependents + depth.
pub(crate) fn cmd_graph_resource_impact_score(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let scores = compute_impact_scores(&config);
    if json {
        let items: Vec<String> = scores
            .iter()
            .map(|(r, s)| format!("{{\"resource\":\"{r}\",\"impact_score\":{s}}}"))
            .collect();
        println!("{{\"resource_impact_scores\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No resources.");
    } else {
        println!("Resource impact scores (dependents + depth):");
        for (r, s) in &scores {
            println!("  {r} — score {s}");
        }
    }
    Ok(())
}

pub(super) fn compute_impact_scores(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut fanin: HashMap<&str, usize> = HashMap::new();
    for resource in config.resources.values() {
        for dep in &resource.depends_on {
            *fanin.entry(dep.as_str()).or_default() += 1;
        }
    }
    let mut results: Vec<(String, usize)> = config
        .resources
        .keys()
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
pub(crate) fn cmd_graph_resource_stability_score(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let scores = compute_stability_scores(&config);
    if json {
        let items: Vec<String> = scores
            .iter()
            .map(|(r, s)| format!("{{\"resource\":\"{r}\",\"stability_score\":{s}}}"))
            .collect();
        println!("{{\"resource_stability_scores\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No resources.");
    } else {
        println!("Resource stability scores (higher = more stable):");
        for (r, s) in &scores {
            println!("  {r} — score {s}");
        }
    }
    Ok(())
}

pub(super) fn compute_stability_scores(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut results: Vec<(String, usize)> = config
        .resources
        .keys()
        .map(|name| {
            let r = &config.resources[name];
            let mut score = 10usize;
            score = score.saturating_sub(r.depends_on.len());
            if r.state.is_some() {
                score += 2;
            }
            if r.content.is_some() {
                score += 1;
            }
            (name.clone(), score)
        })
        .collect();
    results.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    results
}

/// FJ-855: Fan-out count per resource.
pub(crate) fn cmd_graph_resource_dependency_fanout(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let fanouts = compute_fanouts(&config);
    if json {
        let items: Vec<String> = fanouts
            .iter()
            .map(|(r, f)| format!("{{\"resource\":\"{r}\",\"fanout\":{f}}}"))
            .collect();
        println!("{{\"resource_dependency_fanout\":[{}]}}", items.join(","));
    } else if fanouts.is_empty() {
        println!("No resources.");
    } else {
        println!("Resource dependency fan-out:");
        for (r, f) in &fanouts {
            println!("  {r} — {f} dependents");
        }
    }
    Ok(())
}

pub(super) fn compute_fanouts(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut fanin: HashMap<&str, usize> = HashMap::new();
    for resource in config.resources.values() {
        for dep in &resource.depends_on {
            *fanin.entry(dep.as_str()).or_default() += 1;
        }
    }
    let mut results: Vec<(String, usize)> = config
        .resources
        .keys()
        .map(|name| {
            let count = fanin.get(name.as_str()).copied().unwrap_or(0);
            (name.clone(), count)
        })
        .collect();
    results.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    results
}

/// FJ-859: Weighted edges based on resource coupling.
pub(crate) fn cmd_graph_resource_dependency_weight(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let weights = compute_dependency_weights(&config);
    if json {
        let items: Vec<String> = weights
            .iter()
            .map(|(a, b, w)| format!("{{\"from\":\"{a}\",\"to\":\"{b}\",\"weight\":{w}}}"))
            .collect();
        println!("{{\"dependency_weights\":[{}]}}", items.join(","));
    } else if weights.is_empty() {
        println!("No dependency edges.");
    } else {
        println!("Dependency edge weights:");
        for (a, b, w) in &weights {
            println!("  {a} → {b} — weight {w}");
        }
    }
    Ok(())
}

pub(super) fn compute_dependency_weights(
    config: &types::ForjarConfig,
) -> Vec<(String, String, usize)> {
    let mut edges = Vec::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            let mut weight = 1usize;
            if let Some(dep_resource) = config.resources.get(dep) {
                let ma: HashSet<&str> = resource.machine.iter().collect();
                let mb: HashSet<&str> = dep_resource.machine.iter().collect();
                if !ma.is_disjoint(&mb) {
                    weight += 1;
                }
            }
            edges.push((name.clone(), dep.clone(), weight));
        }
    }
    edges.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    edges
}

/// FJ-863: Identify bottleneck resources (high fan-in + fan-out).
pub(crate) fn cmd_graph_resource_dependency_bottleneck(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let bottlenecks = find_dependency_bottlenecks(&config);
    if json {
        let items: Vec<String> = bottlenecks
            .iter()
            .map(|(n, fi, fo)| format!("{{\"resource\":\"{n}\",\"fan_in\":{fi},\"fan_out\":{fo}}}"))
            .collect();
        println!("{{\"bottlenecks\":[{}]}}", items.join(","));
    } else if bottlenecks.is_empty() {
        println!("No dependency bottlenecks found.");
    } else {
        println!("Dependency bottlenecks (fan-in + fan-out):");
        for (n, fi, fo) in &bottlenecks {
            println!("  {n} — fan-in {fi}, fan-out {fo}");
        }
    }
    Ok(())
}

pub(super) fn find_dependency_bottlenecks(
    config: &types::ForjarConfig,
) -> Vec<(String, usize, usize)> {
    let mut fan_in: HashMap<String, usize> = HashMap::new();
    let mut fan_out: HashMap<String, usize> = HashMap::new();
    for (name, resource) in &config.resources {
        fan_out.insert(name.clone(), resource.depends_on.len());
        for dep in &resource.depends_on {
            *fan_in.entry(dep.clone()).or_default() += 1;
        }
    }
    let mut results: Vec<(String, usize, usize)> = config
        .resources
        .keys()
        .map(|n| {
            (
                n.clone(),
                *fan_in.get(n).unwrap_or(&0),
                *fan_out.get(n).unwrap_or(&0),
            )
        })
        .filter(|(_, fi, fo)| *fi > 0 || *fo > 0)
        .collect();
    results.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    results
}

/// FJ-867: Cluster resources by type and show interconnections.
pub(crate) fn cmd_graph_resource_type_clustering(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let clusters = build_type_clusters(&config);
    if json {
        let items: Vec<String> = clusters
            .iter()
            .map(|(t, rs)| {
                let names: Vec<String> = rs.iter().map(|r| format!("\"{r}\"")).collect();
                format!("{{\"type\":\"{}\",\"resources\":[{}]}}", t, names.join(","))
            })
            .collect();
        println!("{{\"type_clusters\":[{}]}}", items.join(","));
    } else if clusters.is_empty() {
        println!("No resources to cluster.");
    } else {
        println!("Resource type clusters:");
        for (t, rs) in &clusters {
            println!("  {} — {} resources: {}", t, rs.len(), rs.join(", "));
        }
    }
    Ok(())
}

pub(super) fn build_type_clusters(config: &types::ForjarConfig) -> Vec<(String, Vec<String>)> {
    let mut clusters: HashMap<String, Vec<String>> = HashMap::new();
    for (name, resource) in &config.resources {
        let type_str = format!("{:?}", resource.resource_type);
        clusters.entry(type_str).or_default().push(name.clone());
    }
    for v in clusters.values_mut() {
        v.sort();
    }
    let mut result: Vec<(String, Vec<String>)> = clusters.into_iter().collect();
    result.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    result
}

/// FJ-871: Identify near-cycle patterns (resources that depend on each other indirectly).
pub(crate) fn cmd_graph_resource_dependency_cycle_risk(
    file: &std::path::Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let risks = find_cycle_risks(&config);
    if json {
        let items: Vec<String> = risks
            .iter()
            .map(|(a, b, depth)| {
                format!("{{\"from\":\"{a}\",\"to\":\"{b}\",\"mutual_depth\":{depth}}}")
            })
            .collect();
        println!("{{\"cycle_risks\":[{}]}}", items.join(","));
    } else if risks.is_empty() {
        println!("No dependency cycle risks found.");
    } else {
        println!("Dependency cycle risks:");
        for (a, b, depth) in &risks {
            println!("  {a} ↔ {b} — mutual depth {depth}");
        }
    }
    Ok(())
}

pub(super) fn find_cycle_risks(config: &types::ForjarConfig) -> Vec<(String, String, usize)> {
    let mut risks = Vec::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            let dep_res = match config.resources.get(dep) {
                Some(r) => r,
                None => continue,
            };
            if !dep_res.depends_on.contains(name) {
                continue;
            }
            let pair = if name < dep {
                (name.clone(), dep.clone())
            } else {
                (dep.clone(), name.clone())
            };
            let already_found = risks
                .iter()
                .any(|(a, b, _): &(String, String, usize)| a == &pair.0 && b == &pair.1);
            if !already_found {
                risks.push((pair.0, pair.1, 1));
            }
        }
    }
    risks.sort_by(|a, b| a.0.cmp(&b.0));
    risks
}

pub(super) use super::graph_scoring_b::*;
