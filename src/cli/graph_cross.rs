//! Cross-machine analysis.

use super::helpers::*;
use std::path::Path;

/// Build a map from resource name to machine string.
fn build_resource_machine_map(
    config: &crate::core::types::ForjarConfig,
) -> std::collections::HashMap<String, String> {
    let mut resource_machine: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (name, resource) in &config.resources {
        let machine = resource.machine.to_string();
        resource_machine.insert(name.clone(), machine);
    }
    resource_machine
}

/// Find all cross-machine dependencies.
fn find_cross_machine_deps(
    config: &crate::core::types::ForjarConfig,
    resource_machine: &std::collections::HashMap<String, String>,
) -> Vec<(String, String, String, String)> {
    let mut cross_deps = Vec::new();
    for (name, resource) in &config.resources {
        let src_machine = resource_machine.get(name).cloned().unwrap_or_default();
        for dep in &resource.depends_on {
            let dep_machine = resource_machine.get(dep).cloned().unwrap_or_default();
            if src_machine != dep_machine {
                cross_deps.push((name.clone(), src_machine.clone(), dep.clone(), dep_machine));
            }
        }
    }
    cross_deps
}

/// Print cross-machine deps as JSON.
fn print_cross_deps_json(cross_deps: &[(String, String, String, String)]) {
    print!("{{\"cross_machine_deps\":[");
    for (i, (src, src_m, dep, dep_m)) in cross_deps.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!(
            r#"{{"resource":"{src}","machine":"{src_m}","depends_on":"{dep}","dep_machine":"{dep_m}"}}"#
        );
    }
    println!("]}}");
}

/// Print cross-machine deps as text.
fn print_cross_deps_text(cross_deps: &[(String, String, String, String)]) {
    if cross_deps.is_empty() {
        println!("No cross-machine dependencies found");
    } else {
        println!("Cross-machine dependencies ({}):", cross_deps.len());
        for (src, src_m, dep, dep_m) in cross_deps {
            println!("  {src} ({src_m}) -> {dep} ({dep_m})");
        }
    }
}

