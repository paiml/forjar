//! Phase 104 — Operational Maturity & Dependency Governance: graph commands (FJ-1095, FJ-1098).

use crate::core::types;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::path::Path;

// ============================================================================
// FJ-1095: Resource dependency change impact radius
// ============================================================================

/// Build forward adjacency: resource -> list of direct dependents.
fn build_forward_adjacency(config: &types::ForjarConfig) -> BTreeMap<String, Vec<String>> {
    let mut adj: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in config.resources.keys() {
        adj.entry(name.clone()).or_default();
    }
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if config.resources.contains_key(dep) {
                adj.entry(dep.clone()).or_default().push(name.clone());
            }
        }
    }
    for deps in adj.values_mut() {
        deps.sort();
    }
    adj
}

/// BFS from `start` through forward adjacency, returning the count of
/// reachable nodes (excluding `start` itself).
fn bfs_reachable_count(start: &str, adj: &BTreeMap<String, Vec<String>>) -> usize {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    visited.insert(start.to_string());
    queue.push_back(start.to_string());
    while let Some(node) = queue.pop_front() {
        for neighbor in adj.get(&node).cloned().unwrap_or_default() {
            if visited.insert(neighbor.clone()) {
                queue.push_back(neighbor);
            }
        }
    }
    // Exclude the start node itself from the count
    visited.len().saturating_sub(1)
}

struct ImpactEntry {
    name: String,
    radius: usize,
}

/// Compute the blast radius (transitive dependents via BFS) for each resource.
fn compute_impact_radius(config: &types::ForjarConfig) -> Vec<ImpactEntry> {
    let adj = build_forward_adjacency(config);
    let mut entries: Vec<ImpactEntry> = config
        .resources
        .keys()
        .map(|name| {
            let radius = bfs_reachable_count(name, &adj);
            ImpactEntry { name: name.clone(), radius }
        })
        .collect();
    entries.sort_by(|a, b| b.radius.cmp(&a.radius).then(a.name.cmp(&b.name)));
    entries
}

fn print_impact_radius_json(entries: &[ImpactEntry]) {
    let items: Vec<String> = entries
        .iter()
        .map(|e| format!("{{\"name\":\"{}\",\"radius\":{}}}", e.name, e.radius))
        .collect();
    println!("{{\"impact_radius\":[{}]}}", items.join(","));
}

fn print_impact_radius_text(entries: &[ImpactEntry]) {
    println!("Change impact radius:");
    if entries.is_empty() {
        println!("  (no resources)");
        return;
    }
    for e in entries {
        println!("  {} (radius={})", e.name, e.radius);
    }
}

/// FJ-1095: Compute blast radius of changing each resource (count of
/// transitive dependents via BFS).
pub(crate) fn cmd_graph_resource_dependency_change_impact_radius(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;
    if cfg.resources.is_empty() {
        if json {
            println!("{{\"impact_radius\":[]}}");
        } else {
            println!("Change impact radius:");
            println!("  (no resources)");
        }
        return Ok(());
    }
    let entries = compute_impact_radius(&cfg);
    if json {
        print_impact_radius_json(&entries);
    } else {
        print_impact_radius_text(&entries);
    }
    Ok(())
}

// ============================================================================
// FJ-1098: Resource dependency sibling analysis
// ============================================================================

struct SiblingGroup {
    deps: Vec<String>,
    members: Vec<String>,
}

/// Identify resources that share the exact same set of dependencies.
/// Group by sorted depends_on list; report groups with 2+ members.
fn find_sibling_groups(config: &types::ForjarConfig) -> Vec<SiblingGroup> {
    let mut groups: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
    let mut sorted_names: Vec<&String> = config.resources.keys().collect();
    sorted_names.sort();
    for name in sorted_names {
        let resource = &config.resources[name];
        let mut deps: Vec<String> = resource
            .depends_on
            .iter()
            .filter(|d| config.resources.contains_key(*d))
            .cloned()
            .collect();
        deps.sort();
        groups.entry(deps).or_default().push(name.clone());
    }
    let mut result: Vec<SiblingGroup> = groups
        .into_iter()
        .filter(|(_, members)| members.len() >= 2)
        .map(|(deps, members)| SiblingGroup { deps, members })
        .collect();
    // Sort groups by member count descending, then by first member name
    result.sort_by(|a, b| {
        b.members
            .len()
            .cmp(&a.members.len())
            .then(a.members[0].cmp(&b.members[0]))
    });
    result
}

