//! Phase 97 — State Analytics & Capacity Planning: graph commands.

use crate::core::types;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::Path;

// ============================================================================
// FJ-1039 helpers
// ============================================================================

/// Build in-degree map and forward adjacency list from resource dependencies.
fn build_kahn_graph(
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

/// Run Kahn's algorithm and return levels of parallel-applicable resources.
fn kahn_levels(
    mut in_degree: HashMap<String, usize>,
    children: &HashMap<String, Vec<String>>,
) -> Vec<Vec<String>> {
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    let mut levels: Vec<Vec<String>> = Vec::new();
    while !queue.is_empty() {
        let mut level: Vec<String> = queue.drain(..).collect();
        level.sort();
        let mut next_queue: VecDeque<String> = VecDeque::new();
        for node in &level {
            if let Some(deps) = children.get(node) {
                for dep in deps {
                    let entry = in_degree.get_mut(dep).expect("node in graph");
                    *entry -= 1;
                    if *entry == 0 {
                        next_queue.push_back(dep.clone());
                    }
                }
            }
        }
        levels.push(level);
        queue = next_queue;
    }
    levels
}

fn print_apply_order_json(levels: &[Vec<String>]) {
    let level_entries: Vec<String> = levels
        .iter()
        .enumerate()
        .map(|(i, members)| {
            let names: Vec<String> = members.iter().map(|n| format!("\"{n}\"")).collect();
            format!(
                "{{\"level\":{},\"parallel_count\":{},\"resources\":[{}]}}",
                i,
                members.len(),
                names.join(",")
            )
        })
        .collect();
    println!(
        "{{\"apply_order_simulation\":{{\"total_levels\":{},\"levels\":[{}]}}}}",
        levels.len(),
        level_entries.join(",")
    );
}

fn print_apply_order_text(levels: &[Vec<String>]) {
    println!("Apply order simulation ({} levels):", levels.len());
    for (i, members) in levels.iter().enumerate() {
        println!(
            "  Level {} ({} parallel): {}",
            i,
            members.len(),
            members.join(", ")
        );
    }
}

// ============================================================================
// FJ-1042 helpers
// ============================================================================

/// Key for grouping resources by (type, machine).
type GroupKey = (String, String);

/// Per-group summary: resource names and their dependency depths.
struct GroupSummary {
    count: usize,
    total_depth: usize,
}

fn build_depth_graph(
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

fn bfs_depths(
    in_degree: &mut HashMap<String, usize>,
    children: &HashMap<String, Vec<String>>,
) -> HashMap<String, usize> {
    let mut depths: HashMap<String, usize> = HashMap::new();
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(n, _)| n.clone())
        .collect();
    for root in &queue {
        depths.insert(root.clone(), 0);
    }
    while let Some(node) = queue.pop_front() {
        let current_depth = depths[&node];
        for dep in children.get(&node).cloned().unwrap_or_default() {
            let entry = depths.entry(dep.clone()).or_insert(0);
            if current_depth + 1 > *entry {
                *entry = current_depth + 1;
            }
            let deg = in_degree.get_mut(&dep).expect("node in graph");
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(dep);
            }
        }
    }
    depths
}

/// Compute the dependency depth for each resource via BFS from roots.
fn compute_dependency_depths(config: &types::ForjarConfig) -> HashMap<String, usize> {
    let (mut in_degree, children) = build_depth_graph(config);
    let mut depths = bfs_depths(&mut in_degree, &children);
    for name in config.resources.keys() {
        depths.entry(name.clone()).or_insert(0);
    }
    depths
}

/// Extract the machine name(s) from a resource's machine target.
fn machine_names(resource: &types::Resource) -> Vec<String> {
    resource.machine.to_vec()
}

