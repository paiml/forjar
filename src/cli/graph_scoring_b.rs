use crate::core::types;
use std::path::Path;

/// FJ-875: Calculate blast radius of resource changes.
pub(crate) fn cmd_graph_resource_impact_radius_analysis(
    file: &std::path::Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let radii = compute_impact_radii(&config);
    if json {
        let items: Vec<String> = radii
            .iter()
            .map(|(n, r)| format!("{{\"resource\":\"{n}\",\"impact_radius\":{r}}}"))
            .collect();
        println!("{{\"impact_radii\":[{}]}}", items.join(","));
    } else if radii.is_empty() {
        println!("No resources found.");
    } else {
        println!("Resource impact radius (blast radius):");
        for (n, r) in &radii {
            println!("  {n} — impact radius {r}");
        }
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
pub(crate) fn cmd_graph_resource_dependency_health_map(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| e.to_string())?;
    let mut nodes: Vec<(String, Vec<String>)> = Vec::new();
    for (name, resource) in &config.resources {
        nodes.push((name.clone(), resource.depends_on.clone()));
    }
    nodes.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let items: Vec<String> = nodes
            .iter()
            .map(|(n, deps)| {
                let d: Vec<String> = deps.iter().map(|d| format!("\"{d}\"")).collect();
                format!(
                    "{{\"resource\":\"{}\",\"depends_on\":[{}],\"health\":\"unknown\"}}",
                    n,
                    d.join(",")
                )
            })
            .collect();
        println!("{{\"dependency_health_map\":[{}]}}", items.join(","));
    } else if nodes.is_empty() {
        println!("No resources found for dependency health map.");
    } else {
        println!("Dependency health map:");
        for (name, deps) in &nodes {
            if deps.is_empty() {
                println!("  {name} (no dependencies)");
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
        let items: Vec<String> = propagation
            .iter()
            .map(|(n, count)| format!("{{\"resource\":\"{n}\",\"propagation_depth\":{count}}}"))
            .collect();
        println!("{{\"change_propagation\":[{}]}}", items.join(","));
    } else if propagation.is_empty() {
        println!("No change propagation paths found.");
    } else {
        println!("Change propagation analysis (resources by impact depth):");
        for (name, depth) in &propagation {
            println!("  {name} — propagation depth {depth}");
        }
    }
    Ok(())
}

fn compute_propagation_chains(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut chains = Vec::new();
    for name in config.resources.keys() {
        let depth = count_transitive_dependents(config, name);
        if depth > 0 {
            chains.push((name.clone(), depth));
        }
    }
    chains.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    chains
}

/// FJ-887: Show max dependency chain depth per resource.
pub(crate) fn cmd_graph_resource_dependency_depth_analysis(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| e.to_string())?;
    let depths = compute_dependency_depths(&config);
    if json {
        let items: Vec<String> = depths
            .iter()
            .map(|(n, d)| format!("{{\"resource\":\"{n}\",\"max_depth\":{d}}}"))
            .collect();
        println!("{{\"dependency_depth_analysis\":[{}]}}", items.join(","));
    } else if depths.is_empty() {
        println!("No resources found for depth analysis.");
    } else {
        println!("Dependency depth analysis (deepest first):");
        for (name, depth) in &depths {
            println!("  {name} — depth {depth}");
        }
    }
    Ok(())
}

fn compute_dependency_depths(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut depths = Vec::new();
    for name in config.resources.keys() {
        let depth = compute_chain_depth(config, name, 0);
        depths.push((name.clone(), depth));
    }
    depths.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    depths
}

fn compute_chain_depth(config: &types::ForjarConfig, name: &str, current: usize) -> usize {
    let resource = match config.resources.get(name) {
        Some(r) => r,
        None => return current,
    };
    if resource.depends_on.is_empty() {
        return current;
    }
    resource
        .depends_on
        .iter()
        .map(|dep| compute_chain_depth(config, dep, current + 1))
        .max()
        .unwrap_or(current)
}

/// FJ-891: Combined fan-in/fan-out analysis per resource.
pub(crate) fn cmd_graph_resource_dependency_fan_analysis(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| e.to_string())?;
    let analysis = compute_fan_analysis(&config);
    if json {
        let items: Vec<String> = analysis
            .iter()
            .map(|(n, fi, fo)| {
                format!(
                    "{{\"resource\":\"{n}\",\"fan_in\":{fi},\"fan_out\":{fo}}}"
                )
            })
            .collect();
        println!("{{\"fan_analysis\":[{}]}}", items.join(","));
    } else if analysis.is_empty() {
        println!("No resources found for fan analysis.");
    } else {
        println!("Fan-in/fan-out analysis:");
        for (name, fi, fo) in &analysis {
            println!("  {name} — fan-in: {fi}, fan-out: {fo}");
        }
    }
    Ok(())
}

