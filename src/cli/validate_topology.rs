//! Phase 102 — Resource Intelligence & Topology Insight: validate commands (FJ-1078, FJ-1081, FJ-1084).

#![allow(dead_code)]

use crate::core::types;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

// ============================================================================
// FJ-1078: Circular dependency depth detection
// ============================================================================

/// A warning describing one circular dependency cycle.
struct CircularWarning {
    cycle: Vec<String>,
}

/// Detect circular dependencies using DFS on the depends_on adjacency graph.
///
/// For each resource, performs iterative DFS tracking the current path. When a
/// back-edge is found (neighbour already on the current path), the cycle is
/// extracted and recorded. Each unique cycle (by sorted representation) is
/// reported at most once.
fn find_circular_dependencies(config: &types::ForjarConfig) -> Vec<CircularWarning> {
    // Build adjacency: resource -> list of dependencies (edges point to depends_on targets).
    let adjacency: HashMap<&str, &Vec<String>> = config
        .resources
        .iter()
        .map(|(name, res)| (name.as_str(), &res.depends_on))
        .collect();

    let mut warnings: Vec<CircularWarning> = Vec::new();
    let mut seen_cycles: HashSet<Vec<String>> = HashSet::new();

    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();

    for start in &names {
        // Iterative DFS with explicit path tracking.
        let mut stack: Vec<(&str, usize)> = vec![(start.as_str(), 0)];
        let mut path: Vec<&str> = vec![start.as_str()];
        let mut on_path: HashSet<&str> = HashSet::new();
        on_path.insert(start.as_str());

        while let Some((node, idx)) = stack.last_mut() {
            let deps = adjacency.get(node).map(|v| v.as_slice()).unwrap_or(&[]);
            if *idx >= deps.len() {
                on_path.remove(*node);
                path.pop();
                stack.pop();
                continue;
            }
            let dep = deps[*idx].as_str();
            *idx += 1;

            if on_path.contains(dep) {
                // Found a cycle — extract it.
                let pos = path.iter().position(|&n| n == dep).unwrap_or(0);
                let cycle: Vec<String> = path[pos..].iter().map(|s| (*s).to_string()).collect();
                let mut key = cycle.clone();
                key.sort();
                if seen_cycles.insert(key) {
                    warnings.push(CircularWarning { cycle });
                }
            } else if config.resources.contains_key(dep) {
                stack.push((dep, 0));
                path.push(dep);
                on_path.insert(dep);
            }
        }
    }
    warnings
}

