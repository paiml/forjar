//! Safety validation — circular deps, machine refs.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::collections::{HashMap, HashSet};
use std::path::Path;


/// FJ-757: Detect circular dependency chains.
pub(crate) fn cmd_validate_check_circular_deps(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let cycles = find_cycles(&config);
    if json {
        let items: Vec<String> = cycles.iter()
            .map(|c| format!("{:?}", c))
            .collect();
        println!("{{\"circular_deps\":[{}]}}", items.join(","));
    } else if cycles.is_empty() {
        println!("No circular dependencies detected.");
    } else {
        println!("Circular dependencies ({}):", cycles.len());
        for cycle in &cycles {
            println!("  {} → {}", cycle.join(" → "), cycle[0]);
        }
    }
    Ok(())
}

/// Find cycles using DFS with coloring (white/gray/black).
fn find_cycles(config: &types::ForjarConfig) -> Vec<Vec<String>> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            adj.entry(name.as_str()).or_default().push(dep.as_str());
        }
    }
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut in_stack = HashSet::new();
    let mut path = Vec::new();
    for name in config.resources.keys() {
        if !visited.contains(name.as_str()) {
            dfs_cycle(name, &adj, &mut visited, &mut in_stack, &mut path, &mut cycles);
        }
    }
    cycles
}

fn dfs_cycle<'a>(
    node: &'a str, adj: &HashMap<&str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>, in_stack: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>, cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node);
    in_stack.insert(node);
    path.push(node);
    if let Some(neighbors) = adj.get(node) {
        for &next in neighbors {
            if !visited.contains(next) {
                dfs_cycle(next, adj, visited, in_stack, path, cycles);
            } else if in_stack.contains(next) {
                let start = path.iter().position(|&n| n == next).unwrap_or(0);
                cycles.push(path[start..].iter().map(|s| s.to_string()).collect());
            }
        }
    }
    path.pop();
    in_stack.remove(node);
}


/// FJ-761: Verify all machine references in resources exist.
pub(crate) fn cmd_validate_check_machine_refs(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let bad = find_bad_machine_refs(&config);
    if json {
        let items: Vec<String> = bad.iter()
            .map(|(r, m)| format!("{{\"resource\":\"{}\",\"machine\":\"{}\"}}", r, m))
            .collect();
        println!("{{\"bad_machine_refs\":[{}]}}", items.join(","));
    } else if bad.is_empty() {
        println!("All machine references are valid.");
    } else {
        println!("Invalid machine references ({}):", bad.len());
        for (resource, machine) in &bad {
            println!("  {} → {} (not defined)", resource, machine);
        }
    }
    Ok(())
}

/// Find resources referencing machines that don't exist in config.
fn find_bad_machine_refs(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let machines: HashSet<&str> = config.machines.keys().map(|k| k.as_str()).collect();
    let mut bad = Vec::new();
    for (name, resource) in &config.resources {
        for m in resource.machine.to_vec() {
            if m != "localhost" && !machines.contains(m.as_str()) {
                bad.push((name.clone(), m));
            }
        }
    }
    bad.sort();
    bad
}
