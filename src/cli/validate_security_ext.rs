//! Phase 100 — Extended Security Validation: dependency symmetry, tag namespace, machine capacity.

#![allow(dead_code)]

use crate::core::types;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

// ============================================================================
// FJ-1062: Resource dependency symmetry (cycle detection)
// ============================================================================

/// Build an adjacency list from `depends_on` fields.
fn build_adjacency(config: &types::ForjarConfig) -> HashMap<String, Vec<String>> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            adj.entry(name.clone()).or_default().push(dep.clone());
        }
    }
    adj
}

/// Check if `start` appears in its own transitive dependency closure via BFS.
fn has_cycle_from(start: &str, adj: &HashMap<String, Vec<String>>) -> Option<String> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    // Seed with direct dependencies of start.
    if let Some(deps) = adj.get(start) {
        for dep in deps {
            if dep == start {
                return Some(format!("{}->>{}", start, start));
            }
            if visited.insert(dep.clone()) {
                queue.push_back(dep.clone());
            }
        }
    }
    while let Some(current) = queue.pop_front() {
        if let Some(deps) = adj.get(&current) {
            for dep in deps {
                if dep == start {
                    return Some(format!("bidirectional path: {}>>...>>{}>>...>>{}", start, current, start));
                }
                if visited.insert(dep.clone()) {
                    queue.push_back(dep.clone());
                }
            }
        }
    }
    None
}

/// Find all resources that appear in their own transitive dependency closure.
fn find_dependency_symmetry_warnings(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let adj = build_adjacency(config);
    let mut warnings = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        if let Some(detail) = has_cycle_from(name, &adj) {
            warnings.push((name.clone(), detail));
        }
    }
    warnings
}

