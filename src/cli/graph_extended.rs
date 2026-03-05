//! Extended graph ops.

use super::helpers::*;
use crate::core::{resolver, types};
use std::path::Path;

/// Compute BFS depth map from roots.
#[allow(clippy::type_complexity)]
fn compute_depth_map<'a>(
    order: &'a [String],
    config: &'a types::ForjarConfig,
) -> (
    std::collections::HashMap<&'a str, usize>,
    std::collections::HashMap<&'a str, Vec<&'a str>>,
    std::collections::HashSet<&'a str>,
) {
    let mut children: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    let mut has_parent = std::collections::HashSet::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            children
                .entry(dep.as_str())
                .or_default()
                .push(name.as_str());
            has_parent.insert(name.as_str());
        }
    }

    let mut depth_map: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut queue = std::collections::VecDeque::new();
    for name in order {
        if !has_parent.contains(name.as_str()) {
            depth_map.insert(name.as_str(), 0);
            queue.push_back(name.as_str());
        }
    }
    while let Some(node) = queue.pop_front() {
        let d = depth_map[node];
        if let Some(kids) = children.get(node) {
            for &kid in kids {
                if !depth_map.contains_key(kid) {
                    depth_map.insert(kid, d + 1);
                    queue.push_back(kid);
                }
            }
        }
    }

    (depth_map, children, has_parent)
}

/// Print depth-filtered graph in Mermaid format.
fn print_depth_mermaid(
    config: &types::ForjarConfig,
    order: &[String],
    included: &std::collections::HashSet<&str>,
    children: &std::collections::HashMap<&str, Vec<&str>>,
) {
    println!("graph TD");
    for (name, res) in &config.resources {
        if !included.contains(name.as_str()) {
            continue;
        }
        for dep in &res.depends_on {
            if included.contains(dep.as_str()) {
                println!("  {dep} --> {name}");
            }
        }
    }
    for name in order {
        if included.contains(name.as_str()) {
            let res = &config.resources[name];
            if res.depends_on.is_empty() && !children.contains_key(name.as_str()) {
                println!("  {name}");
            }
        }
    }
}

/// Print depth-filtered graph in DOT format.
fn print_depth_dot(config: &types::ForjarConfig, included: &std::collections::HashSet<&str>) {
    println!("digraph G {{");
    for (name, res) in &config.resources {
        if !included.contains(name.as_str()) {
            continue;
        }
        for dep in &res.depends_on {
            if included.contains(dep.as_str()) {
                println!("  \"{dep}\" -> \"{name}\"");
            }
        }
    }
    println!("}}");
}

/// Print clustered graph in Mermaid format.
fn print_cluster_mermaid(
    by_machine: &std::collections::HashMap<String, Vec<String>>,
    config: &types::ForjarConfig,
) {
    println!("graph TD");
    for (machine, resources) in by_machine {
        println!("  subgraph {machine}");
        for name in resources {
            println!("    {name}");
        }
        println!("  end");
    }
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            println!("  {dep} --> {name}");
        }
    }
}

/// Print clustered graph in DOT format.
fn print_cluster_dot(
    by_machine: &std::collections::HashMap<String, Vec<String>>,
    config: &types::ForjarConfig,
) {
    println!("digraph G {{");
    for (machine, resources) in by_machine {
        println!("  subgraph cluster_{} {{", machine.replace('-', "_"));
        println!("    label=\"{machine}\";");
        for name in resources {
            println!("    \"{name}\";");
        }
        println!("  }}");
    }
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            println!("  \"{dep}\" -> \"{name}\";");
        }
    }
    println!("}}");
}

/// Print highlighted graph in DOT format.
fn print_highlight_dot(
    order: &[String],
    config: &types::ForjarConfig,
    highlighted: &std::collections::HashSet<String>,
) {
    println!("digraph {{");
    println!("  rankdir=LR;");
    for name in order {
        if highlighted.contains(name) {
            println!("  \"{name}\" [style=filled, fillcolor=yellow];");
        } else {
            println!("  \"{name}\";");
        }
        if let Some(res) = config.resources.get(name) {
            for dep in &res.depends_on {
                let style = if highlighted.contains(name) && highlighted.contains(dep) {
                    " [color=red, penwidth=2]"
                } else {
                    ""
                };
                println!("  \"{name}\" -> \"{dep}\"{style};");
            }
        }
    }
    println!("}}");
}

/// Print highlighted graph in Mermaid format.
fn print_highlight_mermaid(
    order: &[String],
    config: &types::ForjarConfig,
    highlighted: &std::collections::HashSet<String>,
) {
    println!("graph LR");
    for name in order {
        if highlighted.contains(name) {
            println!("  {name}[\"⚡ {name}\"]");
            println!("  style {name} fill:#ffeb3b,stroke:#f44336,stroke-width:2px");
        }
        if let Some(res) = config.resources.get(name) {
            for dep in &res.depends_on {
                if highlighted.contains(name) && highlighted.contains(dep) {
                    println!("  {name} ==>|dep| {dep}");
                } else {
                    println!("  {name} --> {dep}");
                }
            }
        }
    }
}

