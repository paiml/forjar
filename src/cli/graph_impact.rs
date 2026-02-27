//! Impact analysis.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use std::collections::HashMap;


/// Print dependency matrix as JSON.
fn print_dependency_matrix_json(
    names: &[String],
    config: &types::ForjarConfig,
) {
    let mut rows: Vec<String> = Vec::new();
    for name in names {
        let res = &config.resources[name];
        let deps: Vec<String> = res
            .depends_on
            .iter()
            .map(|d| format!(r#""{}""#, d))
            .collect();
        rows.push(format!(
            r#"{{"resource":"{}","depends_on":[{}]}}"#,
            name,
            deps.join(",")
        ));
    }
    println!("[{}]", rows.join(","));
}

/// Print dependency matrix as CSV.
fn print_dependency_matrix_csv(
    names: &[String],
    config: &types::ForjarConfig,
) {
    print!(",");
    println!("{}", names.join(","));
    for row_name in names {
        let res = &config.resources[row_name];
        let cells: Vec<&str> = names
            .iter()
            .map(|col| {
                if res.depends_on.contains(col) {
                    "1"
                } else {
                    "0"
                }
            })
            .collect();
        println!("{},{}", row_name, cells.join(","));
    }
}

/// Read event log files from state directory and count changes per resource.
fn count_resource_changes(state_dir: &std::path::Path) -> std::collections::HashMap<String, usize> {
    let mut change_count: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    if !state_dir.exists() {
        return change_count;
    }
    let entries = match std::fs::read_dir(state_dir) {
        Ok(entries) => entries,
        Err(_) => return change_count,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".events.jsonl") {
                count_events_in_file(&path, &mut change_count);
            }
        }
    }
    change_count
}

/// Parse a single event log file and count resource mentions.
fn count_events_in_file(
    path: &std::path::Path,
    change_count: &mut std::collections::HashMap<String, usize>,
) {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    for line in content.lines() {
        let parsed: serde_json::Value =
            serde_json::from_str(line).unwrap_or(serde_json::Value::Null);
        let resource = parsed["resource"].as_str().unwrap_or("").to_string();
        if !resource.is_empty() {
            *change_count.entry(resource).or_insert(0) += 1;
        }
    }
}

/// Format a hotspot count with appropriate coloring.
fn format_hotspot_heat(count: usize, max_count: usize) -> String {
    if count > max_count / 2 {
        red(&format!("{:>4}", count))
    } else if count > max_count / 4 {
        yellow(&format!("{:>4}", count))
    } else {
        format!("{:>4}", count)
    }
}

/// Find all resources that transitively depend on the given resource.
fn find_transitive_dependents(
    resource: &str,
    config: &types::ForjarConfig,
) -> std::collections::HashSet<String> {
    let mut dependents: std::collections::HashSet<String> = std::collections::HashSet::new();
    dependents.insert(resource.to_string());
    let mut changed = true;
    while changed {
        changed = false;
        for (name, res) in &config.resources {
            if dependents.contains(name) {
                continue;
            }
            for dep in &res.depends_on {
                if dependents.contains(dep) {
                    dependents.insert(name.clone());
                    changed = true;
                    break;
                }
            }
        }
    }
    dependents.remove(resource);
    dependents
}

/// Build reverse dependency map: for each resource, who depends on it?
fn build_reverse_deps(config: &types::ForjarConfig) -> std::collections::HashMap<String, Vec<String>> {
    let mut dependents: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            dependents
                .entry(dep.clone())
                .or_default()
                .push(name.clone());
        }
    }
    dependents
}

/// BFS from target through reverse deps, collecting all affected resources.
fn bfs_blast_radius(
    resource: &str,
    dependents: &std::collections::HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut affected: Vec<String> = Vec::new();
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();

    queue.push_back(resource.to_string());
    visited.insert(resource.to_string());

    while let Some(current) = queue.pop_front() {
        if current != resource {
            affected.push(current.clone());
        }
        if let Some(deps) = dependents.get(&current) {
            for dep in deps {
                if visited.insert(dep.clone()) {
                    queue.push_back(dep.clone());
                }
            }
        }
    }
    affected.sort();
    affected
}

// ── FJ-504: graph --impact-radius ──

pub(crate) fn cmd_graph_impact_radius(file: &Path, resource: &str) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    if !config.resources.contains_key(resource) {
        return Err(format!("Resource '{}' not found", resource));
    }
    let dependents = find_transitive_dependents(resource, &config);
    let total = config.resources.len();
    let pct = if total > 0 {
        (dependents.len() as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };
    println!("Impact radius for '{}':", resource);
    println!(
        "  {} dependent resource(s) ({:.0}% of total)",
        dependents.len(),
        pct
    );
    let mut sorted: Vec<&String> = dependents.iter().collect();
    sorted.sort();
    for d in &sorted {
        println!("  - {}", d);
    }
    Ok(())
}


