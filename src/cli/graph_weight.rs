//! Phase 106 — Graph Weight Analysis: dependency weight scoring & topological summary (FJ-1111, FJ-1114).

use crate::core::types;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::Path;

// ============================================================================
// FJ-1111: Resource dependency weight analysis
// ============================================================================

struct WeightEntry {
    resource: String,
    weight: usize,
    fan_in: usize,
    fan_out: usize,
    depth: usize,
}

/// Compute fan-in for each resource (number of resources that depend on it).
fn compute_fan_in_map(config: &types::ForjarConfig) -> HashMap<String, usize> {
    let mut fan_in: HashMap<String, usize> = HashMap::new();
    for name in config.resources.keys() {
        fan_in.entry(name.clone()).or_insert(0);
    }
    for resource in config.resources.values() {
        for dep in &resource.depends_on {
            if config.resources.contains_key(dep) {
                *fan_in.entry(dep.clone()).or_insert(0) += 1;
            }
        }
    }
    fan_in
}

/// Compute fan-out for each resource (number of direct dependencies).
fn compute_fan_out_map(config: &types::ForjarConfig) -> HashMap<String, usize> {
    config
        .resources
        .iter()
        .map(|(name, res)| {
            let count = res
                .depends_on
                .iter()
                .filter(|d| config.resources.contains_key(*d))
                .count();
            (name.clone(), count)
        })
        .collect()
}

fn build_reverse_adj(config: &types::ForjarConfig) -> HashMap<String, Vec<String>> {
    let mut reverse_adj: HashMap<String, Vec<String>> = HashMap::new();
    for (name, res) in &config.resources {
        for dep in &resource_deps(res, config) {
            reverse_adj
                .entry(dep.clone())
                .or_default()
                .push(name.clone());
        }
    }
    reverse_adj
}
fn find_roots(config: &types::ForjarConfig) -> Vec<String> {
    config
        .resources
        .iter()
        .filter(|(_, res)| {
            res.depends_on
                .iter()
                .filter(|d| config.resources.contains_key(*d))
                .count()
                == 0
        })
        .map(|(n, _)| n.clone())
        .collect()
}
fn compute_depth_map(config: &types::ForjarConfig) -> HashMap<String, usize> {
    let mut depth: HashMap<String, usize> = HashMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    for name in find_roots(config) {
        depth.insert(name.clone(), 0);
        queue.push_back(name);
    }
    let reverse_adj = build_reverse_adj(config);
    while let Some(node) = queue.pop_front() {
        let d = depth[&node];
        if let Some(dependents) = reverse_adj.get(&node) {
            for dep in dependents {
                let entry = depth.entry(dep.clone()).or_insert(0);
                if d + 1 > *entry {
                    *entry = d + 1;
                }
                queue.push_back(dep.clone());
            }
        }
    }
    for name in config.resources.keys() {
        depth.entry(name.clone()).or_insert(0);
    }
    depth
}

/// Get valid dependencies for a resource.
fn resource_deps(res: &types::Resource, config: &types::ForjarConfig) -> Vec<String> {
    res.depends_on
        .iter()
        .filter(|d| config.resources.contains_key(*d))
        .cloned()
        .collect()
}

/// Compute weighted dependency scores: fan_in + fan_out + depth per resource.
fn compute_weight_entries(config: &types::ForjarConfig) -> Vec<WeightEntry> {
    let fan_in = compute_fan_in_map(config);
    let fan_out = compute_fan_out_map(config);
    let depth = compute_depth_map(config);
    let mut entries: Vec<WeightEntry> = config
        .resources
        .keys()
        .map(|name| {
            let fi = fan_in.get(name).copied().unwrap_or(0);
            let fo = fan_out.get(name).copied().unwrap_or(0);
            let d = depth.get(name).copied().unwrap_or(0);
            WeightEntry {
                resource: name.clone(),
                weight: fi + fo + d,
                fan_in: fi,
                fan_out: fo,
                depth: d,
            }
        })
        .collect();
    entries.sort_by(|a, b| b.weight.cmp(&a.weight).then(a.resource.cmp(&b.resource)));
    entries
}

fn print_weight_json(entries: &[WeightEntry]) {
    let items: Vec<String> = entries
        .iter()
        .map(|e| {
            format!(
                "{{\"resource\":\"{}\",\"weight\":{},\"fan_in\":{},\"fan_out\":{},\"depth\":{}}}",
                e.resource, e.weight, e.fan_in, e.fan_out, e.depth
            )
        })
        .collect();
    println!("{{\"dependency_weight_analysis\":[{}]}}", items.join(","));
}

fn print_weight_text(entries: &[WeightEntry]) {
    if entries.is_empty() {
        println!("Dependency weight analysis: (no resources)");
        return;
    }
    println!("Dependency weight analysis:");
    for e in entries {
        println!(
            "  {}: weight={} (fan_in={}, fan_out={}, depth={})",
            e.resource, e.weight, e.fan_in, e.fan_out, e.depth
        );
    }
}

/// FJ-1111: Compute weighted dependency score per resource (fan_in + fan_out + depth),
/// sorted by weight descending.
pub(crate) fn cmd_graph_resource_dependency_weight_analysis(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"dependency_weight_analysis\":[]}}");
        } else {
            println!("Dependency weight analysis: (no resources)");
        }
        return Ok(());
    }
    let entries = compute_weight_entries(&config);
    if json {
        print_weight_json(&entries);
    } else {
        print_weight_text(&entries);
    }
    Ok(())
}

