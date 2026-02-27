//! Topological analysis.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use std::collections::HashMap;


/// FJ-584: Show resources grouped by topological depth level.
pub(crate) fn cmd_graph_topological_levels(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Calculate depth of each resource (max depth of dependencies + 1)
    let mut depths: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    fn calc_depth(
        name: &str,
        resources: &indexmap::IndexMap<String, crate::core::types::Resource>,
        depths: &mut std::collections::HashMap<String, usize>,
        visited: &mut std::collections::HashSet<String>,
    ) -> usize {
        if let Some(&d) = depths.get(name) {
            return d;
        }
        if visited.contains(name) {
            return 0; // cycle protection
        }
        visited.insert(name.to_string());
        let res = match resources.get(name) {
            Some(r) => r,
            None => return 0,
        };
        let max_dep = res
            .depends_on
            .iter()
            .map(|dep| calc_depth(dep, resources, depths, visited))
            .max()
            .unwrap_or(0);
        let depth = if res.depends_on.is_empty() {
            0
        } else {
            max_dep + 1
        };
        depths.insert(name.to_string(), depth);
        depth
    }

    let resource_names: Vec<String> = config.resources.keys().cloned().collect();
    for name in &resource_names {
        let mut visited = std::collections::HashSet::new();
        calc_depth(name, &config.resources, &mut depths, &mut visited);
    }

    // Group by level
    let max_level = depths.values().max().copied().unwrap_or(0);
    let mut levels: Vec<Vec<String>> = vec![Vec::new(); max_level + 1];
    for (name, depth) in &depths {
        levels[*depth].push(name.clone());
    }
    for level in &mut levels {
        level.sort();
    }

    if json {
        let items: Vec<String> = levels
            .iter()
            .enumerate()
            .map(|(i, rs)| {
                let r_items: Vec<String> = rs.iter().map(|r| format!(r#""{}""#, r)).collect();
                format!(
                    r#"{{"level":{},"resources":[{}],"count":{}}}"#,
                    i,
                    r_items.join(","),
                    rs.len()
                )
            })
            .collect();
        println!(r#"{{"topological_levels":[{}]}}"#, items.join(","));
    } else {
        println!("Topological levels ({} levels):", max_level + 1);
        for (i, resources) in levels.iter().enumerate() {
            if !resources.is_empty() {
                println!(
                    "  Level {} ({}): {}",
                    i,
                    resources.len(),
                    resources.join(", ")
                );
            }
        }
    }
    Ok(())
}


/// FJ-634: Show longest dependency chain (critical path analysis).
pub(crate) fn cmd_graph_critical_chain(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Find longest path in DAG using DFS
    fn longest_path(
        name: &str,
        resources: &indexmap::IndexMap<String, crate::core::types::Resource>,
        memo: &mut std::collections::HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
    ) -> Vec<String> {
        if let Some(path) = memo.get(name) {
            return path.clone();
        }
        if visited.contains(name) {
            return vec![name.to_string()];
        }
        visited.insert(name.to_string());
        let res = match resources.get(name) {
            Some(r) => r,
            None => return vec![name.to_string()],
        };
        let mut best: Vec<String> = Vec::new();
        for dep in &res.depends_on {
            let path = longest_path(dep, resources, memo, visited);
            if path.len() > best.len() {
                best = path;
            }
        }
        best.push(name.to_string());
        memo.insert(name.to_string(), best.clone());
        best
    }

    let mut memo = std::collections::HashMap::new();
    let mut longest = Vec::new();
    for name in config.resources.keys() {
        let mut visited = std::collections::HashSet::new();
        let path = longest_path(name, &config.resources, &mut memo, &mut visited);
        if path.len() > longest.len() {
            longest = path;
        }
    }

    if json {
        let items: Vec<String> = longest.iter().map(|n| format!(r#""{}""#, n)).collect();
        println!(
            r#"{{"critical_chain":[{}],"length":{}}}"#,
            items.join(","),
            longest.len()
        );
    } else if longest.is_empty() {
        println!("No dependency chains found");
    } else {
        println!("Critical chain ({} steps):", longest.len());
        println!("  {}", longest.join(" -> "));
    }
    Ok(())
}


/// FJ-624: Show which resources can execute in parallel.
pub(crate) fn cmd_graph_parallel_groups(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Group by topological level — resources at the same level can run in parallel
    let mut depths: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    fn calc_depth_p(
        name: &str,
        resources: &indexmap::IndexMap<String, crate::core::types::Resource>,
        depths: &mut std::collections::HashMap<String, usize>,
        visited: &mut std::collections::HashSet<String>,
    ) -> usize {
        if let Some(&d) = depths.get(name) {
            return d;
        }
        if visited.contains(name) {
            return 0;
        }
        visited.insert(name.to_string());
        let res = match resources.get(name) {
            Some(r) => r,
            None => return 0,
        };
        let max_dep = res
            .depends_on
            .iter()
            .map(|dep| calc_depth_p(dep, resources, depths, visited))
            .max()
            .unwrap_or(0);
        let depth = if res.depends_on.is_empty() {
            0
        } else {
            max_dep + 1
        };
        depths.insert(name.to_string(), depth);
        depth
    }

    let names: Vec<String> = config.resources.keys().cloned().collect();
    for name in &names {
        let mut visited = std::collections::HashSet::new();
        calc_depth_p(name, &config.resources, &mut depths, &mut visited);
    }

    // Group by level
    let mut levels: std::collections::BTreeMap<usize, Vec<String>> =
        std::collections::BTreeMap::new();
    for (name, &depth) in &depths {
        levels.entry(depth).or_default().push(name.clone());
    }
    for group in levels.values_mut() {
        group.sort();
    }

    if json {
        let items: Vec<String> = levels
            .iter()
            .map(|(level, names)| {
                let name_list: Vec<String> = names.iter().map(|n| format!(r#""{}""#, n)).collect();
                format!(
                    r#"{{"level":{},"parallel":[{}]}}"#,
                    level,
                    name_list.join(",")
                )
            })
            .collect();
        println!(
            r#"{{"parallel_groups":[{}],"total_levels":{}}}"#,
            items.join(","),
            levels.len()
        );
    } else if levels.is_empty() {
        println!("No resources to parallelize");
    } else {
        println!("Parallel execution groups ({} levels):", levels.len());
        for (level, names) in &levels {
            println!(
                "  Level {} ({} parallel): {}",
                level,
                names.len(),
                names.join(", ")
            );
        }
    }
    Ok(())
}


/// FJ-694: Show execution order with timing estimates
/// FJ-694: Show resource fan-out metrics (how many resources depend on each)
pub(crate) fn cmd_graph_fan_out(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut fan_out: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for name in cfg.resources.keys() {
        fan_out.entry(name.clone()).or_default();
    }
    for (name, resource) in &cfg.resources {
        for dep in &resource.depends_on {
            fan_out.entry(dep.clone()).or_default().push(name.clone());
        }
    }
    let mut sorted: Vec<_> = fan_out.into_iter().collect();
    sorted.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    if json {
        let entries: Vec<String> = sorted
            .iter()
            .map(|(name, dependents)| {
                let deps: Vec<String> = dependents.iter().map(|d| format!("\"{}\"", d)).collect();
                format!(
                    "{{\"resource\":\"{}\",\"fan_out\":{},\"dependents\":[{}]}}",
                    name,
                    dependents.len(),
                    deps.join(",")
                )
            })
            .collect();
        println!("{{\"fan_out\":[{}]}}", entries.join(","));
    } else {
        println!("Resource fan-out (dependents count):");
        for (name, dependents) in &sorted {
            if dependents.is_empty() {
                println!("  {} — 0 dependents (leaf)", name);
            } else {
                println!(
                    "  {} — {} dependent(s): {}",
                    name,
                    dependents.len(),
                    dependents.join(", ")
                );
            }
        }
    }
    Ok(())
}


/// FJ-724: Show depth-first traversal order
pub(crate) fn cmd_graph_depth_first(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut order: Vec<String> = Vec::new();
    fn dfs(
        name: &str,
        cfg: &types::ForjarConfig,
        visited: &mut std::collections::HashSet<String>,
        order: &mut Vec<String>,
    ) {
        if visited.contains(name) {
            return;
        }
        visited.insert(name.to_string());
        if let Some(resource) = cfg.resources.get(name) {
            for dep in &resource.depends_on {
                dfs(dep, cfg, visited, order);
            }
        }
        order.push(name.to_string());
    }
    let mut names: Vec<String> = cfg.resources.keys().cloned().collect();
    names.sort();
    for name in &names {
        dfs(name, &cfg, &mut visited, &mut order);
    }
    if json {
        let entries: Vec<String> = order
            .iter()
            .enumerate()
            .map(|(i, n)| format!("{{\"step\":{},\"resource\":\"{}\"}}", i + 1, n))
            .collect();
        println!("{{\"depth_first_order\":[{}]}}", entries.join(","));
    } else {
        println!("Depth-first traversal ({} resources):", order.len());
        for (i, name) in order.iter().enumerate() {
            println!("  {}. {}", i + 1, name);
        }
    }
    Ok(())
}


/// BFS topological sort: returns resources in breadth-first order.
fn bfs_topological(cfg: &types::ForjarConfig) -> Vec<String> {
    let mut in_degree: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut dependents: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (name, resource) in &cfg.resources {
        in_degree.entry(name.clone()).or_insert(0);
        for dep in &resource.depends_on {
            dependents.entry(dep.clone()).or_default().push(name.clone());
            *in_degree.entry(name.clone()).or_default() += 1;
        }
    }
    let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
    let mut roots: Vec<String> = in_degree.iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    roots.sort();
    for r in roots { queue.push_back(r); }
    let mut order: Vec<String> = Vec::new();
    while let Some(node) = queue.pop_front() {
        order.push(node.clone());
        let mut next: Vec<String> = dependents.get(&node).cloned().unwrap_or_default();
        next.sort();
        for dep in next {
            if let Some(d) = in_degree.get_mut(&dep) {
                *d -= 1;
                if *d == 0 { queue.push_back(dep); }
            }
        }
    }
    order
}

/// FJ-734: Show breadth-first traversal order.
pub(crate) fn cmd_graph_breadth_first(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let order = bfs_topological(&cfg);
    if json {
        let entries: Vec<String> = order
            .iter()
            .enumerate()
            .map(|(i, n)| format!("{{\"step\":{},\"resource\":\"{}\"}}", i + 1, n))
            .collect();
        println!("{{\"breadth_first_order\":[{}]}}", entries.join(","));
    } else {
        println!("Breadth-first traversal ({} resources):", order.len());
        for (i, name) in order.iter().enumerate() {
            println!("  {}. {}", i + 1, name);
        }
    }
    Ok(())
}