/// Group resources by (type, machine) and compute summary stats.
fn build_provenance_groups(
    config: &types::ForjarConfig,
    depths: &HashMap<String, usize>,
) -> BTreeMap<GroupKey, GroupSummary> {
    let mut groups: BTreeMap<GroupKey, GroupSummary> = BTreeMap::new();
    for (name, resource) in &config.resources {
        let rtype = resource.resource_type.to_string();
        let depth = depths.get(name).copied().unwrap_or(0);
        for machine in machine_names(resource) {
            let key = (rtype.clone(), machine);
            let entry = groups.entry(key).or_insert(GroupSummary {
                count: 0,
                total_depth: 0,
            });
            entry.count += 1;
            entry.total_depth += depth;
        }
    }
    groups
}

fn print_provenance_json(groups: &BTreeMap<GroupKey, GroupSummary>) {
    let entries: Vec<String> = groups
        .iter()
        .map(|((rtype, machine), summary)| {
            let avg = if summary.count > 0 {
                summary.total_depth as f64 / summary.count as f64
            } else {
                0.0
            };
            format!(
                "{{\"type\":\"{}\",\"machine\":\"{}\",\"count\":{},\"avg_dependency_depth\":{:.2}}}",
                rtype, machine, summary.count, avg
            )
        })
        .collect();
    println!(
        "{{\"provenance_summary\":{{\"group_count\":{},\"groups\":[{}]}}}}",
        groups.len(),
        entries.join(",")
    );
}

fn print_provenance_text(groups: &BTreeMap<GroupKey, GroupSummary>) {
    println!("Resource provenance summary ({} groups):", groups.len());
    for ((rtype, machine), summary) in groups {
        let avg = if summary.count > 0 {
            summary.total_depth as f64 / summary.count as f64
        } else {
            0.0
        };
        println!(
            "  [{}] on {}: {} resources, avg depth {:.2}",
            rtype, machine, summary.count, avg
        );
    }
}

// ============================================================================
// Public commands
// ============================================================================

/// FJ-1039: Simulate a valid apply order using topological sort (Kahn's algorithm).
///
/// For each level, lists the resources that can be applied in parallel.
pub(crate) fn cmd_graph_resource_apply_order_simulation(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"apply_order_simulation\":{{\"total_levels\":0,\"levels\":[]}}}}");
        } else {
            println!("No resources to simulate.");
        }
        return Ok(());
    }
    let (in_degree, children) = build_kahn_graph(&config);
    let levels = kahn_levels(in_degree, &children);
    if json {
        print_apply_order_json(&levels);
    } else {
        print_apply_order_text(&levels);
    }
    Ok(())
}

