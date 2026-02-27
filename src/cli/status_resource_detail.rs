//! Resource detail.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use std::collections::HashMap;


/// FJ-593: Show resource timeline from event log.
pub(crate) fn cmd_status_resource_timeline(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut events: Vec<(String, String, String, String)> = Vec::new(); // (timestamp, machine, resource, status)

    for m in &machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let log_path = state_dir.join(format!("{}.events.jsonl", m));
        if !log_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        for line in content.lines() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                let ts = val
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let resource = val
                    .get("resource")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let status = val
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                events.push((ts, m.clone(), resource, status));
            }
        }
    }

    if json {
        let items: Vec<String> = events
            .iter()
            .map(|(ts, m, r, s)| {
                format!(
                    r#"{{"timestamp":"{}","machine":"{}","resource":"{}","status":"{}"}}"#,
                    ts, m, r, s
                )
            })
            .collect();
        println!(
            r#"{{"timeline":[{}],"count":{}}}"#,
            items.join(","),
            events.len()
        );
    } else if events.is_empty() {
        println!("No timeline events found");
    } else {
        println!("Resource timeline ({} events):", events.len());
        for (ts, m, r, s) in &events {
            println!("  [{}] {}:{} — {}", ts, m, r, s);
        }
    }
    Ok(())
}


/// FJ-627: Show runtime dependency graph from lock files.
pub(crate) fn cmd_status_resource_dependencies(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let mut deps: Vec<(String, String, usize)> = Vec::new(); // (machine, resource, dep_count)

    for m in &machines {
        if let Some(filter) = machine {
            if m != filter {
                continue;
            }
        }
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
            for (rname, _) in &lock.resources {
                deps.push((m.clone(), rname.clone(), 0));
            }
        }
    }

    if json {
        let items: Vec<String> = deps
            .iter()
            .map(|(m, r, d)| {
                format!(
                    r#"{{"machine":"{}","resource":"{}","dependencies":{}}}"#,
                    m, r, d
                )
            })
            .collect();
        println!(
            r#"{{"resource_dependencies":[{}],"count":{}}}"#,
            items.join(","),
            deps.len()
        );
    } else if deps.is_empty() {
        println!("No resource dependency data available");
    } else {
        println!("Resource dependencies ({} resources):", deps.len());
        for (m, r, _) in &deps {
            println!("  {}:{}", m, r);
        }
    }
    Ok(())
}


/// FJ-712: Show resource input fields per resource
fn collect_resource_inputs(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, String, String, usize)> {
    let mut result = Vec::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if let Ok(data) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&data) {
                for (name, rl) in &lock.resources {
                    result.push((
                        m.to_string(),
                        name.clone(),
                        format!("{:?}", rl.resource_type),
                        rl.details.len(),
                    ));
                }
            }
        }
    }
    result
}

pub(crate) fn cmd_status_resource_inputs(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let inputs = collect_resource_inputs(state_dir, &targets);
    if json {
        let entries: Vec<String> = inputs.iter().map(|(m, name, rtype, count)| {
            format!(
                "{{\"machine\":\"{}\",\"resource\":\"{}\",\"type\":\"{}\",\"input_count\":{}}}",
                m, name, rtype, count
            )
        }).collect();
        println!("{{\"resource_inputs\":[{}]}}", entries.join(","));
    } else {
        println!("Resource inputs:");
        let mut current_machine = String::new();
        for (m, name, rtype, count) in &inputs {
            if *m != current_machine {
                println!("  Machine: {}", m);
                current_machine = m.clone();
            }
            println!("    {} ({}) — {} input(s)", name, rtype, count);
        }
    }
    Ok(())
}


/// FJ-727: Show count per resource type
fn tally_resource_types(
    state_dir: &Path,
    targets: &[&String],
) -> Vec<(String, usize)> {
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for m in targets {
        let lock_path = state_dir.join(format!("{}.lock.yaml", m));
        if let Ok(data) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<types::StateLock>(&data) {
                for rl in lock.resources.values() {
                    let rtype = format!("{:?}", rl.resource_type);
                    *type_counts.entry(rtype).or_default() += 1;
                }
            }
        }
    }
    let mut sorted: Vec<_> = type_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    sorted
}

pub(crate) fn cmd_status_resource_types_summary(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let machines = discover_machines(state_dir);
    let targets: Vec<&String> = match machine {
        Some(m) => machines.iter().filter(|x| x.as_str() == m).collect(),
        None => machines.iter().collect(),
    };
    let sorted = tally_resource_types(state_dir, &targets);
    if json {
        let entries: Vec<String> = sorted
            .iter()
            .map(|(t, c)| format!("{{\"type\":\"{}\",\"count\":{}}}", t, c))
            .collect();
        println!("{{\"resource_types\":[{}]}}", entries.join(","));
    } else if sorted.is_empty() {
        println!("No resources found.");
    } else {
        println!("Resource types summary:");
        for (rtype, count) in &sorted {
            println!("  {} — {}", rtype, count);
        }
    }
    Ok(())
}

