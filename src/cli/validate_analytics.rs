//! Phase 97 — State Analytics & Capacity Planning: validate commands.

#![allow(dead_code)]

use crate::core::types;
use std::collections::{HashMap, HashSet};
use std::path::Path;

// ============================================================================
// FJ-1038: Resource health correlation
// ============================================================================

/// Identify dependency "hubs" — resources depended on by 3+ others — and warn
/// that failures in shared dependencies will cause correlated failures across
/// all resources that share them.
pub(crate) fn cmd_validate_check_resource_health_correlation(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let hubs = find_dependency_hubs(&config);
    let correlated = find_correlated_groups(&config, &hubs);

    if json {
        let hub_items: Vec<String> = hubs
            .iter()
            .map(|(dep, dependents)| {
                let arr: Vec<String> = dependents.iter().map(|d| format!("\"{}\"", d)).collect();
                format!(
                    "{{\"hub\":\"{}\",\"dependents\":[{}],\"fan_in\":{}}}",
                    dep,
                    arr.join(","),
                    dependents.len()
                )
            })
            .collect();
        let corr_items: Vec<String> = correlated
            .iter()
            .map(|(dep, resources)| {
                let arr: Vec<String> = resources.iter().map(|r| format!("\"{}\"", r)).collect();
                format!(
                    "{{\"shared_dependency\":\"{}\",\"correlated_resources\":[{}]}}",
                    dep,
                    arr.join(",")
                )
            })
            .collect();
        println!(
            "{{\"dependency_hubs\":[{}],\"correlated_failure_groups\":[{}]}}",
            hub_items.join(","),
            corr_items.join(",")
        );
    } else if hubs.is_empty() && correlated.is_empty() {
        println!("No dependency hubs or correlated failure groups detected.");
    } else {
        print_hub_warnings(&hubs);
        print_correlation_warnings(&correlated);
    }
    Ok(())
}

/// Build a map of dependency -> list of resources that depend on it, filtered
/// to only those with a fan-in of 3 or more (dependency "hubs").
fn find_dependency_hubs(config: &types::ForjarConfig) -> Vec<(String, Vec<String>)> {
    let mut dep_to_dependents: HashMap<String, Vec<String>> = HashMap::new();

    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            dep_to_dependents
                .entry(dep.clone())
                .or_default()
                .push(name.clone());
        }
    }

    let mut hubs: Vec<(String, Vec<String>)> = dep_to_dependents
        .into_iter()
        .filter(|(_, dependents)| dependents.len() >= 3)
        .collect();

    for (_, dependents) in &mut hubs {
        dependents.sort();
    }
    hubs.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    hubs
}

/// Find groups of resources that share the same depends_on targets, meaning a
/// failure in that shared dependency would cause correlated failures.
fn find_correlated_groups(
    config: &types::ForjarConfig,
    hubs: &[(String, Vec<String>)],
) -> Vec<(String, Vec<String>)> {
    let hub_set: HashSet<&String> = hubs.iter().map(|(h, _)| h).collect();
    let mut dep_to_resources: HashMap<&String, Vec<String>> = HashMap::new();

    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if hub_set.contains(dep) {
                dep_to_resources.entry(dep).or_default().push(name.clone());
            }
        }
    }

    let mut groups: Vec<(String, Vec<String>)> = dep_to_resources
        .into_iter()
        .filter(|(_, resources)| resources.len() >= 2)
        .map(|(dep, mut resources)| {
            resources.sort();
            resources.dedup();
            (dep.clone(), resources)
        })
        .collect();

    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
    groups
}

fn print_hub_warnings(hubs: &[(String, Vec<String>)]) {
    for (dep, dependents) in hubs {
        println!(
            "warning: '{}' is a dependency hub (fan-in {}) — depended on by: {}",
            dep,
            dependents.len(),
            dependents.join(", ")
        );
    }
}

fn print_correlation_warnings(correlated: &[(String, Vec<String>)]) {
    for (dep, resources) in correlated {
        println!(
            "warning: failure in '{}' would cause correlated failures in: {}",
            dep,
            resources.join(", ")
        );
    }
}