/// Print machine groups as JSON.
fn print_machine_groups_json(groups: &std::collections::BTreeMap<String, Vec<String>>) {
    print!("{{\"groups\":[");
    for (i, (machine, resources)) in groups.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        let res_json: Vec<_> = resources.iter().map(|r| format!(r#""{r}""#)).collect();
        print!(
            r#"{{"machine":"{}","resources":[{}]}}"#,
            machine,
            res_json.join(",")
        );
    }
    println!("]}}");
}

/// Print machine groups as text.
fn print_machine_groups_text(groups: &std::collections::BTreeMap<String, Vec<String>>) {
    for (machine, resources) in groups {
        println!("Machine: {} ({} resources)", machine, resources.len());
        for r in resources {
            println!("  - {r}");
        }
    }
}

/// Classify resource type into a security boundary string.
fn classify_security_boundary(resource: &crate::core::types::Resource) -> Option<String> {
    match resource.resource_type {
        crate::core::types::ResourceType::Network => Some("network".to_string()),
        crate::core::types::ResourceType::User => Some("identity".to_string()),
        crate::core::types::ResourceType::Service => Some("process".to_string()),
        crate::core::types::ResourceType::Mount => Some("filesystem".to_string()),
        crate::core::types::ResourceType::File => {
            if let Some(ref path) = resource.path {
                if path.starts_with("/etc") {
                    Some("system-config".to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// BFS to find direct and indirect impacts from a resource.
fn compute_change_impact(
    resource: &str,
    dependents: &std::collections::HashMap<String, Vec<String>>,
) -> (Vec<String>, Vec<String>) {
    let mut direct: Vec<String> = Vec::new();
    let mut indirect: Vec<String> = Vec::new();
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut queue: std::collections::VecDeque<(String, usize)> = std::collections::VecDeque::new();

    visited.insert(resource.to_string());
    if let Some(deps) = dependents.get(resource) {
        for dep in deps {
            if visited.insert(dep.clone()) {
                queue.push_back((dep.clone(), 1));
                direct.push(dep.clone());
            }
        }
    }

    while let Some((current, depth)) = queue.pop_front() {
        if let Some(deps) = dependents.get(&current) {
            for dep in deps {
                if visited.insert(dep.clone()) {
                    queue.push_back((dep.clone(), depth + 1));
                    indirect.push(dep.clone());
                }
            }
        }
    }

    direct.sort();
    indirect.sort();
    (direct, indirect)
}

/// FJ-664: Visualize dependencies across machines
pub(crate) fn cmd_graph_cross_machine_deps(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;

    let resource_machine = build_resource_machine_map(&config);
    let cross_deps = find_cross_machine_deps(&config, &resource_machine);

    if json {
        print_cross_deps_json(&cross_deps);
    } else {
        print_cross_deps_text(&cross_deps);
    }
    Ok(())
}

/// FJ-674: Group resources by machine in graph output
pub(crate) fn cmd_graph_machine_groups(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;

    let mut groups: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for (name, resource) in &config.resources {
        let machine = resource.machine.to_string();
        groups.entry(machine).or_default().push(name.clone());
    }
    // Sort resources within each group
    for resources in groups.values_mut() {
        resources.sort();
    }

    if json {
        print_machine_groups_json(&groups);
    } else {
        print_machine_groups_text(&groups);
    }
    Ok(())
}

/// FJ-564: Show direct + indirect impact of changing a resource.
pub(crate) fn cmd_graph_change_impact(
    file: &Path,
    resource: &str,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    if !config.resources.contains_key(resource) {
        return Err(format!("Resource '{resource}' not found in config"));
    }

    // Build forward dependency map: what depends on each resource?
    let mut dependents: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            dependents
                .entry(dep.clone())
                .or_default()
                .push(name.clone());
        }
    }

    let (direct, indirect) = compute_change_impact(resource, &dependents);

    if json {
        let d: Vec<String> = direct.iter().map(|d| format!(r#""{d}""#)).collect();
        let i: Vec<String> = indirect.iter().map(|i| format!(r#""{i}""#)).collect();
        println!(
            r#"{{"resource":"{}","direct":[{}],"indirect":[{}],"total_impact":{}}}"#,
            resource,
            d.join(","),
            i.join(","),
            direct.len() + indirect.len()
        );
    } else {
        println!("Change impact for '{resource}':");
        if direct.is_empty() && indirect.is_empty() {
            println!("  No downstream dependencies");
        } else {
            if !direct.is_empty() {
                println!("  Direct ({}):", direct.len());
                for d in &direct {
                    println!("    → {d}");
                }
            }
            if !indirect.is_empty() {
                println!("  Indirect ({}):", indirect.len());
                for i in &indirect {
                    println!("    →→ {i}");
                }
            }
        }
    }
    Ok(())
}

/// FJ-604: Highlight resources crossing security boundaries.
pub(crate) fn cmd_graph_security_boundaries(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut boundaries: Vec<(String, String, String)> = Vec::new(); // (resource, type, boundary)

    for (rname, resource) in &config.resources {
        if let Some(boundary) = classify_security_boundary(resource) {
            let rtype = format!("{:?}", resource.resource_type);
            boundaries.push((rname.clone(), rtype, boundary));
        }
    }

    if json {
        let items: Vec<String> = boundaries
            .iter()
            .map(|(r, t, b)| format!(r#"{{"resource":"{r}","type":"{t}","boundary":"{b}"}}"#))
            .collect();
        println!(
            r#"{{"security_boundaries":[{}],"count":{}}}"#,
            items.join(","),
            boundaries.len()
        );
    } else if boundaries.is_empty() {
        println!("No resources cross security boundaries");
    } else {
        println!("Security boundaries ({} resources):", boundaries.len());
        for (r, t, b) in &boundaries {
            println!("  {r} ({t}) — {b} boundary");
        }
    }
    Ok(())
}

/// FJ-714: Show reverse dependency graph
pub(crate) fn cmd_graph_reverse_deps(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut rev: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for name in cfg.resources.keys() {
        rev.entry(name.clone()).or_default();
    }
    for (name, resource) in &cfg.resources {
        for dep in &resource.depends_on {
            rev.entry(dep.clone()).or_default().push(name.clone());
        }
    }
    let mut sorted: Vec<_> = rev.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    if json {
        let entries: Vec<String> = sorted
            .iter()
            .map(|(name, deps)| {
                let items: Vec<String> = deps.iter().map(|d| format!("\"{d}\"")).collect();
                format!(
                    "{{\"resource\":\"{}\",\"depended_by\":[{}]}}",
                    name,
                    items.join(",")
                )
            })
            .collect();
        println!("{{\"reverse_deps\":[{}]}}", entries.join(","));
    } else {
        println!("Reverse dependencies (who depends on me):");
        for (name, deps) in &sorted {
            if deps.is_empty() {
                println!("  {name} — (none)");
            } else {
                println!("  {} <- {}", name, deps.join(", "));
            }
        }
    }
    Ok(())
}

/// FJ-704: Show leaf resources (no dependents in the DAG)
pub(crate) fn cmd_graph_leaf_resources(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut has_dependents: std::collections::HashSet<String> = std::collections::HashSet::new();
    for resource in cfg.resources.values() {
        for dep in &resource.depends_on {
            has_dependents.insert(dep.clone());
        }
    }
    let mut leaves: Vec<String> = cfg
        .resources
        .keys()
        .filter(|name| !has_dependents.contains(*name))
        .cloned()
        .collect();
    leaves.sort();
    if json {
        let items: Vec<String> = leaves.iter().map(|n| format!("\"{n}\"")).collect();
        println!(
            "{{\"leaf_resources\":[{}],\"count\":{}}}",
            items.join(","),
            leaves.len()
        );
    } else {
        println!("Leaf resources ({} — no dependents):", leaves.len());
        for name in &leaves {
            let rtype = cfg
                .resources
                .get(name)
                .map(|r| format!("{:?}", r.resource_type))
                .unwrap_or_default();
            println!("  {name} ({rtype})");
        }
    }
    Ok(())
}
