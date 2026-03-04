//! Phase 100 — Resource Health & Width Analysis: graph commands.

use crate::core::types;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::Path;

// ============================================================================
// FJ-1063: Resource dependency health overlay
// ============================================================================

fn classify_health(tags: &[String]) -> &'static str {
    let mut has_critical = false;
    for tag in tags {
        let lower = tag.to_lowercase();
        if lower == "deprecated" {
            return "deprecated";
        }
        if lower == "critical" {
            has_critical = true;
        }
    }
    if has_critical {
        "critical"
    } else {
        "healthy"
    }
}

struct HealthNode {
    name: String,
    resource_type: String,
    health: &'static str,
}

struct HealthEdge {
    source: String,
    target: String,
    source_health: &'static str,
    target_health: &'static str,
}

fn build_health_nodes(config: &types::ForjarConfig) -> Vec<HealthNode> {
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    names
        .iter()
        .map(|name| {
            let r = &config.resources[*name];
            HealthNode {
                name: (*name).clone(),
                resource_type: r.resource_type.to_string(),
                health: classify_health(&r.tags),
            }
        })
        .collect()
}

fn build_health_edges(config: &types::ForjarConfig) -> Vec<HealthEdge> {
    let health_map: HashMap<&str, &'static str> = config
        .resources
        .iter()
        .map(|(n, r)| (n.as_str(), classify_health(&r.tags)))
        .collect();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    let mut edges = Vec::new();
    for name in names {
        let resource = &config.resources[name];
        let mut deps = resource.depends_on.clone();
        deps.sort();
        for dep in &deps {
            let sh = health_map.get(name.as_str()).copied().unwrap_or("healthy");
            let th = health_map.get(dep.as_str()).copied().unwrap_or("healthy");
            edges.push(HealthEdge {
                source: name.clone(),
                target: dep.clone(),
                source_health: sh,
                target_health: th,
            });
        }
    }
    edges
}

fn print_health_overlay_json(nodes: &[HealthNode], edges: &[HealthEdge]) {
    let ns: Vec<String> = nodes
        .iter()
        .map(|n| {
            format!(
                "{{\"name\":\"{}\",\"type\":\"{}\",\"health\":\"{}\"}}",
                n.name, n.resource_type, n.health
            )
        })
        .collect();
    let es: Vec<String> = edges
        .iter()
        .map(|e| {
            format!(
                "{{\"source\":\"{}\",\"target\":\"{}\",\"source_health\":\"{}\",\"target_health\":\"{}\"}}",
                e.source, e.target, e.source_health, e.target_health
            )
        })
        .collect();
    println!(
        "{{\"health_overlay\":{{\"nodes\":[{}],\"edges\":[{}]}}}}",
        ns.join(","),
        es.join(",")
    );
}

fn print_health_overlay_text(nodes: &[HealthNode], edges: &[HealthEdge]) {
    println!("Health overlay:");
    println!("  Nodes:");
    for n in nodes {
        println!("    {} ({}): {}", n.name, n.resource_type, n.health);
    }
    println!("  Edges:");
    for e in edges {
        println!(
            "    {} \u{2192} {} [{} \u{2192} {}]",
            e.source, e.target, e.source_health, e.target_health
        );
    }
}

/// FJ-1063: Overlay resource health status on dependency graph.
pub(crate) fn cmd_graph_resource_dependency_health_overlay(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"health_overlay\":{{\"nodes\":[],\"edges\":[]}}}}");
        } else {
            println!("Health overlay:");
            println!("  Nodes:");
            println!("  Edges:");
        }
        return Ok(());
    }
    let nodes = build_health_nodes(&config);
    let edges = build_health_edges(&config);
    if json {
        print_health_overlay_json(&nodes, &edges);
    } else {
        print_health_overlay_text(&nodes, &edges);
    }
    Ok(())
}

// ============================================================================
// FJ-1066: Resource dependency width analysis
// ============================================================================

struct LevelInfo {
    level: usize,
    width: usize,
    resources: Vec<String>,
}

fn compute_levels(config: &types::ForjarConfig) -> Vec<LevelInfo> {
    if config.resources.is_empty() {
        return Vec::new();
    }
    // Build in-degree map and adjacency (dependents)
    let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
    let mut dependents: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in config.resources.keys() {
        in_degree.entry(name.clone()).or_insert(0);
        dependents.entry(name.clone()).or_default();
    }
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if config.resources.contains_key(dep) {
                *in_degree.entry(name.clone()).or_insert(0) += 1;
                dependents
                    .entry(dep.clone())
                    .or_default()
                    .push(name.clone());
            }
        }
    }
    // Kahn's algorithm with level tracking
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    let mut roots: Vec<String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    roots.sort();
    for r in roots {
        queue.push_back((r, 0));
    }
    let mut level_map: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    while let Some((node, level)) = queue.pop_front() {
        level_map.entry(level).or_default().push(node.clone());
        let mut deps = dependents.get(&node).cloned().unwrap_or_default();
        deps.sort();
        for dep in deps {
            let d = in_degree.get_mut(&dep).unwrap();
            *d -= 1;
            if *d == 0 {
                queue.push_back((dep, level + 1));
            }
        }
    }
    level_map
        .into_iter()
        .map(|(level, mut resources)| {
            resources.sort();
            let width = resources.len();
            LevelInfo {
                level,
                width,
                resources,
            }
        })
        .collect()
}

