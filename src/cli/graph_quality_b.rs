use super::graph_quality::*;
use crate::core::types;
use std::collections::{HashMap, VecDeque};
use std::path::Path;

/// Find connected components via BFS on undirected adjacency.
fn find_clusters(config: &types::ForjarConfig) -> Vec<Vec<String>> {
    let adj = build_undirected_adj(config);
    let mut visited: HashMap<String, bool> = HashMap::new();
    let mut clusters: Vec<Vec<String>> = Vec::new();
    let mut sorted_keys: Vec<String> = config.resources.keys().cloned().collect();
    sorted_keys.sort();
    for start in &sorted_keys {
        if visited.contains_key(start) {
            continue;
        }
        let component = bfs_component(start, &adj, &mut visited);
        clusters.push(component);
    }
    clusters.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
    clusters
}

/// BFS to collect one connected component.
fn bfs_component(
    start: &str,
    adj: &HashMap<String, Vec<String>>,
    visited: &mut HashMap<String, bool>,
) -> Vec<String> {
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut component: Vec<String> = Vec::new();
    queue.push_back(start.to_string());
    visited.insert(start.to_string(), true);
    while let Some(node) = queue.pop_front() {
        component.push(node.clone());
        if let Some(neighbors) = adj.get(&node) {
            for nb in neighbors {
                if !visited.contains_key(nb) {
                    visited.insert(nb.clone(), true);
                    queue.push_back(nb.clone());
                }
            }
        }
    }
    component.sort();
    component
}

fn print_cluster_text(clusters: &[Vec<String>]) {
    if clusters.is_empty() {
        println!("Dependency clusters: (no resources)");
        return;
    }
    println!("Dependency clusters:");
    for (i, cluster) in clusters.iter().enumerate() {
        println!(
            "  Cluster {} ({} resources): {}",
            i,
            cluster.len(),
            cluster.join(", ")
        );
    }
}

fn print_cluster_json(clusters: &[Vec<String>]) {
    let items: Vec<String> = clusters
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let res: Vec<String> = c.iter().map(|r| format!("\"{}\"", r)).collect();
            format!("{{\"id\":{},\"resources\":[{}]}}", i, res.join(","))
        })
        .collect();
    println!(
        "{{\"resource_dependency_cluster_analysis\":{{\"cluster_count\":{},\"clusters\":[{}]}}}}",
        clusters.len(),
        items.join(",")
    );
}

/// FJ-1122: Identify clusters of tightly-coupled resources via connected components.
pub(crate) fn cmd_graph_resource_dependency_cluster_analysis(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| format!("read: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&raw).map_err(|e| format!("parse: {e}"))?;
    let clusters = find_clusters(&config);
    if json {
        print_cluster_json(&clusters);
    } else {
        print_cluster_text(&clusters);
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

    const ISOLATED_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  x:\n    type: file\n    machine: m\n    path: /tmp/x\n    content: x\n  y:\n    type: file\n    machine: m\n    path: /tmp/y\n    content: y\n  z:\n    type: file\n    machine: m\n    path: /tmp/z\n    content: z\n    depends_on: [y]\n";

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
        assert_eq!(
            bottlenecks[0].dependents,
            vec!["b".to_string(), "c".to_string(), "d".to_string()]
        );
    }

    #[test]
    fn test_fj1074_no_bottleneck_when_low_fan_in() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let bottlenecks = compute_fan_in(&config);
        // a has fan-in=1 (only b depends on it), b has fan-in=1 (only c), c has fan-in=0
        assert!(bottlenecks.is_empty());
    }

    // ── FJ-1119: critical path (Phase 107) ──
    #[test]
    fn test_fj1119_critical_path_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_critical_path(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1119_critical_path_chain_json() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_critical_path(f.path(), true).is_ok());
    }
    #[test]
    fn test_fj1119_file_not_found() {
        let r = cmd_graph_resource_dependency_critical_path(Path::new("/nonexistent"), false);
        assert!(r.is_err());
    }
    // ── FJ-1122: cluster analysis (Phase 107) ──
    #[test]
    fn test_fj1122_cluster_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_cluster_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1122_find_clusters_isolated() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(ISOLATED_CFG).unwrap();
        let clusters = find_clusters(&config);
        assert_eq!(clusters.len(), 2);
        let big = &clusters[0];
        assert_eq!(big.len(), 2);
        assert!(big.contains(&"y".to_string()));
    }
    #[test]
    fn test_fj1122_single_cluster_chain() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let clusters = find_clusters(&config);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].len(), 3);
    }
}
