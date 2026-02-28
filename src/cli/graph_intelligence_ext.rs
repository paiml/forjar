//! Graph intelligence extensions — fan-out, fan-in, path count, articulation points.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-943: Maximum outgoing edges per node (fan-out bottleneck).
pub(crate) fn cmd_graph_resource_dependency_fan_out(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut fan_outs: Vec<(String, usize)> = config.resources.iter()
        .map(|(name, res)| (name.clone(), res.depends_on.len()))
        .collect();
    fan_outs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let max = fan_outs.first().map(|(_, c)| *c).unwrap_or(0);
    if json {
        let items: Vec<String> = fan_outs.iter()
            .map(|(n, c)| format!("{{\"resource\":\"{}\",\"fan_out\":{}}}", n, c))
            .collect();
        println!("{{\"max_fan_out\":{},\"resources\":[{}]}}", max, items.join(","));
    } else if fan_outs.is_empty() {
        println!("No resources found.");
    } else {
        println!("Fan-out analysis (max: {}):", max);
        for (n, c) in &fan_outs { println!("  {} — {} outgoing", n, c); }
    }
    Ok(())
}

/// FJ-947: Maximum incoming edges per node (convergence point).
pub(crate) fn cmd_graph_resource_dependency_fan_in(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut in_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for name in config.resources.keys() { in_counts.insert(name.clone(), 0); }
    for res in config.resources.values() {
        for dep in &res.depends_on {
            *in_counts.entry(dep.clone()).or_insert(0) += 1;
        }
    }
    let mut fan_ins: Vec<(String, usize)> = in_counts.into_iter().collect();
    fan_ins.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let max = fan_ins.first().map(|(_, c)| *c).unwrap_or(0);
    if json {
        let items: Vec<String> = fan_ins.iter()
            .map(|(n, c)| format!("{{\"resource\":\"{}\",\"fan_in\":{}}}", n, c))
            .collect();
        println!("{{\"max_fan_in\":{},\"resources\":[{}]}}", max, items.join(","));
    } else if fan_ins.is_empty() {
        println!("No resources found.");
    } else {
        println!("Fan-in analysis (max: {}):", max);
        for (n, c) in &fan_ins { println!("  {} — {} incoming", n, c); }
    }
    Ok(())
}

/// FJ-951: Count of distinct dependency paths between all pairs.
pub(crate) fn cmd_graph_resource_dependency_path_count(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<&String> = config.resources.keys().collect();
    let n = names.len();
    let mut total_paths = 0usize;
    for i in 0..n {
        for j in 0..n {
            if i != j { total_paths += count_paths_between(&config, names[i], names[j]); }
        }
    }
    if json {
        println!("{{\"total_dependency_paths\":{},\"nodes\":{}}}", total_paths, n);
    } else {
        println!("Total dependency paths: {} ({} nodes)", total_paths, n);
    }
    Ok(())
}

fn count_paths_between(config: &types::ForjarConfig, from: &str, to: &str) -> usize {
    if from == to { return 1; }
    let res = match config.resources.get(from) { Some(r) => r, None => return 0 };
    let mut count = 0;
    for dep in &res.depends_on {
        count += count_paths_between(config, dep, to);
    }
    count
}

/// FJ-955: Identify articulation points whose removal disconnects graph.
pub(crate) fn cmd_graph_resource_dependency_articulation_points(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    let n = names.len();
    let mut points = Vec::new();
    let base_components = count_components_undirected(&config, &names, None);
    for i in 0..n {
        let removed = count_components_undirected(&config, &names, Some(&names[i]));
        if removed > base_components { points.push(names[i].clone()); }
    }
    points.sort();
    if json {
        let items: Vec<String> = points.iter().map(|p| format!("\"{}\"", p)).collect();
        println!("{{\"articulation_points\":[{}]}}", items.join(","));
    } else if points.is_empty() {
        println!("No articulation points found.");
    } else {
        println!("Articulation points:");
        for p in &points { println!("  {}", p); }
    }
    Ok(())
}

fn count_components_undirected(config: &types::ForjarConfig, names: &[String], exclude: Option<&str>) -> usize {
    let active: Vec<&String> = names.iter().filter(|n| exclude != Some(n.as_str())).collect();
    let mut visited = std::collections::HashSet::new();
    let mut components = 0;
    for name in &active {
        if visited.contains(name.as_str()) { continue; }
        components += 1;
        flood_fill_component(config, name, exclude, &mut visited);
    }
    components
}

fn flood_fill_component<'a>(config: &'a types::ForjarConfig, start: &'a str, exclude: Option<&str>, visited: &mut std::collections::HashSet<&'a str>) {
    let mut stack = vec![start];
    while let Some(v) = stack.pop() {
        if visited.contains(v) { continue; }
        visited.insert(v);
        push_forward_neighbors(config, v, exclude, visited, &mut stack);
        push_reverse_neighbors(config, v, exclude, visited, &mut stack);
    }
}

fn push_forward_neighbors<'a>(config: &'a types::ForjarConfig, v: &str, exclude: Option<&str>, visited: &std::collections::HashSet<&str>, stack: &mut Vec<&'a str>) {
    if let Some(res) = config.resources.get(v) {
        for dep in &res.depends_on {
            if exclude != Some(dep.as_str()) && !visited.contains(dep.as_str()) { stack.push(dep); }
        }
    }
}

fn push_reverse_neighbors<'a>(config: &'a types::ForjarConfig, v: &str, exclude: Option<&str>, visited: &std::collections::HashSet<&str>, stack: &mut Vec<&'a str>) {
    for (other_name, other_res) in &config.resources {
        if exclude != Some(other_name.as_str()) && other_res.depends_on.contains(&v.to_string()) && !visited.contains(other_name.as_str()) {
            stack.push(other_name);
        }
    }
}
