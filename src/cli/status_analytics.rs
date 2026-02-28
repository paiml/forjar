//! Phase 97 — State Analytics & Capacity Planning: status commands.

use std::collections::BTreeMap;
use std::path::Path;

/// Parse a state lock file and extract per-machine metrics used by multiple commands.
struct LockMetrics {
    resource_count: usize,
    has_generated_at: bool,
    has_blake3_version: bool,
    has_drifted: bool,
    resource_types: Vec<String>,
}

fn parse_lock_content(content: &str) -> LockMetrics {
    let resource_count = content
        .lines()
        .filter(|l| l.starts_with("  ") && l.contains("type:"))
        .count();
    let has_generated_at = content.lines().any(|l| l.starts_with("generated_at:"));
    let has_blake3_version = content.lines().any(|l| l.starts_with("blake3_version:"));
    let has_drifted = content.contains("drifted: true");
    let resource_types: Vec<String> = content
        .lines()
        .filter(|l| l.starts_with("  ") && l.contains("type:"))
        .filter_map(|l| {
            l.split("type:")
                .nth(1)
                .map(|t| t.trim().trim_matches('"').to_string())
        })
        .collect();
    LockMetrics {
        resource_count,
        has_generated_at,
        has_blake3_version,
        has_drifted,
        resource_types,
    }
}

/// Iterate state_dir entries, applying optional machine filter, yielding (name, content) pairs.
fn iter_lock_files(
    state_dir: &Path,
    machine: Option<&str>,
) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let entries = std::fs::read_dir(state_dir)
        .unwrap_or_else(|_| std::fs::read_dir("/dev/null").unwrap());
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine {
            if name != filter {
                continue;
            }
        }
        let lock_path = entry.path().join("state.lock.yaml");
        if !lock_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&lock_path).unwrap_or_default();
        results.push((name, content));
    }
    results
}

/// FJ-1037: `status --fleet-state-churn-analysis`
///
/// For each machine, count resources and report churn metrics
/// (resource count, whether any are drifted).
pub(crate) fn cmd_status_fleet_state_churn_analysis(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let mut results: BTreeMap<String, serde_json::Value> = BTreeMap::new();

    for (name, content) in iter_lock_files(state_dir, machine) {
        let metrics = parse_lock_content(&content);
        results.insert(
            name,
            serde_json::json!({
                "resource_count": metrics.resource_count,
                "has_drifted": metrics.has_drifted,
                "churn_indicator": if metrics.has_drifted { "unstable" } else { "stable" },
            }),
        );
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "fleet_state_churn_analysis": results
            }))
            .unwrap()
        );
    } else {
        print_churn_table(&results);
    }
    Ok(())
}

fn print_churn_table(results: &BTreeMap<String, serde_json::Value>) {
    println!("=== Fleet State Churn Analysis ===");
    if results.is_empty() {
        println!("  No machine state found.");
        return;
    }
    for (m, info) in results {
        let count = info["resource_count"].as_u64().unwrap_or(0);
        let drifted = info["has_drifted"].as_bool().unwrap_or(false);
        let indicator = info["churn_indicator"].as_str().unwrap_or("unknown");
        let symbol = if drifted { "!" } else { "~" };
        println!(
            "  {} {}: resources={}, churn={}",
            symbol, m, count, indicator
        );
    }
}

/// FJ-1040: `status --config-maturity-score`
///
/// Compute a maturity score (0-100) based on:
///   - Resource count (more = higher, up to 20 points)
///   - Presence of generated_at timestamp (20 points)
///   - Presence of blake3_version (20 points)
///   - No drifted resources (20 points)
///   - Multiple resource types (20 points)
pub(crate) fn cmd_status_config_maturity_score(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let mut results: BTreeMap<String, serde_json::Value> = BTreeMap::new();

    for (name, content) in iter_lock_files(state_dir, machine) {
        let metrics = parse_lock_content(&content);
        let score = compute_maturity_score(&metrics);
        results.insert(
            name,
            serde_json::json!({
                "score": score,
                "resource_count": metrics.resource_count,
                "has_generated_at": metrics.has_generated_at,
                "has_blake3_version": metrics.has_blake3_version,
                "no_drift": !metrics.has_drifted,
                "multiple_types": unique_type_count(&metrics.resource_types) > 1,
            }),
        );
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "config_maturity_score": results
            }))
            .unwrap()
        );
    } else {
        print_maturity_table(&results);
    }
    Ok(())
}

fn compute_maturity_score(metrics: &LockMetrics) -> u64 {
    let mut score: u64 = 0;
    // Resource count: up to 20 points (1 point per resource, capped at 20)
    score += std::cmp::min(metrics.resource_count as u64, 20);
    // Presence of generated_at: 20 points
    if metrics.has_generated_at {
        score += 20;
    }
    // Presence of blake3_version: 20 points
    if metrics.has_blake3_version {
        score += 20;
    }
    // No drifted resources: 20 points
    if !metrics.has_drifted {
        score += 20;
    }
    // Multiple resource types: 20 points
    if unique_type_count(&metrics.resource_types) > 1 {
        score += 20;
    }
    score
}

fn unique_type_count(types: &[String]) -> usize {
    let mut seen = std::collections::BTreeSet::new();
    for t in types {
        seen.insert(t.as_str());
    }
    seen.len()
}

fn print_maturity_table(results: &BTreeMap<String, serde_json::Value>) {
    println!("=== Config Maturity Score ===");
    if results.is_empty() {
        println!("  No machine state found.");
        return;
    }
    for (m, info) in results {
        let score = info["score"].as_u64().unwrap_or(0);
        let grade = match score {
            80..=100 => "A",
            60..=79 => "B",
            40..=59 => "C",
            20..=39 => "D",
            _ => "F",
        };
        println!("  {}: score={}/100 grade={}", m, score, grade);
    }
}

/// FJ-1043: `status --fleet-capacity-utilization`
///
/// Count total machines, total resources, average resources per machine.
/// Report utilization as a simple ratio.
pub(crate) fn cmd_status_fleet_capacity_utilization(
    state_dir: &Path,
    machine: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let mut total_machines: u64 = 0;
    let mut total_resources: u64 = 0;

    for (_name, content) in iter_lock_files(state_dir, machine) {
        let metrics = parse_lock_content(&content);
        total_machines += 1;
        total_resources += metrics.resource_count as u64;
    }

    let avg_resources = if total_machines > 0 {
        total_resources as f64 / total_machines as f64
    } else {
        0.0
    };
    // Utilization ratio: fraction of a nominal 50-resource-per-machine capacity.
    let utilization_pct = if total_machines > 0 {
        (avg_resources / 50.0 * 100.0).min(100.0)
    } else {
        0.0
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "fleet_capacity_utilization": {
                    "total_machines": total_machines,
                    "total_resources": total_resources,
                    "avg_resources_per_machine": format!("{:.1}", avg_resources),
                    "utilization_pct": format!("{:.1}", utilization_pct),
                }
            }))
            .unwrap()
        );
    } else {
        println!("=== Fleet Capacity Utilization ===");
        if total_machines == 0 {
            println!("  No machines found.");
        } else {
            println!("  Total machines:  {}", total_machines);
            println!("  Total resources: {}", total_resources);
            println!("  Avg resources/machine: {:.1}", avg_resources);
            println!("  Utilization: {:.1}%", utilization_pct);
        }
    }
    Ok(())
}