/// FJ-514: Dependency matrix — output resource dependency matrix.
pub(crate) fn cmd_graph_dependency_matrix(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let names: Vec<String> = config.resources.keys().cloned().collect();

    if json {
        print_dependency_matrix_json(&names, &config);
    } else {
        print_dependency_matrix_csv(&names, &config);
    }
    Ok(())
}


/// FJ-524: Graph hotspots — highlight resources with most changes/failures.
pub(crate) fn cmd_graph_hotspots(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let state_dir = std::path::Path::new("state");
    let change_count = count_resource_changes(state_dir);

    let mut hotspots: Vec<(String, usize)> = config
        .resources
        .keys()
        .map(|name| {
            let count = change_count.get(name).copied().unwrap_or(0);
            (name.clone(), count)
        })
        .collect();

    hotspots.sort_by(|a, b| b.1.cmp(&a.1));

    println!("Resource hotspots (by change frequency):\n");
    let max_count = hotspots.first().map(|(_, c)| *c).unwrap_or(1).max(1);
    for (name, count) in &hotspots {
        let bar_len = (*count as f64 / max_count as f64 * 20.0) as usize;
        let bar: String = "█".repeat(bar_len);
        let heat = format_hotspot_heat(*count, max_count);
        println!("  {} {} {}", heat, bar, name);
    }
    Ok(())
}


/// FJ-534: Graph timeline — show resource application order as ASCII timeline.
pub(crate) fn cmd_graph_timeline(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;

    println!("Resource execution timeline:\n");

    let mut level = 0;
    let mut prev_deps: Vec<String> = Vec::new();

    for (i, name) in order.iter().enumerate() {
        let res = &config.resources[name];
        let has_new_deps =
            !res.depends_on.is_empty() && res.depends_on.iter().any(|d| !prev_deps.contains(d));

        if has_new_deps && i > 0 {
            level += 1;
        }

        let indent = "  ".repeat(level);
        let marker = if i == 0 {
            "┌"
        } else if i == order.len() - 1 {
            "└"
        } else {
            "├"
        };
        let type_str = format!("{:?}", res.resource_type).to_lowercase();
        println!("{}{}── {} [{}]", indent, marker, name, type_str);

        prev_deps = vec![name.clone()];
    }
    Ok(())
}


/// FJ-544: Graph what-if — simulate removing a resource, show impact.
pub(crate) fn cmd_graph_what_if(file: &Path, resource: &str) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    if !config.resources.contains_key(resource) {
        return Err(format!(
            "resource '{}' not found (available: {})",
            resource,
            config
                .resources
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Find all transitive dependents
    let mut affected: Vec<String> = Vec::new();
    let mut queue: Vec<String> = vec![resource.to_string()];

    while let Some(current) = queue.pop() {
        for (name, res) in &config.resources {
            if res.depends_on.contains(&current) && !affected.contains(name) {
                affected.push(name.clone());
                queue.push(name.clone());
            }
        }
    }

    println!("What-if analysis: removing '{}'\n", resource);

    if affected.is_empty() {
        println!(
            "  {} No other resources depend on '{}'.",
            green("✓"),
            resource
        );
    } else {
        println!(
            "  {} {} resources would be affected:\n",
            red("⚠"),
            affected.len()
        );
        for name in &affected {
            let type_str = format!("{:?}", config.resources[name].resource_type).to_lowercase();
            println!("    {} {} [{}]", red("✗"), name, type_str);
        }
    }
    Ok(())
}


/// FJ-554: Show all resources affected by a change to target (blast radius).
pub(crate) fn cmd_graph_blast_radius(file: &Path, resource: &str, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    if !config.resources.contains_key(resource) {
        return Err(format!("Resource '{}' not found in config", resource));
    }

    let dependents = build_reverse_deps(&config);
    let affected = bfs_blast_radius(resource, &dependents);

    if json {
        let items: Vec<String> = affected.iter().map(|a| format!(r#""{}""#, a)).collect();
        println!(
            r#"{{"resource":"{}","blast_radius":[{}],"count":{}}}"#,
            resource,
            items.join(","),
            affected.len()
        );
    } else if affected.is_empty() {
        println!("Blast radius for '{}': no dependent resources", resource);
    } else {
        println!(
            "Blast radius for '{}' ({} affected):",
            resource,
            affected.len()
        );
        for a in &affected {
            println!("  → {}", a);
        }
    }
    Ok(())
}

