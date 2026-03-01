//! Phase 105 — Graph Resilience Extensions: fan-in hotspots & cross-machine bridges (FJ-1103, FJ-1106).

use crate::core::types;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

// ============================================================================
// FJ-1103: Resource dependency fan-in hotspot detection
// ============================================================================

struct FanInEntry {
    resource: String,
    fan_in: usize,
    dependents: Vec<String>,
}

/// Compute fan-in (number of resources that depend on each resource).
fn compute_fan_in(config: &types::ForjarConfig) -> BTreeMap<String, Vec<String>> {
    let mut fan_in: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for name in config.resources.keys() {
        fan_in.entry(name.clone()).or_default();
    }
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if config.resources.contains_key(dep) {
                fan_in.entry(dep.clone()).or_default().push(name.clone());
            }
        }
    }
    for dependents in fan_in.values_mut() {
        dependents.sort();
    }
    fan_in
}

/// Identify hotspot resources where fan-in exceeds the threshold.
/// Threshold: fan-in > average * 2 OR fan-in >= 3 (whichever is lower).
fn find_fan_in_hotspots(config: &types::ForjarConfig) -> Vec<FanInEntry> {
    let fan_in_map = compute_fan_in(config);
    let total_fan_in: usize = fan_in_map.values().map(|v| v.len()).sum();
    let count = fan_in_map.len();
    let avg = if count > 0 { total_fan_in as f64 / count as f64 } else { 0.0 };
    let threshold = (avg * 2.0).max(3.0) as usize;

    let mut hotspots: Vec<FanInEntry> = fan_in_map
        .into_iter()
        .filter(|(_, dependents)| dependents.len() >= threshold)
        .map(|(resource, dependents)| FanInEntry {
            fan_in: dependents.len(),
            resource,
            dependents,
        })
        .collect();
    hotspots.sort_by(|a, b| b.fan_in.cmp(&a.fan_in).then(a.resource.cmp(&b.resource)));
    hotspots
}

fn print_fan_in_hotspots_json(hotspots: &[FanInEntry]) {
    let items: Vec<String> = hotspots
        .iter()
        .map(|e| {
            let deps: Vec<String> = e.dependents.iter().map(|d| format!("\"{}\"", d)).collect();
            format!(
                "{{\"resource\":\"{}\",\"fan_in\":{},\"dependents\":[{}]}}",
                e.resource,
                e.fan_in,
                deps.join(",")
            )
        })
        .collect();
    println!("{{\"fan_in_hotspots\":[{}]}}", items.join(","));
}

fn print_fan_in_hotspots_text(hotspots: &[FanInEntry]) {
    if hotspots.is_empty() {
        println!("Fan-in hotspots: (no hotspots detected)");
        return;
    }
    println!("Fan-in hotspots:");
    for e in hotspots {
        println!(
            "  {}: fan_in={} (depended on by: {})",
            e.resource,
            e.fan_in,
            e.dependents.join(", ")
        );
    }
}

/// FJ-1103: Identify fan-in hotspot resources — those depended on by many
/// other resources (fan-in > average * 2 or fan-in >= 3).
pub(crate) fn cmd_graph_resource_dependency_fan_in_hotspot(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"fan_in_hotspots\":[]}}");
        } else {
            println!("Fan-in hotspots: (no hotspots detected)");
        }
        return Ok(());
    }
    let hotspots = find_fan_in_hotspots(&config);
    if json {
        print_fan_in_hotspots_json(&hotspots);
    } else {
        print_fan_in_hotspots_text(&hotspots);
    }
    Ok(())
}

// ============================================================================
// FJ-1106: Cross-machine dependency bridge detection
// ============================================================================

struct CrossMachineBridge {
    resource: String,
    dependency: String,
    source_machine: String,
    target_machine: String,
}

/// Get the primary machine name for a resource (first entry for Multi).
fn primary_machine(resource: &types::Resource) -> String {
    match &resource.machine {
        types::MachineTarget::Single(s) => s.clone(),
        types::MachineTarget::Multiple(v) => v.first().cloned().unwrap_or_default(),
    }
}

/// Find dependency edges where source and target are on different machines.
fn find_cross_machine_bridges(config: &types::ForjarConfig) -> Vec<CrossMachineBridge> {
    let machine_map: HashMap<String, String> = config
        .resources
        .iter()
        .map(|(name, res)| (name.clone(), primary_machine(res)))
        .collect();

    let mut bridges: Vec<CrossMachineBridge> = Vec::new();
    let mut sorted_names: Vec<&String> = config.resources.keys().collect();
    sorted_names.sort();
    for name in sorted_names {
        let resource = &config.resources[name];
        let src_machine = machine_map.get(name).cloned().unwrap_or_default();
        let mut deps: Vec<&String> = resource
            .depends_on
            .iter()
            .filter(|d| config.resources.contains_key(*d))
            .collect();
        deps.sort();
        for dep in deps {
            let tgt_machine = machine_map.get(dep).cloned().unwrap_or_default();
            if src_machine != tgt_machine {
                bridges.push(CrossMachineBridge {
                    resource: name.clone(),
                    dependency: dep.clone(),
                    source_machine: src_machine.clone(),
                    target_machine: tgt_machine.clone(),
                });
            }
        }
    }
    bridges
}

