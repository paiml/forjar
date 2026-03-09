//! FJ-1415: Cost estimation and resource budgeting.
//!
//! Static analysis of resource costs based on resource types, counts,
//! and declared budgets. Sovereign advantage: no cloud API calls needed.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

pub(crate) fn cmd_cost_estimate(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let mut items = Vec::new();
    let mut total_resources = 0_usize;
    let mut total_machines = std::collections::HashSet::new();

    for (id, resource) in &config.resources {
        for m in resource.machine.iter() {
            total_machines.insert(m.to_owned());
        }

        let cost = estimate_resource_cost(resource);
        total_resources += 1;

        items.push(CostItem {
            id: id.clone(),
            resource_type: format!("{:?}", resource.resource_type),
            complexity: cost.complexity,
            estimated_seconds: cost.estimated_seconds,
            category: cost.category,
        });
    }

    let total_seconds: u64 = items.iter().map(|i| i.estimated_seconds).sum();

    if json {
        print_cost_json(
            &items,
            total_resources,
            total_machines.len(),
            total_seconds,
            &config.name,
        );
    } else {
        print_cost_text(
            &items,
            total_resources,
            total_machines.len(),
            total_seconds,
            &config.name,
        );
    }

    Ok(())
}

struct CostEstimate {
    complexity: &'static str,
    estimated_seconds: u64,
    category: &'static str,
}

fn estimate_resource_cost(resource: &types::Resource) -> CostEstimate {
    match resource.resource_type {
        types::ResourceType::Package => CostEstimate {
            complexity: "medium",
            estimated_seconds: 30,
            category: "package-management",
        },
        types::ResourceType::File => CostEstimate {
            complexity: "low",
            estimated_seconds: 2,
            category: "file-management",
        },
        types::ResourceType::Service => CostEstimate {
            complexity: "medium",
            estimated_seconds: 10,
            category: "service-management",
        },
        types::ResourceType::Mount => CostEstimate {
            complexity: "low",
            estimated_seconds: 5,
            category: "filesystem",
        },
        types::ResourceType::Task => {
            let secs = resource.timeout.unwrap_or(60);
            CostEstimate {
                complexity: "variable",
                estimated_seconds: secs,
                category: "task-execution",
            }
        }
        types::ResourceType::Model => CostEstimate {
            complexity: "high",
            estimated_seconds: 300,
            category: "ml-infrastructure",
        },
        types::ResourceType::Gpu => CostEstimate {
            complexity: "high",
            estimated_seconds: 60,
            category: "gpu-management",
        },
        types::ResourceType::Docker => CostEstimate {
            complexity: "medium",
            estimated_seconds: 30,
            category: "container-management",
        },
        _ => CostEstimate {
            complexity: "low",
            estimated_seconds: 5,
            category: "general",
        },
    }
}

struct CostItem {
    id: String,
    resource_type: String,
    complexity: &'static str,
    estimated_seconds: u64,
    category: &'static str,
}

fn print_cost_json(
    items: &[CostItem],
    resources: usize,
    machines: usize,
    total_secs: u64,
    name: &str,
) {
    let entries: Vec<String> = items
        .iter()
        .map(|i| {
            format!(
                r#"{{"id":"{id}","type":"{rt}","complexity":"{cx}","seconds":{s},"category":"{cat}"}}"#,
                id = i.id,
                rt = i.resource_type,
                cx = i.complexity,
                s = i.estimated_seconds,
                cat = i.category,
            )
        })
        .collect();

    println!(
        r#"{{"stack":"{name}","resources":{resources},"machines":{machines},"total_seconds":{total_secs},"items":[{e}]}}"#,
        e = entries.join(","),
    );
}

fn print_cost_text(
    items: &[CostItem],
    resources: usize,
    machines: usize,
    total_secs: u64,
    name: &str,
) {
    println!("{}\n", bold("Cost Estimation"));
    println!("  Stack:     {}", bold(name));
    println!("  Resources: {resources}");
    println!("  Machines:  {machines}");
    println!("  Est. time: {total_secs}s (sequential)\n");

    for item in items {
        let icon = match item.complexity {
            "high" => red("H"),
            "medium" => yellow("M"),
            _ => green("L"),
        };
        println!(
            "  {icon} {} ({}) ~{}s [{}]",
            item.id, item.resource_type, item.estimated_seconds, item.category
        );
    }

    println!(
        "\n  {} Estimates based on static analysis; actual time depends on network/system load",
        dim("Note:")
    );
}
