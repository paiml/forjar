//! FJ-1450: Configuration complexity analysis.
//!
//! Static analysis of configuration complexity based on resource counts,
//! dependency depth, cross-machine deps, templates, and include depth.
//! Sovereign advantage: no runtime queries — pure config analysis.

use super::helpers::*;
use crate::core::{resolver, types};
use std::path::Path;

struct ComplexityReport {
    resource_count: usize,
    dependency_depth: usize,
    cross_machine_count: usize,
    template_count: usize,
    conditional_count: usize,
    include_depth: usize,
    machine_count: usize,
    total_score: u32,
    grade: &'static str,
    recommendations: Vec<String>,
}

fn compute_complexity(config: &types::ForjarConfig) -> ComplexityReport {
    let resource_count = config.resources.len();
    let machine_count = config.machines.len();
    let include_depth = config.includes.len();

    // Count cross-machine deps, conditionals, templates
    let mut cross_machine_count = 0_usize;
    let mut template_count = 0_usize;
    let mut conditional_count = 0_usize;

    for (_, res) in &config.resources {
        let res_machines = res.machine.to_vec();
        for dep_name in &res.depends_on {
            if let Some(dep_res) = config.resources.get(dep_name) {
                let dep_machines = dep_res.machine.to_vec();
                if res_machines != dep_machines {
                    cross_machine_count += 1;
                }
            }
        }
        if res.when.is_some() {
            conditional_count += 1;
        }
        if res.content.as_ref().is_some_and(|c| c.contains("{{")) {
            template_count += 1;
        }
        if res.path.as_ref().is_some_and(|p| p.contains("{{")) {
            template_count += 1;
        }
    }

    // Compute DAG depth
    let dependency_depth = resolver::build_execution_order(config)
        .map(|order| compute_dag_depth(config, &order))
        .unwrap_or(0);

    // Weighted score (0-100)
    let score = compute_score(&[
        resource_count,
        dependency_depth,
        cross_machine_count,
        template_count,
        conditional_count,
        include_depth,
        machine_count,
    ]);

    let grade = match score {
        0..=20 => "A",
        21..=40 => "B",
        41..=60 => "C",
        61..=80 => "D",
        _ => "F",
    };

    let mut recommendations = Vec::new();
    if resource_count > 50 {
        recommendations.push("Consider splitting into multiple configs (>50 resources)".into());
    }
    if dependency_depth > 8 {
        recommendations.push("Deep dependency chain (>8); consider flattening".into());
    }
    if cross_machine_count > 10 {
        recommendations.push("Many cross-machine deps (>10); consider grouping by machine".into());
    }

    ComplexityReport {
        resource_count,
        dependency_depth,
        cross_machine_count,
        template_count,
        conditional_count,
        include_depth,
        machine_count,
        total_score: score,
        grade,
        recommendations,
    }
}

fn compute_dag_depth(config: &types::ForjarConfig, order: &[String]) -> usize {
    let mut depths: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for name in order {
        let res = &config.resources[name];
        let max_dep = res
            .depends_on
            .iter()
            .filter_map(|d| depths.get(d.as_str()))
            .copied()
            .max()
            .unwrap_or(0);
        depths.insert(name, max_dep + 1);
    }
    depths.values().copied().max().unwrap_or(0)
}

/// Weighted complexity score from dimensions:
/// [resources, depth, cross, templates, conditionals, includes, machines]
fn compute_score(dims: &[usize; 7]) -> u32 {
    let weights: [(u32, u32); 7] = [(1, 30), (5, 20), (3, 15), (2, 10), (2, 10), (3, 10), (2, 5)];
    let total: u32 = dims
        .iter()
        .zip(weights.iter())
        .map(|(v, (w, cap))| ((*v as u32) * w).min(*cap))
        .sum();
    total.min(100)
}

pub(crate) fn cmd_complexity(file: &Path, json: bool) -> Result<(), String> {
    // Try full parse first; if includes fail, parse without includes
    // so we can still analyze the base config's complexity.
    let config = parse_and_validate(file).or_else(|_| {
        let raw =
            std::fs::read_to_string(file).map_err(|e| format!("read {}: {e}", file.display()))?;
        let mut c = crate::core::parser::parse_config(&raw)?;
        // Preserve original include count for scoring even though we couldn't resolve them
        let include_count = c.includes.len();
        c.includes.clear();
        let errors = crate::core::parser::validate_config(&c);
        if !errors.is_empty() {
            return Err(errors
                .iter()
                .map(|e| format!("{e}"))
                .collect::<Vec<_>>()
                .join("; "));
        }
        c.includes = vec!["_".to_string(); include_count]; // restore count for scoring
        Ok(c)
    })?;
    let report = compute_complexity(&config);

    if json {
        print_complexity_json(&report);
    } else {
        print_complexity_text(&report);
    }
    Ok(())
}

fn print_complexity_json(r: &ComplexityReport) {
    let recs: Vec<String> = r
        .recommendations
        .iter()
        .map(|s| format!(r#""{s}""#))
        .collect();
    println!(
        r#"{{"resources":{},"dependency_depth":{},"cross_machine":{},"templates":{},"conditionals":{},"includes":{},"machines":{},"score":{},"grade":"{}","recommendations":[{}]}}"#,
        r.resource_count,
        r.dependency_depth,
        r.cross_machine_count,
        r.template_count,
        r.conditional_count,
        r.include_depth,
        r.machine_count,
        r.total_score,
        r.grade,
        recs.join(","),
    );
}

fn print_complexity_text(r: &ComplexityReport) {
    println!("{}\n", bold("Configuration Complexity Analysis"));
    println!("  Resources:      {}", r.resource_count);
    println!("  Machines:       {}", r.machine_count);
    println!("  DAG depth:      {}", r.dependency_depth);
    println!("  Cross-machine:  {}", r.cross_machine_count);
    println!("  Templates:      {}", r.template_count);
    println!("  Conditionals:   {}", r.conditional_count);
    println!("  Includes:       {}", r.include_depth);

    let grade_colored = match r.grade {
        "A" => green(r.grade),
        "B" => green(r.grade),
        "C" => yellow(r.grade),
        _ => red(r.grade),
    };
    println!("\n  Score: {}/100  Grade: {}", r.total_score, grade_colored);

    if !r.recommendations.is_empty() {
        println!("\n  {}", bold("Recommendations:"));
        for rec in &r.recommendations {
            println!("    - {rec}");
        }
    }
}