// ============================================================================
// FJ-1041: Dependency optimization
// ============================================================================

/// Find redundant (transitive) edges in the dependency graph. If A depends on
/// B and C, and B depends on C, then A -> C is redundant because it is already
/// implied through B -> C. Reports these as optimization suggestions.
pub(crate) fn cmd_validate_check_dependency_optimization(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let redundant = find_redundant_edges(&config);

    if json {
        let items: Vec<String> = redundant
            .iter()
            .map(|(resource, edge, via)| {
                format!(
                    "{{\"resource\":\"{}\",\"redundant_dep\":\"{}\",\"implied_via\":\"{}\"}}",
                    resource, edge, via
                )
            })
            .collect();
        println!("{{\"redundant_edges\":[{}]}}", items.join(","));
    } else if redundant.is_empty() {
        println!("No redundant dependency edges detected.");
    } else {
        for (resource, edge, via) in &redundant {
            println!(
                "suggestion: {}'s dependency on '{}' is redundant — already implied through '{}'",
                resource, edge, via
            );
        }
    }
    Ok(())
}

/// Compute the transitive closure of dependencies for a single resource,
/// excluding its own direct edges. Returns the set of all resources reachable
/// through at least one intermediate step.
fn transitive_deps_via(
    resource: &str,
    direct_deps: &[String],
    dep_map: &HashMap<&str, &[String]>,
) -> HashMap<String, String> {
    // For each direct dep D of `resource`, BFS/DFS from D to find everything
    // D can reach. Those are "implied via D".
    let mut implied: HashMap<String, String> = HashMap::new();
    for direct in direct_deps {
        let reachable = reachable_from(direct, dep_map);
        for r in reachable {
            if r != resource && !implied.contains_key(&r) {
                implied.insert(r, direct.clone());
            }
        }
    }
    implied
}

/// BFS from `start` collecting all transitively reachable resources.
fn reachable_from(start: &str, dep_map: &HashMap<&str, &[String]>) -> HashSet<String> {
    let mut visited = HashSet::new();
    let mut queue = std::collections::VecDeque::new();

    if let Some(deps) = dep_map.get(start) {
        for d in *deps {
            queue.push_back(d.as_str());
        }
    }

    while let Some(current) = queue.pop_front() {
        if !visited.insert(current.to_string()) {
            continue;
        }
        if let Some(deps) = dep_map.get(current) {
            for d in *deps {
                if !visited.contains(d.as_str()) {
                    queue.push_back(d.as_str());
                }
            }
        }
    }
    visited
}

/// Returns `(resource_name, redundant_edge, implied_via)` for each transitive
/// dependency edge that can be safely removed.
fn find_redundant_edges(config: &types::ForjarConfig) -> Vec<(String, String, String)> {
    // Build dep_map: resource_name -> &[depends_on]
    let dep_map: HashMap<&str, &[String]> = config
        .resources
        .iter()
        .map(|(name, res)| (name.as_str(), res.depends_on.as_slice()))
        .collect();

    let mut redundant = Vec::new();

    for (name, resource) in &config.resources {
        if resource.depends_on.len() < 2 {
            continue;
        }
        let implied = transitive_deps_via(name, &resource.depends_on, &dep_map);
        for direct in &resource.depends_on {
            if let Some(via) = implied.get(direct.as_str()) {
                redundant.push((name.clone(), direct.clone(), via.clone()));
            }
        }
    }

    redundant.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    redundant
}

// ============================================================================
// FJ-1044: Resource consolidation opportunities
// ============================================================================