/// FJ-1078: Check for circular dependencies using DFS, warn if any cycle found.
///
/// Parses the config and builds an adjacency graph from `depends_on` edges.
/// Runs DFS-based cycle detection across all resources. Each unique cycle is
/// reported as a warning containing the list of resources forming the loop.
pub(crate) fn cmd_validate_check_resource_circular_dependency_depth(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let warnings = find_circular_dependencies(&cfg);

    if json {
        let items: Vec<serde_json::Value> = warnings
            .iter()
            .map(|w| serde_json::json!({ "cycle": w.cycle }))
            .collect();
        println!(
            "{}",
            serde_json::json!({ "circular_warnings": items, "count": warnings.len() })
        );
    } else if warnings.is_empty() {
        println!("No circular dependency warnings found.");
    } else {
        println!("Circular dependency warnings ({}):", warnings.len());
        for w in &warnings {
            println!("  warning: cycle detected: {}", w.cycle.join(" -> "));
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1081: Orphan resource detection (deep)
// ============================================================================

/// Detect resources that are not reachable from any root.
///
/// A root is defined as a resource with an empty `depends_on` list.
/// Starting from all roots, BFS follows reverse edges (from dependency to
/// dependent) to discover all reachable resources. Any resource not visited
/// is considered an orphan.
fn find_orphan_resources(config: &types::ForjarConfig) -> Vec<String> {
    if config.resources.is_empty() {
        return Vec::new();
    }

    // Build reverse adjacency: dependency -> list of resources that depend on it.
    let mut reverse_adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            reverse_adj
                .entry(dep.as_str())
                .or_default()
                .push(name.as_str());
        }
    }

    // Find roots: resources with no depends_on.
    let roots: Vec<&str> = config
        .resources
        .iter()
        .filter(|(_, res)| res.depends_on.is_empty())
        .map(|(name, _)| name.as_str())
        .collect();

    // BFS from all roots following reverse edges.
    let mut visited: HashSet<&str> = HashSet::new();
    let mut queue: VecDeque<&str> = VecDeque::new();
    for root in &roots {
        visited.insert(root);
        queue.push_back(root);
    }
    while let Some(node) = queue.pop_front() {
        if let Some(dependents) = reverse_adj.get(node) {
            for &dep in dependents {
                if visited.insert(dep) {
                    queue.push_back(dep);
                }
            }
        }
    }

    // Collect unreachable resources (sorted).
    let mut orphans: Vec<String> = config
        .resources
        .keys()
        .filter(|name| !visited.contains(name.as_str()))
        .cloned()
        .collect();
    orphans.sort();
    orphans
}

/// FJ-1081: Detect resources not reachable from any root.
///
/// Parses the config and identifies root resources (those with empty
/// `depends_on`). Performs BFS from all roots through reverse dependency
/// edges to find all reachable resources. Any resource not reached is
/// reported as an orphan.
pub(crate) fn cmd_validate_check_resource_orphan_detection_deep(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let orphans = find_orphan_resources(&cfg);

    if json {
        println!(
            "{}",
            serde_json::json!({ "orphan_resources": orphans, "count": orphans.len() })
        );
    } else if orphans.is_empty() {
        println!("No orphan resources detected.");
    } else {
        println!("Orphan resources ({}):", orphans.len());
        for name in &orphans {
            println!("  orphan: '{}' is not reachable from any root", name);
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1084: Resource provider diversity
// ============================================================================

/// Result of provider diversity analysis.
struct ProviderDiversityResult {
    types_found: usize,
    warning: bool,
}

/// Analyse the diversity of resource types in the config.
///
/// Counts the number of distinct `ResourceType` values across all resources.
/// A warning is raised when there are more than one resource but all share
/// a single type, indicating lack of provider diversity.
fn check_provider_diversity(config: &types::ForjarConfig) -> ProviderDiversityResult {
    let distinct: HashSet<String> = config
        .resources
        .values()
        .map(|r| r.resource_type.to_string())
        .collect();
    let types_found = distinct.len();
    let warning = config.resources.len() > 1 && types_found == 1;
    ProviderDiversityResult {
        types_found,
        warning,
    }
}

/// FJ-1084: Warn if all resources use a single type (no provider diversity).
///
/// Parses the config and counts distinct resource types. When more than one
/// resource exists but only a single type is used, a warning is emitted to
/// encourage diversifying the resource topology.
pub(crate) fn cmd_validate_check_resource_provider_diversity(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let result = check_provider_diversity(&cfg);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "provider_diversity": {
                    "types_found": result.types_found,
                    "warning": result.warning
                }
            })
        );
    } else if result.warning {
        println!(
            "Provider diversity warning: all resources use a single type ({} type found across {} resources).",
            result.types_found,
            cfg.resources.len()
        );
    } else {
        println!(
            "Provider diversity OK: {} distinct type(s) across {} resource(s).",
            result.types_found,
            cfg.resources.len()
        );
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Deserialize a minimal resource from YAML, setting only the `type` field.
    fn make_resource(rtype: &str) -> types::Resource {
        let yaml = format!("type: {}", rtype);
        serde_yaml_ng::from_str(&yaml).unwrap()
    }

    /// Build a ForjarConfig from a list of `(name, resource)` pairs.
    fn make_config(resources: Vec<(&str, types::Resource)>) -> types::ForjarConfig {
        let mut map = indexmap::IndexMap::new();
        for (name, res) in resources {
            map.insert(name.to_string(), res);
        }
        let yaml = "version: '1.0'\nname: test\nresources: {}";
        let mut cfg: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        cfg.resources = map;
        cfg
    }

    // -- FJ-1078: Circular dependency tests --

    #[test]
    fn test_circular_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_circular_dependencies(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_circular_no_cycles() {
        let a = make_resource("package");
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_circular_dependencies(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_circular_self_loop() {
        let mut a = make_resource("file");
        a.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a)]);
        let warnings = find_circular_dependencies(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].cycle, vec!["a"]);
    }

    #[test]
    fn test_circular_two_node_cycle() {
        let mut a = make_resource("package");
        a.depends_on = vec!["b".to_string()];
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_circular_dependencies(&config);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].cycle.contains(&"a".to_string()));
        assert!(warnings[0].cycle.contains(&"b".to_string()));
    }

    #[test]
    fn test_circular_three_node_cycle() {
        let mut a = make_resource("package");
        a.depends_on = vec!["c".to_string()];
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let mut c = make_resource("file");
        c.depends_on = vec!["b".to_string()];
        let config = make_config(vec![("a", a), ("b", b), ("c", c)]);
        let warnings = find_circular_dependencies(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].cycle.len(), 3);
    }

    #[test]
    fn test_circular_with_acyclic_branch() {
        let mut a = make_resource("package");
        a.depends_on = vec!["b".to_string()];
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let mut c = make_resource("file");
        c.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b), ("c", c)]);
        let warnings = find_circular_dependencies(&config);
        // Only the a<->b cycle should be reported
        assert_eq!(warnings.len(), 1);
    }

    // -- FJ-1081: Orphan detection tests --

    #[test]
    fn test_orphan_empty_config() {
        let config = make_config(vec![]);
        let orphans = find_orphan_resources(&config);
        assert!(orphans.is_empty());
    }

    #[test]
    fn test_orphan_all_roots() {
        let a = make_resource("package");
        let b = make_resource("service");
        let config = make_config(vec![("a", a), ("b", b)]);
        let orphans = find_orphan_resources(&config);
        assert!(orphans.is_empty());
    }

    #[test]
    fn test_orphan_linear_chain_no_orphans() {
        let a = make_resource("package");
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let mut c = make_resource("file");
        c.depends_on = vec!["b".to_string()];
        let config = make_config(vec![("a", a), ("b", b), ("c", c)]);
        let orphans = find_orphan_resources(&config);
        assert!(orphans.is_empty());
    }

    #[test]
    fn test_orphan_mutual_dependency_orphans() {
        // Both depend on each other, neither is a root.
        let mut a = make_resource("package");
        a.depends_on = vec!["b".to_string()];
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let orphans = find_orphan_resources(&config);
        assert_eq!(orphans.len(), 2);
        assert_eq!(orphans, vec!["a", "b"]);
    }

    #[test]
    fn test_orphan_partial_reachability() {
        // root -> b, but c depends on unknown "x" (no root path).
        let root = make_resource("package");
        let mut b = make_resource("service");
        b.depends_on = vec!["root".to_string()];
        let mut c = make_resource("file");
        c.depends_on = vec!["x".to_string()];
        let config = make_config(vec![("b", b), ("c", c), ("root", root)]);
        let orphans = find_orphan_resources(&config);
        assert_eq!(orphans, vec!["c"]);
    }

    #[test]
    fn test_orphan_single_resource_no_deps_is_root() {
        let a = make_resource("package");
        let config = make_config(vec![("a", a)]);
        let orphans = find_orphan_resources(&config);
        assert!(orphans.is_empty());
    }

    // -- FJ-1084: Provider diversity tests --

    #[test]
    fn test_diversity_empty_config() {
        let config = make_config(vec![]);
        let result = check_provider_diversity(&config);
        assert_eq!(result.types_found, 0);
        assert!(!result.warning);
    }

    #[test]
    fn test_diversity_single_resource_no_warning() {
        let a = make_resource("package");
        let config = make_config(vec![("a", a)]);
        let result = check_provider_diversity(&config);
        assert_eq!(result.types_found, 1);
        assert!(!result.warning);
    }

    #[test]
    fn test_diversity_multiple_same_type_warns() {
        let a = make_resource("package");
        let b = make_resource("package");
        let config = make_config(vec![("a", a), ("b", b)]);
        let result = check_provider_diversity(&config);
        assert_eq!(result.types_found, 1);
        assert!(result.warning);
    }

    #[test]
    fn test_diversity_multiple_types_no_warning() {
        let a = make_resource("package");
        let b = make_resource("service");
        let c = make_resource("file");
        let config = make_config(vec![("a", a), ("b", b), ("c", c)]);
        let result = check_provider_diversity(&config);
        assert_eq!(result.types_found, 3);
        assert!(!result.warning);
    }

    #[test]
    fn test_diversity_two_types_no_warning() {
        let a = make_resource("package");
        let b = make_resource("service");
        let c = make_resource("package");
        let config = make_config(vec![("a", a), ("b", b), ("c", c)]);
        let result = check_provider_diversity(&config);
        assert_eq!(result.types_found, 2);
        assert!(!result.warning);
    }
}
