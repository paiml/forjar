//! Resilience validation — lifecycle hook coverage, secret rotation, dependency depth limits.

#![allow(dead_code)]

use crate::core::types;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

// ============================================================================
// Side-effect resource types that benefit from lifecycle hooks
// ============================================================================

const SIDE_EFFECT_TYPES: &[types::ResourceType] = &[
    types::ResourceType::Service,
    types::ResourceType::Package,
    types::ResourceType::Mount,
    types::ResourceType::Docker,
    types::ResourceType::Network,
];

/// Maximum allowed dependency chain depth.
const DEPTH_LIMIT: usize = 10;

// ============================================================================
// FJ-1022: Lifecycle hook coverage for side-effect resources
// ============================================================================

/// Warn if resources with side effects (service, package, mount, docker, network)
/// lack pre/post lifecycle hooks. These resource types modify system state and
/// benefit from hooks for safety.
pub(crate) fn cmd_validate_check_resource_lifecycle_hook_coverage(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_lifecycle_hook_coverage_gaps(&config);

    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(name, rtype, has_pre, has_post)| {
                format!(
                    "{{\"resource\":\"{}\",\"type\":\"{}\",\"has_pre_hook\":{},\"has_post_hook\":{}}}",
                    name, rtype, has_pre, has_post
                )
            })
            .collect();
        println!("{{\"lifecycle_hook_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All side-effect resources have lifecycle hooks.");
    } else {
        for (name, rtype, _, _) in &warnings {
            println!("warning: {} ({}) has no lifecycle hooks", name, rtype);
        }
    }
    Ok(())
}

/// Returns `(resource_name, resource_type, has_pre_hook, has_post_hook)` for
/// each side-effect resource missing at least one hook.
fn find_lifecycle_hook_coverage_gaps(
    config: &types::ForjarConfig,
) -> Vec<(String, String, bool, bool)> {
    let mut warnings = Vec::new();
    for (name, resource) in &config.resources {
        if !is_side_effect_type(&resource.resource_type) {
            continue;
        }
        let has_pre = resource.pre_apply.is_some();
        let has_post = resource.post_apply.is_some();
        if !has_pre && !has_post {
            warnings.push((
                name.clone(),
                resource.resource_type.to_string(),
                has_pre,
                has_post,
            ));
        }
    }
    warnings.sort_by(|a, b| a.0.cmp(&b.0));
    warnings
}

fn is_side_effect_type(rtype: &types::ResourceType) -> bool {
    SIDE_EFFECT_TYPES.contains(rtype)
}

// ============================================================================
// FJ-1025: Encrypted secret rotation review
// ============================================================================

/// Warn if any resource content contains `ENC[age,...]` markers (encrypted
/// secrets using age encryption). This is a static check — we cannot know the
/// actual rotation age, so we flag resources that use encrypted secrets as
/// needing rotation review.
pub(crate) fn cmd_validate_check_resource_secret_rotation_age(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_secret_rotation_warnings(&config);

    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|name| {
                format!(
                    "{{\"resource\":\"{}\",\"has_encrypted_content\":true}}",
                    name
                )
            })
            .collect();
        println!("{{\"secret_rotation_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("No encrypted secrets found in resources.");
    } else {
        for name in &warnings {
            println!(
                "review: {} contains encrypted secret (rotation recommended)",
                name
            );
        }
    }
    Ok(())
}

/// Returns resource names whose `content` field contains `ENC[age,` markers.
fn find_secret_rotation_warnings(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings: Vec<String> = config
        .resources
        .iter()
        .filter_map(|(name, resource)| {
            resource
                .content
                .as_ref()
                .filter(|c| c.contains("ENC[age,"))
                .map(|_| name.clone())
        })
        .collect();
    warnings.sort();
    warnings
}

// ============================================================================
// FJ-1028: Dependency depth limit
// ============================================================================

/// Warn if any dependency chain exceeds the maximum depth limit (10).
/// Builds a dependency graph from `depends_on` fields and computes the
/// maximum depth for each resource via BFS from roots.
pub(crate) fn cmd_validate_check_resource_dependency_chain_depth(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let violations = find_depth_limit_violations(&config);

    if json {
        let items: Vec<String> = violations
            .iter()
            .map(|(name, depth)| {
                format!(
                    "{{\"resource\":\"{}\",\"depth\":{},\"limit\":{}}}",
                    name, depth, DEPTH_LIMIT
                )
            })
            .collect();
        println!("{{\"depth_limit_warnings\":[{}]}}", items.join(","));
    } else if violations.is_empty() {
        println!(
            "All dependency chains within depth limit ({}).",
            DEPTH_LIMIT
        );
    } else {
        for (name, depth) in &violations {
            println!(
                "warning: {} has dependency depth {} (limit: {})",
                name, depth, DEPTH_LIMIT
            );
        }
    }
    Ok(())
}

/// Returns `(resource_name, depth)` for each resource whose longest
/// dependency chain exceeds `DEPTH_LIMIT`.
fn find_depth_limit_violations(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let depths = compute_all_depths(config);
    let mut violations: Vec<(String, usize)> = depths
        .into_iter()
        .filter(|(_, depth)| *depth > DEPTH_LIMIT)
        .collect();
    violations.sort_by(|a, b| a.0.cmp(&b.0));
    violations
}

/// Compute the maximum dependency depth for every resource using BFS.
///
/// Depth of a root resource (no dependencies) is 0.
/// Depth of a resource is 1 + max(depth of each dependency).
///
/// Uses Kahn's algorithm (topological BFS) to handle the DAG efficiently
/// and safely handle cycles (cyclic resources get depth 0 — cycles are
/// reported by other validators).
fn compute_all_depths(config: &types::ForjarConfig) -> HashMap<String, usize> {
    let names: HashSet<&String> = config.resources.keys().collect();

    // Build adjacency: for each resource, its valid dependencies.
    // Also track in-degree for topological sort.
    let mut in_degree: HashMap<&String, usize> = HashMap::new();
    let mut dependents: HashMap<&String, Vec<&String>> = HashMap::new();

    for name in &names {
        in_degree.insert(name, 0);
    }

    for (name, resource) in &config.resources {
        let valid_deps: Vec<&String> = resource
            .depends_on
            .iter()
            .filter(|d| names.contains(d))
            .collect();
        *in_degree.entry(name).or_insert(0) += valid_deps.len();
        for dep in valid_deps {
            dependents.entry(dep).or_default().push(name);
        }
    }

    // Kahn's BFS: start from roots (in_degree == 0).
    let mut depths: HashMap<String, usize> = HashMap::new();
    let mut queue: VecDeque<&String> = VecDeque::new();

    for (name, &deg) in &in_degree {
        if deg == 0 {
            queue.push_back(name);
            depths.insert((*name).clone(), 0);
        }
    }

    while let Some(current) = queue.pop_front() {
        let current_depth = depths[current.as_str()];
        if let Some(deps) = dependents.get(current) {
            for dependent in deps {
                let new_depth = current_depth + 1;
                let entry = depths.entry((*dependent).clone()).or_insert(0);
                if new_depth > *entry {
                    *entry = new_depth;
                }
                let deg = in_degree.get_mut(dependent).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(dependent);
                }
            }
        }
    }

    depths
}
