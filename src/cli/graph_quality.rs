//! Phase 101 — Fleet Insight & Dependency Quality: graph commands (FJ-1071, FJ-1074).
#![allow(dead_code)]

use crate::core::types;
use std::collections::BTreeMap;
use std::path::Path;

// ============================================================================
// FJ-1071: Resource dependency critical path highlight
// ============================================================================

/// Build adjacency list from config (resource -> list of dependents).
fn build_adjacency(config: &types::ForjarConfig) -> BTreeMap<String, Vec<String>> {
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
    // Sort dependents for deterministic output
    for deps in adj.values_mut() {
        deps.sort();
    }
    adj
}

/// Compute the longest path starting from a given node using DFS with memoization.
fn longest_path_from(
    node: &str,
    adj: &BTreeMap<String, Vec<String>>,
    memo: &mut BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    if let Some(cached) = memo.get(node) {
        return cached.clone();
    }
    let mut best: Vec<String> = Vec::new();
    if let Some(neighbors) = adj.get(node) {
        for neighbor in neighbors {
            let sub = longest_path_from(neighbor, adj, memo);
            if sub.len() > best.len() {
                best = sub;
            }
        }
    }
    let mut path = vec![node.to_string()];
    path.extend(best);
    memo.insert(node.to_string(), path.clone());
    path
}

/// Find the critical path (longest dependency chain) in the graph.
fn find_critical_path(config: &types::ForjarConfig) -> Vec<String> {
    let adj = build_adjacency(config);
    let mut memo: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut best_path: Vec<String> = Vec::new();
    let mut roots: Vec<&String> = config.resources.keys().collect();
    roots.sort();
    for node in roots {
        let path = longest_path_from(node, &adj, &mut memo);
        if path.len() > best_path.len() {
            best_path = path;
        }
    }
    best_path
}

fn print_critical_path_json(path: &[String]) {
    let names: Vec<String> = path.iter().map(|n| format!("\"{}\"", n)).collect();
    println!(
        "{{\"critical_path\":[{}],\"length\":{}}}",
        names.join(","),
        path.len()
    );
}

fn print_critical_path_text(path: &[String]) {
    println!("Critical path highlight:");
    if path.is_empty() {
        println!("  (no resources)");
        return;
    }
    println!("  {} (length={})", path.join(" -> "), path.len());
}

/// FJ-1071: Highlight the longest dependency chain in the graph.
pub(crate) fn cmd_graph_resource_dependency_critical_path_highlight(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;
    if cfg.resources.is_empty() {
        if json {
            println!("{{\"critical_path\":[],\"length\":0}}");
        } else {
            println!("Critical path highlight:");
            println!("  (no resources)");
        }
        return Ok(());
    }
    let path = find_critical_path(&cfg);
    if json {
        print_critical_path_json(&path);
    } else {
        print_critical_path_text(&path);
    }
    Ok(())
}

// ============================================================================
// FJ-1074: Resource dependency bottleneck detection
// ============================================================================

struct BottleneckInfo {
    name: String,
    fan_in: usize,
    dependents: Vec<String>,
}

/// Compute fan-in (number of resources that depend on each resource).
fn compute_fan_in(config: &types::ForjarConfig) -> Vec<BottleneckInfo> {
    let mut fan_in_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in config.resources.keys() {
        fan_in_map.entry(name.clone()).or_default();
    }
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if config.resources.contains_key(dep) {
                fan_in_map.entry(dep.clone()).or_default().push(name.clone());
            }
        }
    }
    let mut results: Vec<BottleneckInfo> = fan_in_map
        .into_iter()
        .filter(|(_, deps)| deps.len() >= 2)
        .map(|(name, mut deps)| {
            deps.sort();
            let fan_in = deps.len();
            BottleneckInfo { name, fan_in, dependents: deps }
        })
        .collect();
    results.sort_by(|a, b| b.fan_in.cmp(&a.fan_in).then(a.name.cmp(&b.name)));
    results
}

