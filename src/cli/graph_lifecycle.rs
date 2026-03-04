//! Phase 99 — Resource Lifecycle & Dependency Age: graph commands.
#![allow(dead_code)]

use crate::core::types;
use std::collections::BTreeMap;
use std::path::Path;

// ============================================================================
// FJ-1055: Resource lifecycle stage map
// ============================================================================

fn classify_stage(tags: &[String]) -> &'static str {
    for tag in tags {
        let lower = tag.to_lowercase();
        if lower == "deprecated" {
            return "deprecated";
        }
        if lower == "stable" {
            return "stable";
        }
    }
    "active"
}

fn build_stage_map(config: &types::ForjarConfig) -> BTreeMap<&'static str, Vec<String>> {
    let mut stages: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for (name, resource) in &config.resources {
        let stage = classify_stage(&resource.tags);
        stages.entry(stage).or_default().push(name.clone());
    }
    for members in stages.values_mut() {
        members.sort();
    }
    stages
}

fn print_stage_map_json(stages: &BTreeMap<&'static str, Vec<String>>) {
    let entries: Vec<String> = stages
        .iter()
        .map(|(stage, members)| {
            let names: Vec<String> = members.iter().map(|n| format!("\"{n}\"")).collect();
            format!("\"{}\":[{}]", stage, names.join(","))
        })
        .collect();
    println!(
        "{{\"lifecycle_stage_map\":{{\"stages\":{{{}}}}}}}",
        entries.join(",")
    );
}

fn print_stage_map_text(stages: &BTreeMap<&'static str, Vec<String>>) {
    if stages.is_empty() {
        println!("No resources to classify.");
        return;
    }
    println!("Lifecycle stage map:");
    for (stage, members) in stages {
        println!("  {}: {}", stage, members.join(", "));
    }
}

/// FJ-1055: Map resources to lifecycle stages (active, stable, deprecated) based on tags.
pub(crate) fn cmd_graph_resource_lifecycle_stage_map(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"lifecycle_stage_map\":{{\"stages\":{{}}}}}}");
        } else {
            println!("No resources to classify.");
        }
        return Ok(());
    }
    let stages = build_stage_map(&config);
    if json {
        print_stage_map_json(&stages);
    } else {
        print_stage_map_text(&stages);
    }
    Ok(())
}

// ============================================================================
// FJ-1058: Resource dependency age overlay
// ============================================================================

struct DependencyEdge {
    source: String,
    target: String,
    source_type: String,
    target_type: String,
}

fn build_dependency_edges(config: &types::ForjarConfig) -> Vec<DependencyEdge> {
    let mut edges: Vec<DependencyEdge> = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        let mut deps = resource.depends_on.clone();
        deps.sort();
        for dep in &deps {
            let target_type = config
                .resources
                .get(dep)
                .map(|r| r.resource_type.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            edges.push(DependencyEdge {
                source: name.clone(),
                target: dep.clone(),
                source_type: resource.resource_type.to_string(),
                target_type,
            });
        }
    }
    edges
}

fn print_edges_json(edges: &[DependencyEdge]) {
    let items: Vec<String> = edges
        .iter()
        .map(|e| {
            format!(
                "{{\"source\":\"{}\",\"target\":\"{}\",\"source_type\":\"{}\",\"target_type\":\"{}\"}}",
                e.source, e.target, e.source_type, e.target_type
            )
        })
        .collect();
    println!(
        "{{\"dependency_age_overlay\":{{\"edges\":[{}]}}}}",
        items.join(",")
    );
}

fn print_edges_text(edges: &[DependencyEdge]) {
    println!("Dependency age overlay ({} edges):", edges.len());
    for e in edges {
        println!(
            "  {} ({}) \u{2192} {} ({})",
            e.source, e.source_type, e.target, e.target_type
        );
    }
}

/// FJ-1058: Overlay resource type info on dependency graph edges.
pub(crate) fn cmd_graph_resource_dependency_age_overlay(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"dependency_age_overlay\":{{\"edges\":[]}}}}");
        } else {
            println!("Dependency age overlay (0 edges):");
        }
        return Ok(());
    }
    let edges = build_dependency_edges(&config);
    if json {
        print_edges_json(&edges);
    } else {
        print_edges_text(&edges);
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

    // ── FJ-1055: lifecycle stage map ──

    #[test]
    fn test_fj1055_stage_map_empty() {
        let f = write_temp_config(
            "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
        );
        assert!(cmd_graph_resource_lifecycle_stage_map(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1055_stage_map_json_empty() {
        let f = write_temp_config(
            "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
        );
        assert!(cmd_graph_resource_lifecycle_stage_map(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1055_stage_map_classification() {
        let f = write_temp_config(
            "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    tags: [stable]\n  c:\n    type: package\n    machine: m\n    packages: [curl]\n    tags: [deprecated]\n",
        );
        assert!(cmd_graph_resource_lifecycle_stage_map(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1055_stage_map_classification_json() {
        let f = write_temp_config(
            "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: service\n    machine: m\n    name: nginx\n    tags: [stable]\n",
        );
        assert!(cmd_graph_resource_lifecycle_stage_map(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1055_classify_stage_helper() {
        assert_eq!(classify_stage(&[]), "active");
        assert_eq!(classify_stage(&["web".to_string()]), "active");
        assert_eq!(classify_stage(&["stable".to_string()]), "stable");
        assert_eq!(classify_stage(&["deprecated".to_string()]), "deprecated");
        assert_eq!(
            classify_stage(&["critical".to_string(), "deprecated".to_string()]),
            "deprecated"
        );
    }

    // ── FJ-1058: dependency age overlay ──

    #[test]
    fn test_fj1058_age_overlay_empty() {
        let f = write_temp_config(
            "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
        );
        assert!(cmd_graph_resource_dependency_age_overlay(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1058_age_overlay_json_empty() {
        let f = write_temp_config(
            "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
        );
        assert!(cmd_graph_resource_dependency_age_overlay(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1058_age_overlay_with_deps() {
        let f = write_temp_config(
            "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  svc:\n    type: service\n    machine: m\n    name: nginx\n    depends_on: [pkg]\n  pkg:\n    type: package\n    machine: m\n    packages: [nginx]\n",
        );
        assert!(cmd_graph_resource_dependency_age_overlay(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1058_age_overlay_with_deps_json() {
        let f = write_temp_config(
            "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  svc:\n    type: service\n    machine: m\n    name: nginx\n    depends_on: [pkg]\n  pkg:\n    type: package\n    machine: m\n    packages: [nginx]\n",
        );
        assert!(cmd_graph_resource_dependency_age_overlay(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1058_age_overlay_no_deps() {
        let f = write_temp_config(
            "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: package\n    machine: m\n    packages: [curl]\n",
        );
        assert!(cmd_graph_resource_dependency_age_overlay(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1058_build_edges_helper() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  svc:\n    type: service\n    machine: m\n    name: nginx\n    depends_on: [pkg]\n  pkg:\n    type: package\n    machine: m\n    packages: [nginx]\n";
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let edges = build_dependency_edges(&config);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].source, "svc");
        assert_eq!(edges[0].target, "pkg");
        assert_eq!(edges[0].source_type, "service");
        assert_eq!(edges[0].target_type, "package");
    }
}
