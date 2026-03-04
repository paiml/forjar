//! FJ-1428: Multi-config apply ordering.
//!
//! `forjar multi-apply` loads multiple config files, builds a cross-config
//! dependency graph, and determines execution order.

use super::helpers::*;
#[cfg(test)]
use std::collections::BTreeMap;
use std::path::Path;

/// A config in the multi-config graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfigNode {
    pub name: String,
    pub path: String,
    pub resources: usize,
    pub machines: Vec<String>,
    pub depends_on: Vec<String>,
}

/// Multi-config execution plan.
#[derive(Debug, serde::Serialize)]
pub struct MultiConfigPlan {
    pub configs: Vec<ConfigNode>,
    pub execution_order: Vec<Vec<String>>,
    pub total_configs: usize,
    pub total_resources: usize,
}

/// Analyze multi-config dependencies and generate execution plan.
pub fn cmd_multi_config(files: &[std::path::PathBuf], json: bool) -> Result<(), String> {
    let configs = load_configs(files)?;
    let order = compute_execution_order(&configs);
    let total_resources: usize = configs.iter().map(|c| c.resources).sum();

    let plan = MultiConfigPlan {
        total_configs: configs.len(),
        total_resources,
        configs,
        execution_order: order,
    };

    if json {
        let out = serde_json::to_string_pretty(&plan).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_multi_config_plan(&plan);
    }
    Ok(())
}

fn load_configs(files: &[std::path::PathBuf]) -> Result<Vec<ConfigNode>, String> {
    let mut nodes = Vec::new();
    for f in files {
        let config = parse_and_validate(f)?;
        let machines = collect_machines(&config);
        let deps = extract_cross_config_deps(&config, files, f);
        nodes.push(ConfigNode {
            name: config.name.clone(),
            path: f.display().to_string(),
            resources: config.resources.len(),
            machines,
            depends_on: deps,
        });
    }
    Ok(nodes)
}

fn collect_machines(config: &crate::core::types::ForjarConfig) -> Vec<String> {
    config.machines.keys().cloned().collect()
}

fn extract_cross_config_deps(
    config: &crate::core::types::ForjarConfig,
    _all_files: &[std::path::PathBuf],
    _current: &Path,
) -> Vec<String> {
    // Check config-level data sources for forjar-state references
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

fn compute_execution_order(configs: &[ConfigNode]) -> Vec<Vec<String>> {
    let mut waves: Vec<Vec<String>> = Vec::new();
    let mut placed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut remaining: Vec<&ConfigNode> = configs.iter().collect();

    for _iteration in 0..100 {
        if remaining.is_empty() {
            break;
        }
        let (wave, still_remaining) = partition_ready(&remaining, &placed);
        if wave.is_empty() {
            waves.push(still_remaining.iter().map(|c| c.name.clone()).collect());
            break;
        }
        for name in &wave {
            placed.insert(name.clone());
        }
        waves.push(wave);
        remaining = still_remaining;
    }
    waves
}

fn partition_ready<'a>(
    remaining: &[&'a ConfigNode],
    placed: &std::collections::BTreeSet<String>,
) -> (Vec<String>, Vec<&'a ConfigNode>) {
    let mut wave = Vec::new();
    let mut still = Vec::new();
    for c in remaining {
        if c.depends_on.iter().all(|d| placed.contains(d)) {
            wave.push(c.name.clone());
        } else {
            still.push(*c);
        }
    }
    (wave, still)
}

fn print_multi_config_plan(plan: &MultiConfigPlan) {
    println!("Multi-Config Execution Plan");
    println!("===========================");
    println!("Configs: {}", plan.total_configs);
    println!("Resources: {}", plan.total_resources);
    println!();
    for c in &plan.configs {
        let deps = if c.depends_on.is_empty() {
            "none".to_string()
        } else {
            c.depends_on.join(", ")
        };
        println!("  {} ({} resources, deps: {})", c.name, c.resources, deps);
    }
    println!();
    println!("Execution Order:");
    for (i, wave) in plan.execution_order.iter().enumerate() {
        println!("  Wave {i}: {}", wave.join(", "));
    }
}

/// Build a dependency map for stack ordering.
#[cfg(test)]
pub fn build_stack_deps(configs: &[ConfigNode]) -> BTreeMap<String, Vec<String>> {
    configs
        .iter()
        .map(|c| (c.name.clone(), c.depends_on.clone()))
        .collect()
}
