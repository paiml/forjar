//! Phase 103 — Dependency Depth Histogram & Redundancy Analysis (FJ-1087, FJ-1090).

use crate::core::types;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::Path;

// ============================================================================
// FJ-1087 helpers: Dependency depth histogram
// ============================================================================

/// Build adjacency (parent -> children) and in-degree from depends_on edges.
fn build_depth_adjacency(
    config: &types::ForjarConfig,
) -> (HashMap<String, usize>, HashMap<String, Vec<String>>) {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for name in config.resources.keys() {
        in_degree.entry(name.clone()).or_insert(0);
        children.entry(name.clone()).or_default();
    }
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if config.resources.contains_key(dep) {
                children.entry(dep.clone()).or_default().push(name.clone());
                *in_degree.entry(name.clone()).or_default() += 1;
            }
        }
    }
    (in_degree, children)
}

/// BFS from roots, computing max depth (longest path from any root) per node.
fn compute_max_depths(
    in_degree: &HashMap<String, usize>,
    children: &HashMap<String, Vec<String>>,
) -> HashMap<String, usize> {
    let mut depths: HashMap<String, usize> = HashMap::new();
    let mut remaining: HashMap<String, usize> = in_degree.clone();
    let mut queue: VecDeque<String> = remaining
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    for root in &queue {
        depths.insert(root.clone(), 0);
    }
    while let Some(node) = queue.pop_front() {
        let current = depths[&node];
        for child in children.get(&node).cloned().unwrap_or_default() {
            let entry = depths.entry(child.clone()).or_insert(0);
            if current + 1 > *entry {
                *entry = current + 1;
            }
            let deg = remaining.get_mut(&child).expect("node in graph");
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(child);
            }
        }
    }
    depths
}

/// Build a histogram: depth -> count of resources at that depth.
fn build_histogram(depths: &HashMap<String, usize>) -> BTreeMap<usize, usize> {
    let mut hist: BTreeMap<usize, usize> = BTreeMap::new();
    for &d in depths.values() {
        *hist.entry(d).or_insert(0) += 1;
    }
    hist
}

fn print_depth_histogram_json(hist: &BTreeMap<usize, usize>, max_depth: usize) {
    let entries: Vec<String> = hist
        .iter()
        .map(|(d, c)| format!("\"{}\":{}", d, c))
        .collect();
    println!(
        "{{\"depth_histogram\":{{{}}},\"max_depth\":{}}}",
        entries.join(","),
        max_depth
    );
}

fn print_depth_histogram_text(hist: &BTreeMap<usize, usize>, max_depth: usize) {
    println!("Dependency depth histogram (max_depth={}):", max_depth);
    for (depth, count) in hist {
        println!("  depth {}: {} resource(s)", depth, count);
    }
}

// ============================================================================
// FJ-1090 helpers: Dependency redundancy analysis
// ============================================================================

/// A redundant edge: resource depends on dep both directly and transitively.
struct RedundantEdge {
    resource: String,
    redundant_dep: String,
}

/// Find all redundant dependency edges.
///
/// An edge A -> B is redundant if B is also reachable from A through other
/// direct dependencies of A (i.e., A -> C -> ... -> B).
/// Get valid direct dependencies for a resource.
fn valid_deps(resource: &types::Resource, config: &types::ForjarConfig) -> Vec<String> {
    resource.depends_on.iter().filter(|d| config.resources.contains_key(*d)).cloned().collect()
}

/// Check if `dep` is redundant for `name` — reachable via another direct dependency.
fn is_dep_redundant(dep: &str, direct_deps: &[String], config: &types::ForjarConfig) -> bool {
    direct_deps.iter().any(|other| other != dep && reachable_via_upstream(other, dep, config))
}

fn find_redundant_edges(config: &types::ForjarConfig) -> Vec<RedundantEdge> {
    let mut sorted_resources: Vec<&String> = config.resources.keys().collect();
    sorted_resources.sort();
    let mut results: Vec<RedundantEdge> = Vec::new();
    for name in sorted_resources {
        let direct_deps = valid_deps(&config.resources[name], config);
        if direct_deps.len() < 2 { continue; }
        for dep in &direct_deps {
            if is_dep_redundant(dep, &direct_deps, config) {
                results.push(RedundantEdge { resource: name.clone(), redundant_dep: dep.clone() });
            }
        }
    }
    results
}

/// Check if `target` is reachable from `start` by following depends_on edges
/// upstream (start's dependencies, their dependencies, etc.).
fn reachable_via_upstream(
    start: &str,
    target: &str,
    config: &types::ForjarConfig,
) -> bool {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(start.to_string());
    visited.insert(start.to_string());
    while let Some(node) = queue.pop_front() {
        if let Some(resource) = config.resources.get(&node) {
            for dep in &resource.depends_on {
                if dep == target {
                    return true;
                }
                if config.resources.contains_key(dep) && visited.insert(dep.clone()) {
                    queue.push_back(dep.clone());
                }
            }
        }
    }
    false
}