/// FJ-1042: Provenance summary — group resources by type and machine,
/// reporting count and average dependency depth per group.
pub(crate) fn cmd_graph_resource_provenance_summary(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    if config.resources.is_empty() {
        if json {
            println!("{{\"provenance_summary\":{{\"group_count\":0,\"groups\":[]}}}}");
        } else {
            println!("No resources to summarize.");
        }
        return Ok(());
    }
    let depths = compute_dependency_depths(&config);
    let groups = build_provenance_groups(&config, &depths);
    if json {
        print_provenance_json(&groups);
    } else {
        print_provenance_text(&groups);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_config(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    const EMPTY_CONFIG: &str = r#"
version: "1.0"
name: test
resources: {}
"#;

    const SIMPLE_CONFIG: &str = r#"
version: "1.0"
name: test
resources:
  pkg-a:
    type: package
    machine: web
    packages: [curl]
  cfg-a:
    type: file
    machine: web
    path: /etc/app.conf
    depends_on: [pkg-a]
  svc-a:
    type: service
    machine: web
    name: app
    depends_on: [cfg-a]
"#;

    const MULTI_MACHINE_CONFIG: &str = r#"
version: "1.0"
name: test
resources:
  pkg-web:
    type: package
    machine: web
    packages: [nginx]
  pkg-db:
    type: package
    machine: db
    packages: [postgres]
  cfg-web:
    type: file
    machine: web
    path: /etc/nginx.conf
    depends_on: [pkg-web]
  svc-web:
    type: service
    machine: web
    name: nginx
    depends_on: [cfg-web]
  svc-db:
    type: service
    machine: db
    name: postgres
    depends_on: [pkg-db]
"#;

    // -- FJ-1039 tests --

    #[test]
    fn test_apply_order_empty() {
        let f = write_config(EMPTY_CONFIG);
        let result = cmd_graph_resource_apply_order_simulation(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_order_empty_json() {
        let f = write_config(EMPTY_CONFIG);
        let result = cmd_graph_resource_apply_order_simulation(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_order_linear_chain() {
        let f = write_config(SIMPLE_CONFIG);
        let result = cmd_graph_resource_apply_order_simulation(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_order_linear_chain_json() {
        let f = write_config(SIMPLE_CONFIG);
        let result = cmd_graph_resource_apply_order_simulation(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_order_parallel_roots() {
        let f = write_config(MULTI_MACHINE_CONFIG);
        let result = cmd_graph_resource_apply_order_simulation(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kahn_levels_linear() {
        let f = write_config(SIMPLE_CONFIG);
        let content = std::fs::read_to_string(f.path()).unwrap();
        let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
        let (in_degree, children) = build_kahn_graph(&config);
        let levels = kahn_levels(in_degree, &children);
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["pkg-a"]);
        assert_eq!(levels[1], vec!["cfg-a"]);
        assert_eq!(levels[2], vec!["svc-a"]);
    }

    #[test]
    fn test_kahn_levels_parallel() {
        let f = write_config(MULTI_MACHINE_CONFIG);
        let content = std::fs::read_to_string(f.path()).unwrap();
        let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
        let (in_degree, children) = build_kahn_graph(&config);
        let levels = kahn_levels(in_degree, &children);
        // Level 0: pkg-db, pkg-web (both roots, sorted alphabetically)
        assert_eq!(levels[0], vec!["pkg-db", "pkg-web"]);
    }

    // -- FJ-1042 tests --

    #[test]
    fn test_provenance_empty() {
        let f = write_config(EMPTY_CONFIG);
        let result = cmd_graph_resource_provenance_summary(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provenance_empty_json() {
        let f = write_config(EMPTY_CONFIG);
        let result = cmd_graph_resource_provenance_summary(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provenance_simple() {
        let f = write_config(SIMPLE_CONFIG);
        let result = cmd_graph_resource_provenance_summary(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provenance_simple_json() {
        let f = write_config(SIMPLE_CONFIG);
        let result = cmd_graph_resource_provenance_summary(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provenance_groups() {
        let f = write_config(MULTI_MACHINE_CONFIG);
        let content = std::fs::read_to_string(f.path()).unwrap();
        let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
        let depths = compute_dependency_depths(&config);
        let groups = build_provenance_groups(&config, &depths);
        // Should have groups: (file, web), (package, db), (package, web), (service, db), (service, web)
        assert_eq!(groups.len(), 5);
        let pkg_web = groups
            .get(&("package".to_string(), "web".to_string()))
            .unwrap();
        assert_eq!(pkg_web.count, 1);
        assert_eq!(pkg_web.total_depth, 0); // root node
        let svc_web = groups
            .get(&("service".to_string(), "web".to_string()))
            .unwrap();
        assert_eq!(svc_web.count, 1);
        assert_eq!(svc_web.total_depth, 2); // depth 2 (pkg-web -> cfg-web -> svc-web)
    }

    #[test]
    fn test_dependency_depths() {
        let f = write_config(SIMPLE_CONFIG);
        let content = std::fs::read_to_string(f.path()).unwrap();
        let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
        let depths = compute_dependency_depths(&config);
        assert_eq!(depths["pkg-a"], 0);
        assert_eq!(depths["cfg-a"], 1);
        assert_eq!(depths["svc-a"], 2);
    }

    #[test]
    fn test_file_not_found() {
        let result = cmd_graph_resource_apply_order_simulation(Path::new("/nonexistent"), false);
        assert!(result.is_err());
        let result = cmd_graph_resource_provenance_summary(Path::new("/nonexistent"), false);
        assert!(result.is_err());
    }
}