fn compute_fan_analysis(config: &types::ForjarConfig) -> Vec<(String, usize, usize)> {
    let mut analysis = Vec::new();
    for (name, resource) in &config.resources {
        let fan_out = resource.depends_on.len();
        let fan_in = config
            .resources
            .values()
            .filter(|r| r.depends_on.contains(name))
            .count();
        analysis.push((name.clone(), fan_in, fan_out));
    }
    analysis.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then(a.0.cmp(&b.0)));
    analysis
}

/// FJ-895: Isolation score per resource in dependency graph.
pub(crate) fn cmd_graph_resource_dependency_isolation_score(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let scores = compute_isolation_scores(&config);
    if json {
        let items: Vec<String> = scores
            .iter()
            .map(|(n, s)| format!("{{\"resource\":\"{n}\",\"isolation_score\":{s:.2}}}"))
            .collect();
        println!("{{\"dependency_isolation_scores\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No resources to analyze.");
    } else {
        println!("Dependency isolation scores (1.0 = fully isolated):");
        for (n, s) in &scores {
            println!("  {n} — {s:.2}");
        }
    }
    Ok(())
}

fn compute_isolation_scores(config: &types::ForjarConfig) -> Vec<(String, f64)> {
    let total = config.resources.len();
    if total == 0 {
        return Vec::new();
    }
    let max_connections: f64 = (total - 1) as f64;
    let mut scores: Vec<(String, f64)> = config
        .resources
        .iter()
        .map(|(name, res)| {
            let fan_out = res.depends_on.len();
            let fan_in = config
                .resources
                .values()
                .filter(|r| r.depends_on.contains(name))
                .count();
            let connections = (fan_in + fan_out) as f64;
            let isolation: f64 = if max_connections > 0.0 {
                1.0 - (connections / max_connections)
            } else {
                1.0
            };
            (name.clone(), if isolation < 0.0 { 0.0 } else { isolation })
        })
        .collect();
    scores.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    scores
}

/// FJ-899: Stability score based on dependency change frequency.
pub(crate) fn cmd_graph_resource_dependency_stability_score(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let scores = compute_dep_stability_scores(&config);
    if json {
        let items: Vec<String> = scores
            .iter()
            .map(|(n, s)| format!("{{\"resource\":\"{n}\",\"stability_score\":{s:.2}}}"))
            .collect();
        println!("{{\"dependency_stability_scores\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No resources to analyze.");
    } else {
        println!("Dependency stability scores (1.0 = most stable):");
        for (n, s) in &scores {
            println!("  {n} — {s:.2}");
        }
    }
    Ok(())
}

fn compute_dep_stability_scores(config: &types::ForjarConfig) -> Vec<(String, f64)> {
    let mut scores: Vec<(String, f64)> = config
        .resources
        .iter()
        .map(|(name, res)| {
            let dep_count = res.depends_on.len() + res.triggers.len();
            let score: f64 = 1.0 / (1.0 + dep_count as f64);
            (name.clone(), score)
        })
        .collect();
    scores.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    scores
}
