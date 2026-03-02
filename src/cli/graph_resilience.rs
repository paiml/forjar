//! Graph resilience analysis — parallel execution groups and execution cost estimation.

use crate::core::types;
use std::collections::{HashMap, VecDeque};
use std::path::Path;

/// Return the execution cost weight for a resource type.
fn type_cost(rt: &types::ResourceType) -> u32 {
    match rt {
        types::ResourceType::File => 1,
        types::ResourceType::Package => 5,
        types::ResourceType::Service => 3,
        types::ResourceType::Mount => 2,
        types::ResourceType::User => 2,
        types::ResourceType::Docker => 4,
        types::ResourceType::Cron => 1,
        types::ResourceType::Network => 2,
        types::ResourceType::Pepita => 3,
        types::ResourceType::Model => 5,
        types::ResourceType::Gpu => 5,
        types::ResourceType::Task => 3,
        types::ResourceType::Recipe => 1,
    }
}

/// FJ-1023: Partition the dependency graph into parallelizable execution groups
/// using topological levels (Kahn's algorithm with BFS level assignment).
///
/// Resources in the same group have no dependencies on each other and can
/// execute in parallel.
pub(crate) fn cmd_graph_resource_dependency_parallel_groups(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    if config.resources.is_empty() {
        if json {
            println!("{{\"parallel_groups\":[]}}");
        } else {
            println!("No resources to parallelize.");
        }
        return Ok(());
    }

    // Build in-degree map and forward adjacency (dep -> list of dependents).
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut forward: HashMap<String, Vec<String>> = HashMap::new();
    for (name, resource) in &config.resources {
        in_degree.entry(name.clone()).or_insert(0);
        forward.entry(name.clone()).or_default();
        for dep in &resource.depends_on {
            forward.entry(dep.clone()).or_default().push(name.clone());
            *in_degree.entry(name.clone()).or_default() += 1;
        }
    }

    // Kahn's algorithm with level tracking.
    // Seed queue with all zero-in-degree nodes tagged at group 0.
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut roots: Vec<String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    roots.sort();
    for r in roots {
        queue.push_back((r, 0));
    }

    let mut groups: Vec<Vec<String>> = Vec::new();

    while let Some((node, level)) = queue.pop_front() {
        // Ensure the groups vector is large enough.
        while groups.len() <= level {
            groups.push(Vec::new());
        }
        groups[level].push(node.clone());

        let mut children: Vec<String> = forward.get(&node).cloned().unwrap_or_default();
        children.sort();
        for child in children {
            if let Some(d) = in_degree.get_mut(&child) {
                *d -= 1;
                if *d == 0 {
                    queue.push_back((child, level + 1));
                }
            }
        }
    }

    // Sort resources within each group for deterministic output.
    for group in &mut groups {
        group.sort();
    }

    if json {
        let items: Vec<String> = groups
            .iter()
            .enumerate()
            .map(|(i, rs)| {
                let names: Vec<String> = rs.iter().map(|r| format!("\"{}\"", r)).collect();
                format!("{{\"group\":{},\"resources\":[{}]}}", i, names.join(","))
            })
            .collect();
        println!("{{\"parallel_groups\":[{}]}}", items.join(","));
    } else {
        for (i, rs) in groups.iter().enumerate() {
            println!("Group {} (parallel): {}", i, rs.join(", "));
        }
    }

    Ok(())
}