// ============================================================================
// FJ-1114: Resource dependency topological summary
// ============================================================================

struct TopoLayer {
    layer: usize,
    resources: Vec<String>,
}

fn build_in_degree(config: &types::ForjarConfig) -> BTreeMap<String, usize> {
    let mut in_count: BTreeMap<String, usize> = BTreeMap::new();
    for name in config.resources.keys() {
        in_count.entry(name.clone()).or_insert(0);
    }
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if config.resources.contains_key(dep) {
                *in_count.entry(name.clone()).or_insert(0) += 1;
            }
        }
    }
    in_count
}
fn compute_topological_layers(config: &types::ForjarConfig) -> Vec<TopoLayer> {
    let mut in_count = build_in_degree(config);
    let reverse_adj = build_reverse_adj(config);
    let mut layers: Vec<TopoLayer> = Vec::new();
    let mut queue: VecDeque<String> = {
        let mut roots: Vec<String> = in_count
            .iter()
            .filter(|(_, c)| **c == 0)
            .map(|(n, _)| n.clone())
            .collect();
        roots.sort();
        roots.into_iter().collect()
    };
    while !queue.is_empty() {
        let mut current_layer: Vec<String> = queue.drain(..).collect();
        current_layer.sort();
        let mut next: VecDeque<String> = VecDeque::new();
        for node in &current_layer {
            for dep in reverse_adj.get(node).into_iter().flatten() {
                let c = in_count.get_mut(dep).unwrap();
                *c -= 1;
                if *c == 0 {
                    next.push_back(dep.clone());
                }
            }
        }
        layers.push(TopoLayer {
            layer: layers.len(),
            resources: current_layer,
        });
        queue = next;
    }
    layers
}

fn print_topo_json(layers: &[TopoLayer]) {
    let items: Vec<String> = layers
        .iter()
        .map(|l| {
            let res: Vec<String> = l.resources.iter().map(|r| format!("\"{r}\"")).collect();
            format!(
                "{{\"layer\":{},\"count\":{},\"resources\":[{}]}}",
                l.layer,
                l.resources.len(),
                res.join(",")
            )
        })
        .collect();
    println!("{{\"topological_summary\":[{}]}}", items.join(","));
}

fn print_topo_text(layers: &[TopoLayer]) {
    if layers.is_empty() {
        println!("Topological summary: (no resources)");
        return;
    }
    println!("Topological summary:");
    for l in layers {
        println!(
            "  layer {}: {} resources ({})",
            l.layer,
            l.resources.len(),
            l.resources.join(", ")
        );
    }
}

/// FJ-1114: Print topological layers (BFS from roots) with resource counts.
pub(crate) fn cmd_graph_resource_dependency_topological_summary(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"topological_summary\":[]}}");
        } else {
            println!("Topological summary: (no resources)");
        }
        return Ok(());
    }
    let layers = compute_topological_layers(&config);
    if json {
        print_topo_json(&layers);
    } else {
        print_topo_text(&layers);
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

    const CHAIN_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  base:\n    type: file\n    machine: m\n    path: /tmp/base\n    content: base\n  mid:\n    type: file\n    machine: m\n    path: /tmp/mid\n    content: mid\n    depends_on: [base]\n  leaf:\n    type: file\n    machine: m\n    path: /tmp/leaf\n    content: leaf\n    depends_on: [mid]\n";

    const FAN_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  root:\n    type: file\n    machine: m\n    path: /tmp/root\n    content: root\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    depends_on: [root]\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [root]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [root]\n";

    // ── FJ-1111: weight analysis ──

    #[test]
    fn test_fj1111_weight_analysis_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_weight_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1111_weight_analysis_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_weight_analysis(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1111_weight_analysis_chain() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_weight_analysis(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1111_weight_analysis_chain_json() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_weight_analysis(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1111_file_not_found() {
        let result =
            cmd_graph_resource_dependency_weight_analysis(Path::new("/nonexistent"), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj1111_compute_weight_entries_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(FAN_CFG).unwrap();
        let entries = compute_weight_entries(&config);
        // root: fan_in=3, fan_out=0, depth=0 => weight=3
        let root = entries.iter().find(|e| e.resource == "root").unwrap();
        assert_eq!(root.fan_in, 3);
        assert_eq!(root.fan_out, 0);
        assert_eq!(root.weight, 3);
    }

    // ── FJ-1114: topological summary ──

    #[test]
    fn test_fj1114_topo_summary_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_topological_summary(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1114_topo_summary_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_topological_summary(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1114_topo_summary_chain() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_topological_summary(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1114_topo_summary_chain_json() {
        let f = write_temp_config(CHAIN_CFG);
        assert!(cmd_graph_resource_dependency_topological_summary(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1114_file_not_found() {
        let result =
            cmd_graph_resource_dependency_topological_summary(Path::new("/nonexistent"), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj1114_compute_layers_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CHAIN_CFG).unwrap();
        let layers = compute_topological_layers(&config);
        // base (layer 0), mid (layer 1), leaf (layer 2)
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].resources, vec!["base"]);
        assert_eq!(layers[1].resources, vec!["mid"]);
        assert_eq!(layers[2].resources, vec!["leaf"]);
    }
}