/// FJ-1062: Check that no resource appears in its own transitive dependency closure.
///
/// Parses the config and builds an adjacency list from `depends_on`. For each
/// resource, performs BFS to detect if there is a transitive path back to itself
/// (i.e., a dependency cycle). Warns for each resource involved in a cycle.
pub(crate) fn cmd_validate_check_resource_dependency_symmetry_deep(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_dependency_symmetry_warnings(&config);

    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(n, d)| format!(r#"{{"resource":"{}","detail":"{}"}}"#, n, d))
            .collect();
        println!(
            r#"{{"dependency_symmetry_warnings":[{}],"count":{}}}"#,
            items.join(","),
            warnings.len()
        );
    } else if warnings.is_empty() {
        println!("No dependency symmetry issues found.");
    } else {
        println!("Dependency symmetry warnings ({}):", warnings.len());
        for (name, detail) in &warnings {
            println!("  warning: resource '{}' — {}", name, detail);
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1065: Resource tag namespace
// ============================================================================

/// A warning about unnamespaced tags on a resource.
struct TagNamespaceWarning {
    resource: String,
    unnamespaced_tags: Vec<String>,
}

/// Find resources with tags that lack a namespace prefix (no colon).
fn find_tag_namespace_warnings(config: &types::ForjarConfig) -> Vec<TagNamespaceWarning> {
    let mut warnings = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        let bad_tags: Vec<String> = resource
            .tags
            .iter()
            .filter(|t| !t.contains(':'))
            .cloned()
            .collect();
        if !bad_tags.is_empty() {
            warnings.push(TagNamespaceWarning {
                resource: name.clone(),
                unnamespaced_tags: bad_tags,
            });
        }
    }
    warnings
}

/// FJ-1065: Check that resource tags follow namespace conventions (contain a colon).
///
/// Parses the config and checks each resource's tags. Tags without a colon
/// separator (e.g., "production" vs "env:prod") are flagged as unnamespaced.
/// Namespaced tags improve filtering and organizational clarity.
pub(crate) fn cmd_validate_check_resource_tag_namespace(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_tag_namespace_warnings(&config);

    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|w| {
                let tags: Vec<String> = w
                    .unnamespaced_tags
                    .iter()
                    .map(|t| format!(r#""{}""#, t))
                    .collect();
                format!(
                    r#"{{"resource":"{}","unnamespaced_tags":[{}]}}"#,
                    w.resource,
                    tags.join(",")
                )
            })
            .collect();
        println!(
            r#"{{"tag_namespace_warnings":[{}],"count":{}}}"#,
            items.join(","),
            warnings.len()
        );
    } else if warnings.is_empty() {
        println!("All resource tags follow namespace conventions.");
    } else {
        for w in &warnings {
            println!(
                "warning: resource '{}' has unnamespaced tags: {}",
                w.resource,
                w.unnamespaced_tags.join(", ")
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1068: Resource machine capacity
// ============================================================================

/// Default threshold for resources per machine.
const DEFAULT_MACHINE_CAPACITY_THRESHOLD: usize = 20;

/// A warning about a machine exceeding the resource capacity threshold.
struct MachineCapacityWarning {
    machine: String,
    resource_count: usize,
    threshold: usize,
}

/// Count resources per machine and warn if any exceeds the threshold.
fn find_machine_capacity_warnings(
    config: &types::ForjarConfig,
    threshold: usize,
) -> Vec<MachineCapacityWarning> {
    let mut machine_counts: HashMap<String, usize> = HashMap::new();
    for resource in config.resources.values() {
        for machine in resource.machine.to_vec() {
            *machine_counts.entry(machine).or_insert(0) += 1;
        }
    }
    let mut warnings = Vec::new();
    let mut machines: Vec<String> = machine_counts.keys().cloned().collect();
    machines.sort();
    for machine in machines {
        let count = machine_counts[&machine];
        if count > threshold {
            warnings.push(MachineCapacityWarning {
                machine,
                resource_count: count,
                threshold,
            });
        }
    }
    warnings
}

/// FJ-1068: Check that no machine has more resources than the capacity threshold.
///
/// Parses the config and counts how many resources target each machine.
/// Warns if any machine has more than `DEFAULT_MACHINE_CAPACITY_THRESHOLD` (20)
/// resources, which may indicate overloading or configuration drift.
pub(crate) fn cmd_validate_check_resource_machine_capacity(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_machine_capacity_warnings(&config, DEFAULT_MACHINE_CAPACITY_THRESHOLD);

    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|w| {
                format!(
                    r#"{{"machine":"{}","resource_count":{},"threshold":{}}}"#,
                    w.machine, w.resource_count, w.threshold
                )
            })
            .collect();
        println!(
            r#"{{"machine_capacity_warnings":[{}],"count":{}}}"#,
            items.join(","),
            warnings.len()
        );
    } else if warnings.is_empty() {
        println!("All machines are within resource capacity thresholds.");
    } else {
        for w in &warnings {
            println!(
                "warning: machine '{}' has {} resources (threshold: {})",
                w.machine, w.resource_count, w.threshold
            );
        }
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

    // -- FJ-1062: Dependency symmetry tests --

    #[test]
    fn test_dependency_symmetry_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_dependency_symmetry_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_dependency_symmetry_no_cycle() {
        let a = make_resource("package");
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_dependency_symmetry_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_dependency_symmetry_direct_cycle() {
        let mut a = make_resource("package");
        a.depends_on = vec!["b".to_string()];
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_dependency_symmetry_warnings(&config);
        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0].0, "a");
        assert_eq!(warnings[1].0, "b");
    }

    #[test]
    fn test_dependency_symmetry_transitive_cycle() {
        let mut a = make_resource("file");
        a.depends_on = vec!["b".to_string()];
        let mut b = make_resource("file");
        b.depends_on = vec!["c".to_string()];
        let mut c = make_resource("file");
        c.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b), ("c", c)]);
        let warnings = find_dependency_symmetry_warnings(&config);
        assert_eq!(warnings.len(), 3);
    }

    #[test]
    fn test_dependency_symmetry_self_loop() {
        let mut a = make_resource("file");
        a.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a)]);
        let warnings = find_dependency_symmetry_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].0, "a");
    }

    // -- FJ-1065: Tag namespace tests --

    #[test]
    fn test_tag_namespace_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_tag_namespace_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_tag_namespace_all_namespaced() {
        let mut r = make_resource("file");
        r.tags = vec!["env:prod".to_string(), "tier:web".to_string()];
        let config = make_config(vec![("web-cfg", r)]);
        let warnings = find_tag_namespace_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_tag_namespace_warns_unnamespaced() {
        let mut r = make_resource("file");
        r.tags = vec!["production".to_string(), "env:staging".to_string()];
        let config = make_config(vec![("app-cfg", r)]);
        let warnings = find_tag_namespace_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].resource, "app-cfg");
        assert_eq!(warnings[0].unnamespaced_tags, vec!["production"]);
    }

    #[test]
    fn test_tag_namespace_multiple_bad_tags() {
        let mut r = make_resource("service");
        r.tags = vec![
            "critical".to_string(),
            "production".to_string(),
            "tier:web".to_string(),
        ];
        let config = make_config(vec![("svc", r)]);
        let warnings = find_tag_namespace_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].unnamespaced_tags.len(), 2);
    }

    #[test]
    fn test_tag_namespace_no_tags_no_warning() {
        let r = make_resource("package");
        let config = make_config(vec![("pkg", r)]);
        let warnings = find_tag_namespace_warnings(&config);
        assert!(warnings.is_empty());
    }

    // -- FJ-1068: Machine capacity tests --

    #[test]
    fn test_machine_capacity_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_machine_capacity_warnings(&config, 20);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_machine_capacity_under_threshold() {
        let mut resources = Vec::new();
        for i in 0..5 {
            let r = make_resource("file");
            resources.push((format!("res-{}", i), r));
        }
        let pairs: Vec<(&str, types::Resource)> = resources
            .iter()
            .map(|(n, r)| (n.as_str(), r.clone()))
            .collect();
        let config = make_config(pairs);
        let warnings = find_machine_capacity_warnings(&config, 20);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_machine_capacity_over_threshold() {
        let mut resources = Vec::new();
        for i in 0..25 {
            let mut r = make_resource("file");
            r.machine = types::MachineTarget::Single("web".to_string());
            resources.push((format!("res-{}", i), r));
        }
        let pairs: Vec<(&str, types::Resource)> = resources
            .iter()
            .map(|(n, r)| (n.as_str(), r.clone()))
            .collect();
        let config = make_config(pairs);
        let warnings = find_machine_capacity_warnings(&config, 20);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].machine, "web");
        assert_eq!(warnings[0].resource_count, 25);
        assert_eq!(warnings[0].threshold, 20);
    }

    #[test]
    fn test_machine_capacity_multiple_machines() {
        let mut resources = Vec::new();
        for i in 0..25 {
            let mut r = make_resource("file");
            r.machine = types::MachineTarget::Multiple(vec![
                "web".to_string(),
                "db".to_string(),
            ]);
            resources.push((format!("res-{}", i), r));
        }
        let pairs: Vec<(&str, types::Resource)> = resources
            .iter()
            .map(|(n, r)| (n.as_str(), r.clone()))
            .collect();
        let config = make_config(pairs);
        let warnings = find_machine_capacity_warnings(&config, 20);
        // Both "web" and "db" have 25 resources each.
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn test_machine_capacity_exact_threshold_no_warning() {
        let mut resources = Vec::new();
        for i in 0..20 {
            let mut r = make_resource("file");
            r.machine = types::MachineTarget::Single("app".to_string());
            resources.push((format!("res-{}", i), r));
        }
        let pairs: Vec<(&str, types::Resource)> = resources
            .iter()
            .map(|(n, r)| (n.as_str(), r.clone()))
            .collect();
        let config = make_config(pairs);
        let warnings = find_machine_capacity_warnings(&config, 20);
        // Exactly at threshold — no warning (only warns if >20).
        assert!(warnings.is_empty());
    }
}
