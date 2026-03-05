//! FJ-1451: Dependency impact analysis.
//!
//! BFS through reverse dependency graph to compute blast radius
//! for a specific resource change. Shows affected resources,
//! machines, risk level, and estimated cascade time.

use super::helpers::*;
use crate::core::types;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

struct AffectedResource {
    name: String,
    resource_type: String,
    machine: String,
    depth: usize,
    estimated_seconds: u64,
}

struct ImpactReport {
    source: String,
    affected: Vec<AffectedResource>,
    risk_level: &'static str,
    total_affected: usize,
    machines_affected: usize,
    estimated_cascade_seconds: u64,
}

fn build_reverse_deps(config: &types::ForjarConfig) -> HashMap<String, Vec<String>> {
    let mut rev: HashMap<String, Vec<String>> = HashMap::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            rev.entry(dep.clone()).or_default().push(name.clone());
        }
    }
    rev
}

fn compute_impact(resource: &str, config: &types::ForjarConfig) -> ImpactReport {
    let rev_deps = build_reverse_deps(config);
    let mut affected = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back((resource.to_string(), 0_usize));
    visited.insert(resource.to_string());

    while let Some((current, depth)) = queue.pop_front() {
        if current != resource {
            let res = &config.resources[&current];
            let machine = res.machine.to_vec().first().cloned().unwrap_or_default();
            let est_secs = estimate_resource_seconds(res);
            affected.push(AffectedResource {
                name: current.clone(),
                resource_type: format!("{:?}", res.resource_type).to_lowercase(),
                machine,
                depth,
                estimated_seconds: est_secs,
            });
        }
        if let Some(deps) = rev_deps.get(&current) {
            for dep in deps {
                if visited.insert(dep.clone()) {
                    queue.push_back((dep.clone(), depth + 1));
                }
            }
        }
    }

    affected.sort_by(|a, b| a.depth.cmp(&b.depth).then(a.name.cmp(&b.name)));

    let total_affected = affected.len();
    let machines_affected_count = {
        let set: HashSet<&str> = affected.iter().map(|a| a.machine.as_str()).collect();
        set.len()
    };
    let estimated_cascade_seconds: u64 = affected.iter().map(|a| a.estimated_seconds).sum();

    let risk_level = match total_affected {
        0 => "none",
        1..=3 => "low",
        4..=10 => "medium",
        11..=25 => "high",
        _ => "critical",
    };

    ImpactReport {
        source: resource.to_string(),
        affected,
        risk_level,
        total_affected,
        machines_affected: machines_affected_count,
        estimated_cascade_seconds,
    }
}

fn estimate_resource_seconds(res: &types::Resource) -> u64 {
    match res.resource_type {
        types::ResourceType::Package => 30,
        types::ResourceType::File => 2,
        types::ResourceType::Service => 10,
        types::ResourceType::Task => res.timeout.unwrap_or(60),
        types::ResourceType::Model => 300,
        types::ResourceType::Gpu => 60,
        types::ResourceType::Docker => 30,
        _ => 5,
    }
}

pub(crate) fn cmd_impact(file: &Path, resource: &str, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    if !config.resources.contains_key(resource) {
        return Err(format!("Resource '{resource}' not found in config"));
    }

    let report = compute_impact(resource, &config);

    if json {
        print_impact_json(&report);
    } else {
        print_impact_text(&report);
    }
    Ok(())
}

fn print_impact_json(r: &ImpactReport) {
    let items: Vec<String> = r
        .affected
        .iter()
        .map(|a| {
            format!(
                r#"{{"name":"{}","type":"{}","machine":"{}","depth":{},"seconds":{}}}"#,
                a.name, a.resource_type, a.machine, a.depth, a.estimated_seconds
            )
        })
        .collect();
    println!(
        r#"{{"source":"{}","risk":"{}","total_affected":{},"machines_affected":{},"cascade_seconds":{},"affected":[{}]}}"#,
        r.source,
        r.risk_level,
        r.total_affected,
        r.machines_affected,
        r.estimated_cascade_seconds,
        items.join(","),
    );
}

fn print_impact_text(r: &ImpactReport) {
    println!("{}\n", bold("Dependency Impact Analysis"));
    println!("  Source:    {}", bold(&r.source));

    let risk_colored = match r.risk_level {
        "critical" | "high" => red(r.risk_level),
        "medium" => yellow(r.risk_level),
        _ => green(r.risk_level),
    };
    println!("  Risk:      {risk_colored}");
    println!("  Affected:  {} resource(s)", r.total_affected);
    println!("  Machines:  {} machine(s)", r.machines_affected);
    println!("  Est. cascade: {}s\n", r.estimated_cascade_seconds);

    for a in &r.affected {
        let depth_marker = "  ".repeat(a.depth);
        println!(
            "  {}> {} [{}] on {} (~{}s)",
            depth_marker, a.name, a.resource_type, a.machine, a.estimated_seconds
        );
    }

    if r.affected.is_empty() {
        println!(
            "  {} No downstream resources depend on '{}'",
            green("OK"),
            r.source
        );
    }
}
