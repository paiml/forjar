//! Core graph commands.

use super::helpers::*;
use crate::core::{resolver, types};
use std::path::Path;

/// Get the machine label string for a resource.
fn machine_label(resource: &types::Resource) -> String {
    match &resource.machine {
        types::MachineTarget::Single(m) => m.clone(),
        types::MachineTarget::Multiple(ms) => ms.join(","),
    }
}

/// Output graph in mermaid format.
fn print_graph_mermaid(config: &types::ForjarConfig) {
    println!("graph TD");
    for (id, resource) in &config.resources {
        let machine = machine_label(resource);
        println!(
            "    {}[\"{}: {} ({})\"]",
            id, id, resource.resource_type, machine
        );
        for dep in &resource.depends_on {
            println!("    {dep} --> {id}");
        }
    }
}

/// Output graph in DOT format.
fn print_graph_dot(config: &types::ForjarConfig) {
    println!("digraph forjar {{");
    println!("    rankdir=TB;");
    println!("    node [shape=box, style=rounded];");
    for (id, resource) in &config.resources {
        let machine = machine_label(resource);
        println!(
            "    \"{}\" [label=\"{}: {} ({})\"];",
            id, id, resource.resource_type, machine
        );
        for dep in &resource.depends_on {
            println!("    \"{dep}\" -> \"{id}\";");
        }
    }
    println!("}}");
}

/// Output graph in ASCII tree format.
fn print_graph_ascii(config: &types::ForjarConfig) -> Result<(), String> {
    let execution_order = resolver::build_execution_order(config)?;
    println!("{}", bold("Dependency Graph"));
    println!();
    for id in &execution_order {
        if let Some(resource) = config.resources.get(id) {
            print_ascii_resource(id, resource);
        }
    }
    println!();
    println!("{} resources in execution order.", execution_order.len());
    Ok(())
}

/// Print a single resource in ASCII format.
fn print_ascii_resource(id: &str, resource: &types::Resource) {
    let machine = machine_label(resource);
    if resource.depends_on.is_empty() {
        println!(
            "  {} {} ({}, {})",
            green("*"),
            bold(id),
            resource.resource_type,
            dim(&machine)
        );
    } else {
        let deps: Vec<&str> = resource.depends_on.iter().map(|s| s.as_str()).collect();
        println!(
            "  {} {} ({}, {}) <- [{}]",
            yellow("*"),
            bold(id),
            resource.resource_type,
            dim(&machine),
            deps.join(", ")
        );
    }
}

pub(crate) fn cmd_graph(
    file: &Path,
    format: &str,
    machine_filter: Option<&str>,
    group_filter: Option<&str>,
) -> Result<(), String> {
    let mut config = parse_and_validate(file)?;

    // FJ-294: Filter resources by machine or group
    if machine_filter.is_some() || group_filter.is_some() {
        config.resources.retain(|_id, resource| {
            if let Some(mf) = machine_filter {
                let matches = match &resource.machine {
                    types::MachineTarget::Single(m) => m == mf,
                    types::MachineTarget::Multiple(ms) => ms.iter().any(|m| m == mf),
                };
                if !matches {
                    return false;
                }
            }
            if let Some(gf) = group_filter {
                if resource.resource_group.as_deref() != Some(gf) {
                    return false;
                }
            }
            true
        });
    }

    match format {
        "mermaid" => print_graph_mermaid(&config),
        "dot" => print_graph_dot(&config),
        "ascii" => print_graph_ascii(&config)?,
        "svg" => super::graph_svg::print_graph_svg(&config),
        other => {
            return Err(format!(
                "unknown graph format '{other}': use mermaid, dot, ascii, or svg"
            ))
        }
    }

    Ok(())
}

// FJ-354: Show transitive dependents of a resource
pub(crate) fn cmd_graph_affected(file: &Path, resource: &str) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    if !config.resources.contains_key(resource) {
        return Err(format!("Resource '{resource}' not found in config"));
    }

    // Build reverse dependency map
    let mut dependents: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (id, res) in &config.resources {
        for dep in &res.depends_on {
            dependents.entry(dep.clone()).or_default().push(id.clone());
        }
    }

    // BFS to find all transitive dependents
    let mut affected = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    let mut visited = std::collections::HashSet::new();
    queue.push_back(resource.to_string());
    visited.insert(resource.to_string());

    while let Some(current) = queue.pop_front() {
        if let Some(deps) = dependents.get(&current) {
            for dep in deps {
                if visited.insert(dep.clone()) {
                    affected.push(dep.clone());
                    queue.push_back(dep.clone());
                }
            }
        }
    }

    affected.sort();

    println!("Resources affected by changes to '{}':\n", bold(resource));
    if affected.is_empty() {
        println!("  (none — no resources depend on '{resource}')");
    } else {
        for a in &affected {
            println!("  {} {}", yellow("→"), a);
        }
        println!(
            "\n{} {} transitive dependent(s)",
            green("Total:"),
            affected.len()
        );
    }

    Ok(())
}