fn print_bottleneck_json(bottlenecks: &[BottleneckInfo]) {
    let items: Vec<String> = bottlenecks
        .iter()
        .map(|b| {
            let deps: Vec<String> = b.dependents.iter().map(|d| format!("\"{}\"", d)).collect();
            format!(
                "{{\"name\":\"{}\",\"fan_in\":{},\"dependents\":[{}]}}",
                b.name,
                b.fan_in,
                deps.join(",")
            )
        })
        .collect();
    println!("{{\"bottlenecks\":[{}]}}", items.join(","));
}

fn print_bottleneck_text(bottlenecks: &[BottleneckInfo]) {
    println!("Bottleneck detection:");
    if bottlenecks.is_empty() {
        println!("  (no bottlenecks detected)");
        return;
    }
    for b in bottlenecks {
        println!(
            "  {} (fan-in={}): depended on by {}",
            b.name,
            b.fan_in,
            b.dependents.join(", ")
        );
    }
}

/// FJ-1074: Identify resources with high fan-in that create bottlenecks.
pub(crate) fn cmd_graph_resource_dependency_bottleneck_detection(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;
    if cfg.resources.is_empty() {
        if json {
            println!("{{\"bottlenecks\":[]}}");
        } else {
            println!("Bottleneck detection:");
            println!("  (no bottlenecks detected)");
        }
        return Ok(());
    }
    let bottlenecks = compute_fan_in(&cfg);
    if json {
        print_bottleneck_json(&bottlenecks);
    } else {
        print_bottleneck_text(&bottlenecks);
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

    const BOTTLENECK_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a]\n  d:\n    type: service\n    machine: m\n    name: nginx\n    depends_on: [a]\n";

    // ── FJ-1071: critical path highlight ──

    #[test]
    fn test_fj1071_critical_path_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_critical_path_highlight(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1071_critical_path_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_critical_path_highlight(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1071_critical_path_chain() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_critical_path_highlight(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1071_critical_path_chain_json() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_critical_path_highlight(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1071_find_critical_path_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let path = find_critical_path(&config);
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], "a");
        assert_eq!(path[1], "b");
        assert_eq!(path[2], "c");
    }

    #[test]
    fn test_fj1071_build_adjacency_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let adj = build_adjacency(&config);
        assert_eq!(adj.len(), 3);
        assert_eq!(adj["a"], vec!["b".to_string()]);
        assert_eq!(adj["b"], vec!["c".to_string()]);
        assert!(adj["c"].is_empty());
    }

    // ── FJ-1074: bottleneck detection ──

    #[test]
    fn test_fj1074_bottleneck_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_bottleneck_detection(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1074_bottleneck_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_bottleneck_detection(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1074_bottleneck_with_deps() {
        let f = write_temp_config(BOTTLENECK_CFG);
        assert!(cmd_graph_resource_dependency_bottleneck_detection(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1074_bottleneck_with_deps_json() {
        let f = write_temp_config(BOTTLENECK_CFG);
        assert!(cmd_graph_resource_dependency_bottleneck_detection(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1074_compute_fan_in_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(BOTTLENECK_CFG).unwrap();
        let bottlenecks = compute_fan_in(&config);
        assert_eq!(bottlenecks.len(), 1);
        assert_eq!(bottlenecks[0].name, "a");
        assert_eq!(bottlenecks[0].fan_in, 3);
        assert_eq!(bottlenecks[0].dependents, vec!["b".to_string(), "c".to_string(), "d".to_string()]);
    }

    #[test]
    fn test_fj1074_no_bottleneck_when_low_fan_in() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let bottlenecks = compute_fan_in(&config);
        // a has fan-in=1 (only b depends on it), b has fan-in=1 (only c), c has fan-in=0
        assert!(bottlenecks.is_empty());
    }
}