/// FJ-1026: Estimate total execution cost by weighting resources by type and
/// computing the critical path (longest weighted path through the DAG).
pub(crate) fn cmd_graph_resource_dependency_execution_cost(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    if config.resources.is_empty() {
        if json {
            println!("{{\"execution_cost\":{{\"total_resources\":0,\"total_cost\":0,\"critical_path_cost\":0,\"critical_path\":[]}}}}");
        } else {
            println!("No resources to analyze.");
        }
        return Ok(());
    }

    let total_resources = config.resources.len();

    // Compute individual costs and total cost.
    let costs: HashMap<String, u32> = config
        .resources
        .iter()
        .map(|(name, res)| (name.clone(), type_cost(&res.resource_type)))
        .collect();

    let total_cost: u32 = costs.values().sum();

    // Find the critical path (longest weighted path) using DFS with memoization.
    // For each node, compute the maximum cost path starting from that node
    // (following reverse dependency direction: node -> its dependents).
    // Actually, the critical path is the longest path from any root to any leaf,
    // weighted by node costs. We compute it by finding the longest path ending
    // at each node (sum of costs along the path from a root to that node).
    let mut best_cost: HashMap<String, u32> = HashMap::new();
    let mut best_path: HashMap<String, Vec<String>> = HashMap::new();

    fn longest_path_to(
        name: &str,
        resources: &indexmap::IndexMap<String, types::Resource>,
        costs: &HashMap<String, u32>,
        best_cost: &mut HashMap<String, u32>,
        best_path: &mut HashMap<String, Vec<String>>,
        visiting: &mut std::collections::HashSet<String>,
    ) -> (u32, Vec<String>) {
        if let Some(&c) = best_cost.get(name) {
            return (c, best_path.get(name).cloned().unwrap_or_default());
        }
        if visiting.contains(name) {
            // Cycle protection — return just this node's cost.
            let c = costs.get(name).copied().unwrap_or(1);
            return (c, vec![name.to_string()]);
        }
        visiting.insert(name.to_string());

        let node_cost = costs.get(name).copied().unwrap_or(1);
        let res = match resources.get(name) {
            Some(r) => r,
            None => {
                best_cost.insert(name.to_string(), node_cost);
                best_path.insert(name.to_string(), vec![name.to_string()]);
                return (node_cost, vec![name.to_string()]);
            }
        };

        if res.depends_on.is_empty() {
            // Root node — path is just itself.
            best_cost.insert(name.to_string(), node_cost);
            best_path.insert(name.to_string(), vec![name.to_string()]);
            return (node_cost, vec![name.to_string()]);
        }

        // Find the dependency with the longest path to it.
        let mut max_dep_cost = 0u32;
        let mut max_dep_path: Vec<String> = Vec::new();
        for dep in &res.depends_on {
            let (dc, dp) = longest_path_to(dep, resources, costs, best_cost, best_path, visiting);
            if dc > max_dep_cost {
                max_dep_cost = dc;
                max_dep_path = dp;
            }
        }

        let total = max_dep_cost + node_cost;
        let mut path = max_dep_path;
        path.push(name.to_string());
        best_cost.insert(name.to_string(), total);
        best_path.insert(name.to_string(), path.clone());
        (total, path)
    }

    let names: Vec<String> = config.resources.keys().cloned().collect();
    for name in &names {
        let mut visiting = std::collections::HashSet::new();
        longest_path_to(
            name,
            &config.resources,
            &costs,
            &mut best_cost,
            &mut best_path,
            &mut visiting,
        );
    }

    // Find the node with the highest accumulated cost — that's the critical path endpoint.
    let (critical_path_cost, critical_path) = names
        .iter()
        .map(|n| {
            let c = best_cost.get(n).copied().unwrap_or(0);
            let p = best_path.get(n).cloned().unwrap_or_default();
            (c, p)
        })
        .max_by_key(|(c, _)| *c)
        .unwrap_or((0, Vec::new()));

    if json {
        let path_items: Vec<String> = critical_path.iter().map(|n| format!("\"{}\"", n)).collect();
        println!(
            "{{\"execution_cost\":{{\"total_resources\":{},\"total_cost\":{},\"critical_path_cost\":{},\"critical_path\":[{}]}}}}",
            total_resources, total_cost, critical_path_cost, path_items.join(",")
        );
    } else {
        println!("Total resources: {}", total_resources);
        println!("Total estimated cost: {}", total_cost);
        println!(
            "Critical path cost: {} (path: {})",
            critical_path_cost,
            critical_path.join(" -> ")
        );
    }

    Ok(())
}