fn print_width_json(levels: &[LevelInfo], max_width: usize) {
    let items: Vec<String> = levels
        .iter()
        .map(|l| {
            let names: Vec<String> = l.resources.iter().map(|n| format!("\"{n}\"")).collect();
            format!(
                "{{\"level\":{},\"width\":{},\"resources\":[{}]}}",
                l.level,
                l.width,
                names.join(",")
            )
        })
        .collect();
    println!(
        "{{\"width_analysis\":{{\"levels\":[{}],\"max_width\":{}}}}}",
        items.join(","),
        max_width
    );
}

fn print_width_text(levels: &[LevelInfo], max_width: usize) {
    println!("Width analysis:");
    for l in levels {
        println!(
            "  Level {} (width {}): {}",
            l.level,
            l.width,
            l.resources.join(", ")
        );
    }
    println!("  Max width: {max_width}");
}

/// FJ-1066: Analyze dependency graph width per level using Kahn's algorithm.
pub(crate) fn cmd_graph_resource_dependency_width_analysis(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"width_analysis\":{{\"levels\":[],\"max_width\":0}}}}");
        } else {
            println!("Width analysis:");
            println!("  Max width: 0");
        }
        return Ok(());
    }
    let levels = compute_levels(&config);
    let max_width = levels.iter().map(|l| l.width).max().unwrap_or(0);
    if json {
        print_width_json(&levels, max_width);
    } else {
        print_width_text(&levels, max_width);
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

    const HEALTH_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: service\n    machine: m\n    name: nginx\n    tags: [deprecated]\n    depends_on: [a]\n  c:\n    type: package\n    machine: m\n    packages: [curl]\n    tags: [critical]\n";

    const WIDTH_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n  d:\n    type: service\n    machine: m\n    name: nginx\n    depends_on: [a, b]\n";

    // ── FJ-1063: health overlay ──

    #[test]
    fn test_fj1063_health_overlay_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_health_overlay(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1063_health_overlay_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_health_overlay(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1063_health_overlay_with_tags() {
        let f = write_temp_config(HEALTH_CFG);
        assert!(cmd_graph_resource_dependency_health_overlay(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1063_health_overlay_with_tags_json() {
        let f = write_temp_config(HEALTH_CFG);
        assert!(cmd_graph_resource_dependency_health_overlay(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1063_classify_health_helper() {
        assert_eq!(classify_health(&[]), "healthy");
        assert_eq!(classify_health(&["web".to_string()]), "healthy");
        assert_eq!(classify_health(&["deprecated".to_string()]), "deprecated");
        assert_eq!(classify_health(&["critical".to_string()]), "critical");
        assert_eq!(
            classify_health(&["critical".to_string(), "deprecated".to_string()]),
            "deprecated"
        );
    }

    #[test]
    fn test_fj1063_build_health_nodes() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(HEALTH_CFG).unwrap();
        let nodes = build_health_nodes(&config);
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].name, "a");
        assert_eq!(nodes[0].health, "healthy");
        assert_eq!(nodes[1].name, "b");
        assert_eq!(nodes[1].health, "deprecated");
        assert_eq!(nodes[2].name, "c");
        assert_eq!(nodes[2].health, "critical");
    }

    // ── FJ-1066: width analysis ──

    #[test]
    fn test_fj1066_width_analysis_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_width_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1066_width_analysis_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_width_analysis(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1066_width_analysis_with_deps() {
        let f = write_temp_config(WIDTH_CFG);
        assert!(cmd_graph_resource_dependency_width_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1066_width_analysis_with_deps_json() {
        let f = write_temp_config(WIDTH_CFG);
        assert!(cmd_graph_resource_dependency_width_analysis(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1066_compute_levels_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(WIDTH_CFG).unwrap();
        let levels = compute_levels(&config);
        assert!(!levels.is_empty());
        // Level 0 should have a, b, c (roots); level 1 should have d
        assert_eq!(levels[0].level, 0);
        assert_eq!(levels[0].width, 3);
        assert!(levels[0].resources.contains(&"a".to_string()));
        assert!(levels[0].resources.contains(&"b".to_string()));
        assert!(levels[0].resources.contains(&"c".to_string()));
        assert_eq!(levels[1].level, 1);
        assert_eq!(levels[1].width, 1);
        assert_eq!(levels[1].resources, vec!["d".to_string()]);
    }

    #[test]
    fn test_fj1066_max_width() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(WIDTH_CFG).unwrap();
        let levels = compute_levels(&config);
        let max_width = levels.iter().map(|l| l.width).max().unwrap_or(0);
        assert_eq!(max_width, 3);
    }
}