/// Find resources of the same type on different machines with similar names
/// (edit distance <= 2) or identical content. Suggest these as consolidation
/// candidates — they may be duplicated work that could use `machine: [a, b]`.
pub(crate) fn cmd_validate_check_resource_consolidation_opportunities(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let opportunities = find_consolidation_opportunities(&config);

    if json {
        let items: Vec<String> = opportunities
            .iter()
            .map(|opp| {
                format!(
                    "{{\"resource_a\":\"{}\",\"resource_b\":\"{}\",\"reason\":\"{}\",\"resource_type\":\"{}\"}}",
                    opp.name_a, opp.name_b, opp.reason, opp.resource_type
                )
            })
            .collect();
        println!("{{\"consolidation_opportunities\":[{}]}}", items.join(","));
    } else if opportunities.is_empty() {
        println!("No resource consolidation opportunities detected.");
    } else {
        for opp in &opportunities {
            println!(
                "suggestion: {} and {} ({}) could be consolidated — {}",
                opp.name_a, opp.name_b, opp.resource_type, opp.reason
            );
        }
    }
    Ok(())
}

/// A consolidation opportunity between two resources.
struct ConsolidationOpportunity {
    name_a: String,
    name_b: String,
    resource_type: String,
    reason: String,
}

/// Find consolidation opportunities across resources of the same type on
/// different machines.
fn find_consolidation_opportunities(config: &types::ForjarConfig) -> Vec<ConsolidationOpportunity> {
    let entries: Vec<(&String, &types::Resource)> = config.resources.iter().collect();
    let mut opportunities = Vec::new();
    let mut seen = HashSet::new();

    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let (name_a, res_a) = entries[i];
            let (name_b, res_b) = entries[j];

            if res_a.resource_type != res_b.resource_type {
                continue;
            }
            if !on_different_machines(res_a, res_b) {
                continue;
            }

            let pair_key = format!("{}:{}", name_a, name_b);
            if seen.contains(&pair_key) {
                continue;
            }

            if let Some(reason) = check_consolidation_reason(name_a, res_a, name_b, res_b) {
                seen.insert(pair_key);
                opportunities.push(ConsolidationOpportunity {
                    name_a: name_a.clone(),
                    name_b: name_b.clone(),
                    resource_type: res_a.resource_type.to_string(),
                    reason,
                });
            }
        }
    }

    opportunities.sort_by(|a, b| {
        a.name_a
            .cmp(&b.name_a)
            .then_with(|| a.name_b.cmp(&b.name_b))
    });
    opportunities
}

/// Check whether two resources target different machines.
fn on_different_machines(a: &types::Resource, b: &types::Resource) -> bool {
    let machines_a: HashSet<String> = a.machine.to_vec().into_iter().collect();
    let machines_b: HashSet<String> = b.machine.to_vec().into_iter().collect();
    machines_a.is_disjoint(&machines_b)
}

/// Return a consolidation reason if two same-type resources on different
/// machines have similar names (edit distance <= 2) or identical content.
fn check_consolidation_reason(
    name_a: &str,
    res_a: &types::Resource,
    name_b: &str,
    res_b: &types::Resource,
) -> Option<String> {
    // Check identical content first (stronger signal).
    if has_identical_content(res_a, res_b) {
        return Some("identical content on different machines".to_string());
    }

    // Check similar names (edit distance <= 2).
    let distance = levenshtein(name_a, name_b);
    if distance <= 2 && distance > 0 {
        return Some(format!(
            "similar names (edit distance {}), may be duplicates",
            distance
        ));
    }

    None
}

/// Check whether two resources have identical non-empty content.
fn has_identical_content(a: &types::Resource, b: &types::Resource) -> bool {
    match (&a.content, &b.content) {
        (Some(ca), Some(cb)) if !ca.is_empty() && !cb.is_empty() => ca == cb,
        _ => false,
    }
}

/// Compute the Levenshtein edit distance between two strings.
///
/// Uses a single-row dynamic programming approach (O(min(m,n)) space).
pub(crate) fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    // Ensure a is the shorter string for space efficiency.
    if a_chars.len() > b_chars.len() {
        return levenshtein_chars(&b_chars, &a_chars);
    }
    levenshtein_chars(&a_chars, &b_chars)
}

/// Inner Levenshtein computation over char slices where `a.len() <= b.len()`.
fn levenshtein_chars(a: &[char], b: &[char]) -> usize {
    let n = a.len();
    let m = b.len();

    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];

    for j in 1..=m {
        curr[0] = j;
        for i in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[i] = (prev[i] + 1).min(curr[i - 1] + 1).min(prev[i - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}