fn print_sibling_json(groups: &[SiblingGroup]) {
    let items: Vec<String> = groups
        .iter()
        .map(|g| {
            let deps_str: Vec<String> = g.deps.iter().map(|d| format!("\"{}\"", d)).collect();
            let members_str: Vec<String> =
                g.members.iter().map(|m| format!("\"{}\"", m)).collect();
            format!(
                "{{\"deps\":[{}],\"members\":[{}],\"count\":{}}}",
                deps_str.join(","),
                members_str.join(","),
                g.members.len()
            )
        })
        .collect();
    println!(
        "{{\"sibling_groups\":[{}],\"count\":{}}}",
        items.join(","),
        groups.len()
    );
}

fn print_sibling_text(groups: &[SiblingGroup]) {
    println!("Sibling analysis ({} groups):", groups.len());
    if groups.is_empty() {
        println!("  (no sibling groups detected)");
        return;
    }
    for g in groups {
        let deps_label = if g.deps.is_empty() {
            "(no dependencies)".to_string()
        } else {
            g.deps.join(", ")
        };
        println!(
            "  [{}] share deps [{}]",
            g.members.join(", "),
            deps_label
        );
    }
}

/// FJ-1098: Identify resources that share the exact same set of dependencies
/// (siblings). Groups resources by their sorted depends_on list, reports
/// groups with 2+ members.
pub(crate) fn cmd_graph_resource_dependency_sibling_analysis(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;
    if cfg.resources.is_empty() {
        if json {
            println!("{{\"sibling_groups\":[],\"count\":0}}");
        } else {
            println!("Sibling analysis (0 groups):");
            println!("  (no sibling groups detected)");
        }
        return Ok(());
    }
    let groups = find_sibling_groups(&cfg);
    if json {
        print_sibling_json(&groups);
    } else {
        print_sibling_text(&groups);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const EMPTY_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n";

    const CHAIN_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n";

    const SIBLING_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a]\n  d:\n    type: service\n    machine: m\n    name: nginx\n    depends_on: [a]\n";

    // ── FJ-1095: change impact radius ──

    #[test]
    fn test_fj1095_impact_radius_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_change_impact_radius(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1095_impact_radius_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_change_impact_radius(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1095_impact_radius_chain() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_change_impact_radius(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1095_impact_radius_chain_json() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_change_impact_radius(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1095_compute_impact_radius_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let entries = compute_impact_radius(&config);
        // a -> b -> c: a has radius 2, b has radius 1, c has radius 0
        let a = entries.iter().find(|e| e.name == "a").unwrap();
        assert_eq!(a.radius, 2);
        let b = entries.iter().find(|e| e.name == "b").unwrap();
        assert_eq!(b.radius, 1);
        let c = entries.iter().find(|e| e.name == "c").unwrap();
        assert_eq!(c.radius, 0);
    }

    #[test]
    fn test_fj1095_impact_radius_fan_out() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(SIBLING_CFG).unwrap();
        let entries = compute_impact_radius(&config);
        // a has 3 dependents (b, c, d)
        let a = entries.iter().find(|e| e.name == "a").unwrap();
        assert_eq!(a.radius, 3);
    }

    #[test]
    fn test_fj1095_build_forward_adjacency_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let adj = build_forward_adjacency(&config);
        assert_eq!(adj["a"], vec!["b".to_string()]);
        assert_eq!(adj["b"], vec!["c".to_string()]);
        assert!(adj["c"].is_empty());
    }

    #[test]
    fn test_fj1095_file_not_found() {
        let result =
            cmd_graph_resource_dependency_change_impact_radius(Path::new("/nonexistent"), false);
        assert!(result.is_err());
    }

    // ── FJ-1098: sibling analysis ──

    #[test]
    fn test_fj1098_sibling_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_sibling_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1098_sibling_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_sibling_analysis(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1098_sibling_chain_no_siblings() {
        // In a chain a->b->c, no two resources share the same deps
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_sibling_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1098_sibling_with_siblings() {
        let f = write_temp_config(SIBLING_CFG);
        assert!(cmd_graph_resource_dependency_sibling_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1098_sibling_with_siblings_json() {
        let f = write_temp_config(SIBLING_CFG);
        assert!(cmd_graph_resource_dependency_sibling_analysis(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1098_find_sibling_groups_helper() {
        // b, c, d all depend on [a] — they are siblings
        let config: types::ForjarConfig = serde_yaml_ng::from_str(SIBLING_CFG).unwrap();
        let groups = find_sibling_groups(&config);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].deps, vec!["a".to_string()]);
        assert_eq!(
            groups[0].members,
            vec!["b".to_string(), "c".to_string(), "d".to_string()]
        );
    }

    #[test]
    fn test_fj1098_no_siblings_in_chain() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let groups = find_sibling_groups(&config);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_fj1098_file_not_found() {
        let result =
            cmd_graph_resource_dependency_sibling_analysis(Path::new("/nonexistent"), false);
        assert!(result.is_err());
    }
}