fn print_cross_machine_bridges_json(bridges: &[CrossMachineBridge]) {
    let items: Vec<String> = bridges
        .iter()
        .map(|b| {
            format!(
                "{{\"resource\":\"{}\",\"dependency\":\"{}\",\"source_machine\":\"{}\",\"target_machine\":\"{}\"}}",
                b.resource, b.dependency, b.source_machine, b.target_machine
            )
        })
        .collect();
    println!("{{\"cross_machine_bridges\":[{}]}}", items.join(","));
}

fn print_cross_machine_bridges_text(bridges: &[CrossMachineBridge]) {
    if bridges.is_empty() {
        println!("Cross-machine bridges: (no cross-machine dependencies)");
        return;
    }
    println!("Cross-machine bridges:");
    for b in bridges {
        println!(
            "  {} -> {} ({} -> {})",
            b.resource, b.dependency, b.source_machine, b.target_machine
        );
    }
}

/// FJ-1106: Find dependency edges where source and target are on different
/// machines — cross-machine bridges that represent network-dependent
/// critical paths.
pub(crate) fn cmd_graph_resource_dependency_cross_machine_bridge(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"cross_machine_bridges\":[]}}");
        } else {
            println!("Cross-machine bridges: (no cross-machine dependencies)");
        }
        return Ok(());
    }
    let bridges = find_cross_machine_bridges(&config);
    if json {
        print_cross_machine_bridges_json(&bridges);
    } else {
        print_cross_machine_bridges_text(&bridges);
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

    const FAN_IN_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  base:\n    type: file\n    machine: m\n    path: /tmp/base\n    content: base\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    depends_on: [base]\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [base]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [base]\n  d:\n    type: service\n    machine: m\n    name: nginx\n    depends_on: [base]\n";

    const CROSS_MACHINE_CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m1:\n    hostname: m1\n    addr: 10.0.0.1\n  m2:\n    hostname: m2\n    addr: 10.0.0.2\nresources:\n  db:\n    type: file\n    machine: m1\n    path: /tmp/db\n    content: db\n  app:\n    type: file\n    machine: m2\n    path: /tmp/app\n    content: app\n    depends_on: [db]\n  web:\n    type: file\n    machine: m2\n    path: /tmp/web\n    content: web\n    depends_on: [app]\n";

    // ── FJ-1103: fan-in hotspot ──

    #[test]
    fn test_fj1103_fan_in_hotspot_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_fan_in_hotspot(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1103_fan_in_hotspot_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_fan_in_hotspot(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1103_fan_in_hotspot_with_data() {
        let f = write_temp_config(FAN_IN_CFG);
        assert!(cmd_graph_resource_dependency_fan_in_hotspot(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1103_fan_in_hotspot_with_data_json() {
        let f = write_temp_config(FAN_IN_CFG);
        assert!(cmd_graph_resource_dependency_fan_in_hotspot(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1103_compute_fan_in_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(FAN_IN_CFG).unwrap();
        let fan_in = compute_fan_in(&config);
        // base is depended on by a, b, c, d
        assert_eq!(fan_in["base"].len(), 4);
        // a, b, c, d have no dependents
        assert!(fan_in["a"].is_empty());
        assert!(fan_in["b"].is_empty());
    }

    #[test]
    fn test_fj1103_find_hotspots_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(FAN_IN_CFG).unwrap();
        let hotspots = find_fan_in_hotspots(&config);
        // base has fan-in=4, threshold=max(0.8*2, 3)=3, so base is a hotspot
        assert_eq!(hotspots.len(), 1);
        assert_eq!(hotspots[0].resource, "base");
        assert_eq!(hotspots[0].fan_in, 4);
    }

    #[test]
    fn test_fj1103_file_not_found() {
        let result =
            cmd_graph_resource_dependency_fan_in_hotspot(Path::new("/nonexistent"), false);
        assert!(result.is_err());
    }

    // ── FJ-1106: cross-machine bridge ──

    #[test]
    fn test_fj1106_cross_machine_bridge_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_cross_machine_bridge(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1106_cross_machine_bridge_json_empty() {
        let f = write_temp_config(EMPTY_CFG);
        assert!(cmd_graph_resource_dependency_cross_machine_bridge(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1106_cross_machine_bridge_with_data() {
        let f = write_temp_config(CROSS_MACHINE_CFG);
        assert!(cmd_graph_resource_dependency_cross_machine_bridge(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1106_cross_machine_bridge_with_data_json() {
        let f = write_temp_config(CROSS_MACHINE_CFG);
        assert!(cmd_graph_resource_dependency_cross_machine_bridge(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1106_find_bridges_helper() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(CROSS_MACHINE_CFG).unwrap();
        let bridges = find_cross_machine_bridges(&config);
        // app (m2) -> db (m1) is a cross-machine bridge
        // web (m2) -> app (m2) is NOT cross-machine
        assert_eq!(bridges.len(), 1);
        assert_eq!(bridges[0].resource, "app");
        assert_eq!(bridges[0].dependency, "db");
        assert_eq!(bridges[0].source_machine, "m2");
        assert_eq!(bridges[0].target_machine, "m1");
    }

    #[test]
    fn test_fj1106_no_cross_machine_same_machine() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(FAN_IN_CFG).unwrap();
        let bridges = find_cross_machine_bridges(&config);
        // All resources are on the same machine 'm'
        assert!(bridges.is_empty());
    }

    #[test]
    fn test_fj1106_file_not_found() {
        let result =
            cmd_graph_resource_dependency_cross_machine_bridge(Path::new("/nonexistent"), false);
        assert!(result.is_err());
    }
}