// ── FJ-394: graph --depth ──

pub(crate) fn cmd_graph_depth(file: &Path, format: &str, max_depth: usize) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;

    let (depth_map, children, _has_parent) = compute_depth_map(&order, &config);

    let included: std::collections::HashSet<&str> = depth_map
        .iter()
        .filter(|(_, &d)| d <= max_depth)
        .map(|(&n, _)| n)
        .collect();

    if format == "mermaid" {
        print_depth_mermaid(&config, &order, &included, &children);
    } else {
        print_depth_dot(&config, &included);
    }
    Ok(())
}

// ── FJ-404: graph --cluster ──

pub(crate) fn cmd_graph_cluster(file: &Path, format: &str) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Group resources by machine
    let mut by_machine: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, res) in &config.resources {
        let m = match &res.machine {
            types::MachineTarget::Single(m) => m.clone(),
            types::MachineTarget::Multiple(ms) => {
                ms.first().cloned().unwrap_or_else(|| "unknown".to_string())
            }
        };
        by_machine.entry(m).or_default().push(name.clone());
    }

    if format == "mermaid" {
        print_cluster_mermaid(&by_machine, &config);
    } else {
        print_cluster_dot(&by_machine, &config);
    }
    Ok(())
}

// ── FJ-414: graph --orphans ──

pub(crate) fn cmd_graph_orphans(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Build who-depends-on-whom
    let mut has_dependents = std::collections::HashSet::new();
    let mut has_deps = std::collections::HashSet::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            has_dependents.insert(dep.clone());
            has_deps.insert(name.clone());
        }
    }

    let mut orphans = Vec::new();
    for name in config.resources.keys() {
        if !has_dependents.contains(name) && !has_deps.contains(name) {
            orphans.push(name.clone());
        }
    }

    if orphans.is_empty() {
        println!("{} No orphan resources found", green("✓"));
    } else {
        println!(
            "{} {} orphan resource(s) (no deps, no dependents):",
            yellow("⚠"),
            orphans.len()
        );
        for name in &orphans {
            let res = &config.resources[name];
            println!("  {} (type: {:?})", name, res.resource_type);
        }
    }
    Ok(())
}

// ── FJ-424: graph --stats ──

pub(crate) fn cmd_graph_stats(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;

    let nodes = config.resources.len();
    let mut edges = 0usize;
    let mut max_deps = 0usize;
    for res in config.resources.values() {
        edges += res.depends_on.len();
        max_deps = max_deps.max(res.depends_on.len());
    }

    let (depth_map, children, has_parent) = compute_depth_map(&order, &config);
    let max_depth = depth_map.values().copied().max().unwrap_or(0);

    // Width = max nodes at any depth level
    let mut width_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for &d in depth_map.values() {
        *width_map.entry(d).or_insert(0) += 1;
    }
    let max_width = width_map.values().copied().max().unwrap_or(0);

    let roots = order
        .iter()
        .filter(|n| !has_parent.contains(n.as_str()))
        .count();
    let leaves = order
        .iter()
        .filter(|n| !children.contains_key(n.as_str()))
        .count();

    println!("{}", bold("Graph Statistics"));
    println!("  Nodes:     {nodes}");
    println!("  Edges:     {edges}");
    println!("  Depth:     {max_depth}");
    println!("  Width:     {max_width}");
    println!("  Roots:     {roots}");
    println!("  Leaves:    {leaves}");
    println!("  Max deps:  {max_deps}");
    if nodes > 0 {
        println!("  Density:   {:.2}", edges as f64 / nodes as f64);
    }
    Ok(())
}

// ── FJ-434: graph --json ──

pub(crate) fn cmd_graph_json(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;

    let mut adjacency: Vec<String> = Vec::new();
    for name in &order {
        let deps = config
            .resources
            .get(name)
            .map(|r| {
                r.depends_on
                    .iter()
                    .map(|s| format!("\"{s}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        adjacency.push(format!("\"{name}\":[{deps}]"));
    }
    println!("{{{}}}", adjacency.join(","));
    Ok(())
}

// ── FJ-444: graph --highlight ──

pub(crate) fn cmd_graph_highlight(file: &Path, format: &str, resource: &str) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;

    // Collect transitive deps of the highlighted resource
    let mut highlighted = std::collections::HashSet::new();
    highlighted.insert(resource.to_string());

    // BFS to find all transitive deps
    let mut queue = vec![resource.to_string()];
    while let Some(current) = queue.pop() {
        if let Some(res) = config.resources.get(&current) {
            for dep in &res.depends_on {
                if highlighted.insert(dep.clone()) {
                    queue.push(dep.clone());
                }
            }
        }
    }

    if format == "dot" {
        print_highlight_dot(&order, &config, &highlighted);
    } else {
        print_highlight_mermaid(&order, &config, &highlighted);
    }
    Ok(())
}
