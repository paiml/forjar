//! FJ-1429: Stack dependency graph.
//!
//! DAG of configs: networking → compute → storage.
//! Cycle detection, parallel independent stacks, serial dependent stacks.

use super::helpers::*;
use std::collections::{BTreeMap, BTreeSet};

/// A stack node in the dependency graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StackNode {
    pub name: String,
    pub path: String,
    pub resources: usize,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
}

/// Stack dependency graph report.
#[derive(Debug, serde::Serialize)]
pub struct StackGraphReport {
    pub nodes: Vec<StackNode>,
    pub total_stacks: usize,
    pub total_resources: usize,
    pub has_cycles: bool,
    pub parallel_groups: Vec<Vec<String>>,
}

/// Analyze stack dependency graph.
pub fn cmd_stack_graph(files: &[std::path::PathBuf], json: bool) -> Result<(), String> {
    let nodes = build_graph(files)?;
    let has_cycles = detect_cycles(&nodes);
    let parallel_groups = compute_parallel_groups(&nodes);
    let total_resources: usize = nodes.iter().map(|n| n.resources).sum();

    let report = StackGraphReport {
        total_stacks: nodes.len(),
        total_resources,
        has_cycles,
        parallel_groups,
        nodes,
    };

    if json {
        let out = serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_stack_graph(&report);
    }

    if has_cycles {
        Err("cycle detected in stack dependency graph".to_string())
    } else {
        Ok(())
    }
}

fn build_graph(files: &[std::path::PathBuf]) -> Result<Vec<StackNode>, String> {
    let mut configs = Vec::new();
    for f in files {
        let config = parse_and_validate(f)?;
        let deps = extract_deps(&config);
        configs.push((
            config.name.clone(),
            f.display().to_string(),
            config.resources.len(),
            deps,
        ));
    }

    let all_names: BTreeSet<String> = configs.iter().map(|(n, _, _, _)| n.clone()).collect();

    let mut nodes = Vec::new();
    for (name, path, resources, deps) in &configs {
        let dependents = find_dependents(name, &configs);
        nodes.push(StackNode {
            name: name.clone(),
            path: path.clone(),
            resources: *resources,
            dependencies: deps
                .iter()
                .filter(|d| all_names.contains(*d))
                .cloned()
                .collect(),
            dependents,
        });
    }
    Ok(nodes)
}

fn extract_deps(config: &crate::core::types::ForjarConfig) -> Vec<String> {
    let mut deps = Vec::new();
    for (_key, ds) in &config.data {
        if ds.source_type == crate::core::types::DataSourceType::ForjarState {
            if let Some(ref cfg_name) = ds.config {
                deps.push(cfg_name.clone());
            }
        }
    }
    deps.sort();
    deps.dedup();
    deps
}

fn find_dependents(name: &str, configs: &[(String, String, usize, Vec<String>)]) -> Vec<String> {
    configs
        .iter()
        .filter(|(_, _, _, deps)| deps.contains(&name.to_string()))
        .map(|(n, _, _, _)| n.clone())
        .collect()
}

fn detect_cycles(nodes: &[StackNode]) -> bool {
    let dep_map: BTreeMap<&str, &[String]> = nodes
        .iter()
        .map(|n| (n.name.as_str(), n.dependencies.as_slice()))
        .collect();

    for node in nodes {
        if has_cycle(&node.name, &dep_map, &mut BTreeSet::new()) {
            return true;
        }
    }
    false
}

fn has_cycle(
    name: &str,
    deps: &BTreeMap<&str, &[String]>,
    visiting: &mut BTreeSet<String>,
) -> bool {
    if visiting.contains(name) {
        return true;
    }
    visiting.insert(name.to_string());
    if let Some(node_deps) = deps.get(name) {
        for d in *node_deps {
            if has_cycle(d, deps, visiting) {
                return true;
            }
        }
    }
    visiting.remove(name);
    false
}

fn compute_parallel_groups(nodes: &[StackNode]) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut placed: BTreeSet<String> = BTreeSet::new();
    let mut remaining: Vec<&StackNode> = nodes.iter().collect();

    for _i in 0..100 {
        if remaining.is_empty() {
            break;
        }
        let (group, still) = partition_wave(&remaining, &placed);
        if group.is_empty() {
            groups.push(still.iter().map(|n| n.name.clone()).collect());
            break;
        }
        for name in &group {
            placed.insert(name.clone());
        }
        groups.push(group);
        remaining = still;
    }
    groups
}

fn partition_wave<'a>(
    remaining: &[&'a StackNode],
    placed: &BTreeSet<String>,
) -> (Vec<String>, Vec<&'a StackNode>) {
    let mut group = Vec::new();
    let mut still = Vec::new();
    for n in remaining {
        if n.dependencies.iter().all(|d| placed.contains(d)) {
            group.push(n.name.clone());
        } else {
            still.push(*n);
        }
    }
    (group, still)
}

fn print_stack_graph(report: &StackGraphReport) {
    println!("Stack Dependency Graph");
    println!("======================");
    println!("Stacks: {}", report.total_stacks);
    println!("Resources: {}", report.total_resources);
    println!("Cycles: {}", if report.has_cycles { "YES" } else { "none" });
    println!();
    for n in &report.nodes {
        let deps = if n.dependencies.is_empty() {
            "none".to_string()
        } else {
            n.dependencies.join(", ")
        };
        println!("  {} ({} resources, deps: {})", n.name, n.resources, deps);
    }
    println!();
    println!("Parallel Groups:");
    for (i, g) in report.parallel_groups.iter().enumerate() {
        println!("  Group {i}: {}", g.join(", "));
    }
}