fn print_redundancy_json(edges: &[RedundantEdge]) {
    let items: Vec<String> = edges
        .iter()
        .map(|e| {
            format!(
                "{{\"resource\":\"{}\",\"redundant_dep\":\"{}\"}}",
                e.resource, e.redundant_dep
            )
        })
        .collect();
    println!(
        "{{\"redundant_edges\":[{}],\"count\":{}}}",
        items.join(","),
        edges.len()
    );
}

fn print_redundancy_text(edges: &[RedundantEdge]) {
    println!("Redundancy analysis ({} redundant edges):", edges.len());
    if edges.is_empty() {
        println!("  (no redundant dependency edges detected)");
        return;
    }
    for e in edges {
        println!(
            "  {} -> {} (also reachable transitively)",
            e.resource, e.redundant_dep
        );
    }
}

// ============================================================================
// Public commands
// ============================================================================

/// FJ-1087: Histogram of dependency chain depths across all resources.
///
/// Builds adjacency from depends_on, computes depth (longest path from any
/// root) for each resource, then prints a histogram of depth counts.
pub(crate) fn cmd_graph_resource_dependency_depth_histogram(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"depth_histogram\":{{}},\"max_depth\":0}}");
        } else {
            println!("Dependency depth histogram (max_depth=0):");
            println!("  (no resources)");
        }
        return Ok(());
    }
    let (in_degree, children) = build_depth_adjacency(&config);
    let depths = compute_max_depths(&in_degree, &children);
    let hist = build_histogram(&depths);
    let max_depth = hist.keys().last().copied().unwrap_or(0);
    if json {
        print_depth_histogram_json(&hist, max_depth);
    } else {
        print_depth_histogram_text(&hist, max_depth);
    }
    Ok(())
}

/// FJ-1090: Identify redundant dependency edges.
///
/// An edge A->B is redundant if A depends on B directly but also transitively
/// via another dependency chain (A->C->...->B).
pub(crate) fn cmd_graph_resource_dependency_redundancy_analysis(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"redundant_edges\":[],\"count\":0}}");
        } else {
            println!("Redundancy analysis (0 redundant edges):");
            println!("  (no redundant dependency edges detected)");
        }
        return Ok(());
    }
    let edges = find_redundant_edges(&config);
    if json {
        print_redundancy_json(&edges);
    } else {
        print_redundancy_text(&edges);
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

    const REDUNDANT_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a, b]\n";

    // ── FJ-1087: depth histogram ──

    #[test]
    fn test_fj1087_depth_histogram_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_depth_histogram(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1087_depth_histogram_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_depth_histogram(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1087_depth_histogram_chain() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_depth_histogram(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1087_depth_histogram_chain_json() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_depth_histogram(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1087_histogram_values() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let (in_degree, children) = build_depth_adjacency(&config);
        let depths = compute_max_depths(&in_degree, &children);
        assert_eq!(depths["a"], 0);
        assert_eq!(depths["b"], 1);
        assert_eq!(depths["c"], 2);
        let hist = build_histogram(&depths);
        assert_eq!(hist[&0], 1); // a
        assert_eq!(hist[&1], 1); // b
        assert_eq!(hist[&2], 1); // c
    }

    #[test]
    fn test_fj1087_file_not_found() {
        let result = cmd_graph_resource_dependency_depth_histogram(Path::new("/nonexistent"), false);
        assert!(result.is_err());
    }

    // ── FJ-1090: redundancy analysis ──

    #[test]
    fn test_fj1090_redundancy_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_redundancy_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1090_redundancy_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_redundancy_analysis(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1090_redundancy_chain_no_redundancy() {
        let f = write_temp_config(CHAIN_CFG);
        let result = cmd_graph_resource_dependency_redundancy_analysis(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj1090_redundancy_with_redundant_edge() {
        let f = write_temp_config(REDUNDANT_CFG);
        assert!(cmd_graph_resource_dependency_redundancy_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1090_redundancy_with_redundant_edge_json() {
        let f = write_temp_config(REDUNDANT_CFG);
        assert!(cmd_graph_resource_dependency_redundancy_analysis(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1090_find_redundant_edges_helper() {
        // c depends on [a, b] and b depends on [a], so c->a is redundant (reachable via c->b->a)
        let config: types::ForjarConfig = serde_yaml_ng::from_str(REDUNDANT_CFG).unwrap();
        let edges = find_redundant_edges(&config);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].resource, "c");
        assert_eq!(edges[0].redundant_dep, "a");
    }

    #[test]
    fn test_fj1090_no_redundancy_in_chain() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let edges = find_redundant_edges(&config);
        assert!(edges.is_empty());
    }

    #[test]
    fn test_fj1090_file_not_found() {
        let result = cmd_graph_resource_dependency_redundancy_analysis(Path::new("/nonexistent"), false);
        assert!(result.is_err());
    }
}
