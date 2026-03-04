//! FJ-1424: Cross-machine resource dependency analysis.
//!
//! `forjar cross-deps` analyzes cross-machine resource dependencies in a config.
//! Resources on machine A can depend on resources on machine B via depends_on.
//! This command validates, visualizes, and reports cross-machine dependency chains.

use super::helpers::*;
use std::collections::BTreeMap;
use std::path::Path;

/// A cross-machine dependency edge.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CrossDep {
    pub from_resource: String,
    pub from_machine: String,
    pub to_resource: String,
    pub to_machine: String,
    pub dep_type: String,
}

/// Cross-machine dependency report.
#[derive(Debug, serde::Serialize)]
pub struct CrossDepReport {
    pub edges: Vec<CrossDep>,
    pub total_resources: usize,
    pub cross_machine_deps: usize,
    pub same_machine_deps: usize,
    pub machines_involved: Vec<String>,
    pub execution_waves: Vec<Vec<String>>,
}

/// Analyze cross-machine dependencies.
pub fn cmd_cross_deps(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let res_machine = build_resource_machine_map(&config);
    let (edges, cross_count, same_count, machines) = analyze_deps(&config, &res_machine);
    let waves = build_execution_waves(&config);

    let report = CrossDepReport {
        edges,
        total_resources: config.resources.len(),
        cross_machine_deps: cross_count,
        same_machine_deps: same_count,
        machines_involved: machines.into_iter().collect(),
        execution_waves: waves,
    };

    if json {
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{output}");
    } else {
        print_cross_dep_report(&report);
    }

    Ok(())
}

fn build_resource_machine_map(
    config: &crate::core::types::ForjarConfig,
) -> BTreeMap<String, Vec<String>> {
    config
        .resources
        .iter()
        .map(|(id, res)| (id.clone(), res.machine.to_vec()))
        .collect()
}

fn analyze_deps(
    config: &crate::core::types::ForjarConfig,
    res_machine: &BTreeMap<String, Vec<String>>,
) -> (Vec<CrossDep>, usize, usize, std::collections::BTreeSet<String>) {
    let mut edges = Vec::new();
    let mut cross_count = 0usize;
    let mut same_count = 0usize;
    let mut machines = std::collections::BTreeSet::new();

    for (id, res) in &config.resources {
        let my_machines = res.machine.to_vec();
        for m in &my_machines {
            machines.insert(m.clone());
        }
        for dep in &res.depends_on {
            let (c, s) = classify_dep(id, &my_machines, dep, res_machine, &mut edges);
            cross_count += c;
            same_count += s;
        }
    }
    (edges, cross_count, same_count, machines)
}

fn classify_dep(
    id: &str,
    my_machines: &[String],
    dep: &str,
    res_machine: &BTreeMap<String, Vec<String>>,
    edges: &mut Vec<CrossDep>,
) -> (usize, usize) {
    let Some(dep_machines) = res_machine.get(dep) else {
        return (0, 0);
    };
    let is_cross = !my_machines.iter().all(|m| dep_machines.contains(m));
    if !is_cross {
        return (0, 1);
    }
    for from_m in my_machines {
        for to_m in dep_machines.iter().filter(|t| *t != from_m) {
            edges.push(CrossDep {
                from_resource: id.to_string(),
                from_machine: from_m.clone(),
                to_resource: dep.to_string(),
                to_machine: to_m.clone(),
                dep_type: "cross-machine".to_string(),
            });
        }
    }
    (1, 0)
}

fn build_execution_waves(config: &crate::core::types::ForjarConfig) -> Vec<Vec<String>> {
    let mut waves: Vec<Vec<String>> = Vec::new();
    let mut placed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    // Simple layered topological ordering
    let ids: Vec<String> = config.resources.keys().cloned().collect();
    let mut remaining: Vec<String> = ids;
    let mut iteration = 0;

    while !remaining.is_empty() && iteration < 100 {
        let mut wave = Vec::new();
        let mut still_remaining = Vec::new();

        for id in &remaining {
            if let Some(res) = config.resources.get(id) {
                let deps_met = res.depends_on.iter().all(|d| placed.contains(d.as_str()));
                if deps_met {
                    wave.push(id.clone());
                } else {
                    still_remaining.push(id.clone());
                }
            }
        }

        if wave.is_empty() {
            // Circular dependency or unreachable — dump remaining
            waves.push(still_remaining);
            break;
        }

        for id in &wave {
            placed.insert(id.clone());
        }
        waves.push(wave);
        remaining = still_remaining;
        iteration += 1;
    }

    waves
}

fn print_cross_dep_report(report: &CrossDepReport) {
    println!("Cross-Machine Dependency Report");
    println!("===============================");
    println!("Resources: {}", report.total_resources);
    println!("Cross-machine deps: {}", report.cross_machine_deps);
    println!("Same-machine deps: {}", report.same_machine_deps);
    println!("Machines: {}", report.machines_involved.join(", "));
    println!();

    if !report.edges.is_empty() {
        println!("Cross-Machine Edges:");
        for e in &report.edges {
            println!(
                "  {} ({}) -> {} ({})",
                e.from_resource, e.from_machine, e.to_resource, e.to_machine
            );
        }
        println!();
    }

    println!("Execution Waves:");
    for (i, wave) in report.execution_waves.iter().enumerate() {
        println!("  Wave {i}: {}", wave.join(", "));
    }
}