// FJ-375: Critical path — longest dependency chain
pub(crate) fn cmd_graph_critical_path(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Build adjacency list
    let mut adj: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for (id, res) in &config.resources {
        for dep in &res.depends_on {
            adj.entry(dep.as_str()).or_default().push(id.as_str());
        }
    }

    // Find longest path via DFS from each root
    fn dfs<'a>(
        node: &'a str,
        adj: &std::collections::HashMap<&str, Vec<&'a str>>,
        memo: &mut std::collections::HashMap<&'a str, Vec<&'a str>>,
    ) -> Vec<&'a str> {
        if let Some(cached) = memo.get(node) {
            return cached.clone();
        }
        let mut longest = Vec::new();
        if let Some(children) = adj.get(node) {
            for child in children {
                let path = dfs(child, adj, memo);
                if path.len() > longest.len() {
                    longest = path;
                }
            }
        }
        let mut result = vec![node];
        result.extend(longest);
        memo.insert(node, result.clone());
        result
    }

    let mut memo = std::collections::HashMap::new();
    let mut critical = Vec::new();
    for id in config.resources.keys() {
        let path = dfs(id, &adj, &mut memo);
        if path.len() > critical.len() {
            critical = path;
        }
    }

    println!("Critical path ({} resources):\n", critical.len());
    for (i, node) in critical.iter().enumerate() {
        let prefix = if i == 0 {
            "┌"
        } else if i == critical.len() - 1 {
            "└"
        } else {
            "│"
        };
        println!("  {} {}", prefix, bold(node));
    }

    Ok(())
}

/// FJ-595: Show DAG execution order (flattened topological sort via Kahn's algorithm).
pub(crate) fn cmd_graph_execution_order(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Build in-degree map and adjacency list (Kahn's algorithm)
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut dependents: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for name in config.resources.keys() {
        in_degree.entry(name.clone()).or_insert(0);
    }
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            dependents
                .entry(dep.clone())
                .or_default()
                .push(name.clone());
            *in_degree.entry(name.clone()).or_insert(0) += 1;
        }
    }

    // BFS with alphabetical tie-breaking
    let mut queue: std::collections::BTreeSet<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(k, _)| k.clone())
        .collect();
    let mut order: Vec<String> = Vec::new();

    while let Some(name) = queue.iter().next().cloned() {
        queue.remove(&name);
        order.push(name.clone());
        if let Some(deps) = dependents.get(&name) {
            for dep in deps {
                if let Some(deg) = in_degree.get_mut(dep) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.insert(dep.clone());
                    }
                }
            }
        }
    }

    if json {
        print_execution_order_json(&order, &config);
    } else {
        print_execution_order_text(&order, &config);
    }
    Ok(())
}

/// Print execution order as JSON.
fn print_execution_order_json(order: &[String], config: &types::ForjarConfig) {
    let items: Vec<String> = order
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let rtype = config
                .resources
                .get(name)
                .map(|r| format!("{:?}", r.resource_type))
                .unwrap_or_else(|| "unknown".to_string());
            format!(
                r#"{{"step":{},"resource":"{}","type":"{}"}}"#,
                i + 1,
                name,
                rtype
            )
        })
        .collect();
    println!(
        r#"{{"execution_order":[{}],"total":{}}}"#,
        items.join(","),
        order.len()
    );
}

/// Print execution order as text.
fn print_execution_order_text(order: &[String], config: &types::ForjarConfig) {
    if order.is_empty() {
        println!("No resources in execution order");
    } else {
        println!("Execution order ({} resources):", order.len());
        for (i, name) in order.iter().enumerate() {
            let rtype = config
                .resources
                .get(name)
                .map(|r| format!("{:?}", r.resource_type))
                .unwrap_or_else(|| "unknown".to_string());
            println!("  {}. {} ({})", i + 1, name, rtype);
        }
    }
}

// FJ-385: Reverse dependency graph
pub(crate) fn cmd_graph_reverse(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    println!("Reverse Dependency Graph:\n");
    println!("(what depends on each resource)\n");

    let mut reverse_deps: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for id in config.resources.keys() {
        reverse_deps.entry(id.clone()).or_default();
    }
    for (id, res) in &config.resources {
        for dep in &res.depends_on {
            reverse_deps
                .entry(dep.clone())
                .or_default()
                .push(id.clone());
        }
    }

    for (resource, dependents) in &reverse_deps {
        if dependents.is_empty() {
            println!(
                "  {} {} (leaf — nothing depends on this)",
                dim("○"),
                resource
            );
        } else {
            println!("  {} {} ({})", bold("●"), resource, dependents.len());
            for d in dependents {
                println!("    {} {}", yellow("←"), d);
            }
        }
    }

    Ok(())
}
