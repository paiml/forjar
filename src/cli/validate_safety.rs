//! Safety validation — circular deps, machine refs.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::collections::{HashMap, HashSet};
use std::path::Path;


/// FJ-757: Detect circular dependency chains.
pub(crate) fn cmd_validate_check_circular_deps(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let cycles = find_cycles(&config);
    if json {
        let items: Vec<String> = cycles.iter()
            .map(|c| format!("{:?}", c))
            .collect();
        println!("{{\"circular_deps\":[{}]}}", items.join(","));
    } else if cycles.is_empty() {
        println!("No circular dependencies detected.");
    } else {
        println!("Circular dependencies ({}):", cycles.len());
        for cycle in &cycles {
            println!("  {} → {}", cycle.join(" → "), cycle[0]);
        }
    }
    Ok(())
}

/// Find cycles using DFS with coloring (white/gray/black).
fn find_cycles(config: &types::ForjarConfig) -> Vec<Vec<String>> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            adj.entry(name.as_str()).or_default().push(dep.as_str());
        }
    }
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut in_stack = HashSet::new();
    let mut path = Vec::new();
    for name in config.resources.keys() {
        if !visited.contains(name.as_str()) {
            dfs_cycle(name, &adj, &mut visited, &mut in_stack, &mut path, &mut cycles);
        }
    }
    cycles
}

fn dfs_cycle<'a>(
    node: &'a str, adj: &HashMap<&str, Vec<&'a str>>,
    visited: &mut HashSet<&'a str>, in_stack: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>, cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node);
    in_stack.insert(node);
    path.push(node);
    if let Some(neighbors) = adj.get(node) {
        for &next in neighbors {
            if !visited.contains(next) {
                dfs_cycle(next, adj, visited, in_stack, path, cycles);
            } else if in_stack.contains(next) {
                let start = path.iter().position(|&n| n == next).unwrap_or(0);
                cycles.push(path[start..].iter().map(|s| s.to_string()).collect());
            }
        }
    }
    path.pop();
    in_stack.remove(node);
}


/// FJ-761: Verify all machine references in resources exist.
pub(crate) fn cmd_validate_check_machine_refs(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let bad = find_bad_machine_refs(&config);
    if json {
        let items: Vec<String> = bad.iter()
            .map(|(r, m)| format!("{{\"resource\":\"{}\",\"machine\":\"{}\"}}", r, m))
            .collect();
        println!("{{\"bad_machine_refs\":[{}]}}", items.join(","));
    } else if bad.is_empty() {
        println!("All machine references are valid.");
    } else {
        println!("Invalid machine references ({}):", bad.len());
        for (resource, machine) in &bad {
            println!("  {} → {} (not defined)", resource, machine);
        }
    }
    Ok(())
}

/// Find resources referencing machines that don't exist in config.
fn find_bad_machine_refs(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let machines: HashSet<&str> = config.machines.keys().map(|k| k.as_str()).collect();
    let mut bad = Vec::new();
    for (name, resource) in &config.resources {
        for m in resource.machine.to_vec() {
            if m != "localhost" && !machines.contains(m.as_str()) {
                bad.push((name.clone(), m));
            }
        }
    }
    bad.sort();
    bad
}


/// FJ-765: Verify consistent package providers per machine.
pub(crate) fn cmd_validate_check_provider_consistency(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let conflicts = find_provider_conflicts(&config);
    if json {
        let items: Vec<String> = conflicts.iter()
            .map(|(m, ps)| format!("{{\"machine\":\"{}\",\"providers\":{:?}}}", m, ps))
            .collect();
        println!("{{\"provider_conflicts\":[{}]}}", items.join(","));
    } else if conflicts.is_empty() {
        println!("All machines use consistent package providers.");
    } else {
        println!("Provider inconsistencies ({}):", conflicts.len());
        for (m, providers) in &conflicts {
            println!("  {} — mixed providers: {}", m, providers.join(", "));
        }
    }
    Ok(())
}

/// Find machines where package resources use multiple providers.
fn find_provider_conflicts(config: &types::ForjarConfig) -> Vec<(String, Vec<String>)> {
    let mut machine_providers: HashMap<String, HashSet<String>> = HashMap::new();
    for resource in config.resources.values() {
        if resource.resource_type != types::ResourceType::Package { continue; }
        if let Some(ref p) = resource.provider {
            for m in resource.machine.to_vec() {
                machine_providers.entry(m).or_default().insert(p.clone());
            }
        }
    }
    let mut conflicts: Vec<(String, Vec<String>)> = machine_providers.into_iter()
        .filter(|(_, ps)| ps.len() > 1)
        .map(|(m, ps)| { let mut v: Vec<String> = ps.into_iter().collect(); v.sort(); (m, v) })
        .collect();
    conflicts.sort_by(|a, b| a.0.cmp(&b.0));
    conflicts
}


/// FJ-769: Verify state field values are valid for each resource type.
pub(crate) fn cmd_validate_check_state_values(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let bad = find_bad_states(&config);
    if json {
        let items: Vec<String> = bad.iter()
            .map(|(r, t, s)| format!("{{\"resource\":\"{}\",\"type\":\"{}\",\"state\":\"{}\"}}", r, t, s))
            .collect();
        println!("{{\"invalid_states\":[{}]}}", items.join(","));
    } else if bad.is_empty() {
        println!("All resource state values are valid.");
    } else {
        println!("Invalid state values ({}):", bad.len());
        for (r, t, s) in &bad { println!("  {} (type {}) — invalid state \"{}\"", r, t, s); }
    }
    Ok(())
}

/// Check that each resource's state field is valid for its type.
fn find_bad_states(config: &types::ForjarConfig) -> Vec<(String, String, String)> {
    let mut bad = Vec::new();
    for (name, resource) in &config.resources {
        if let Some(ref s) = resource.state {
            let valid = match resource.resource_type {
                types::ResourceType::File => ["file", "directory", "symlink", "absent"].contains(&s.as_str()),
                types::ResourceType::Service => ["running", "stopped", "enabled", "disabled"].contains(&s.as_str()),
                types::ResourceType::Mount => ["mounted", "unmounted", "absent"].contains(&s.as_str()),
                types::ResourceType::Docker => ["running", "stopped", "absent"].contains(&s.as_str()),
                types::ResourceType::Package => ["present", "absent"].contains(&s.as_str()),
                _ => true,
            };
            if !valid {
                bad.push((name.clone(), resource.resource_type.to_string(), s.clone()));
            }
        }
    }
    bad.sort_by(|a, b| a.0.cmp(&b.0));
    bad
}


/// FJ-773: Detect machines defined but not referenced by any resource.
pub(crate) fn cmd_validate_check_unused_machines(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let unused = find_unused_machines(&config);
    if json {
        let items: Vec<String> = unused.iter().map(|m| format!("\"{}\"", m)).collect();
        println!("{{\"unused_machines\":[{}]}}", items.join(","));
    } else if unused.is_empty() {
        println!("All defined machines are referenced by resources.");
    } else {
        println!("Unused machines ({}):", unused.len());
        for m in &unused { println!("  {}", m); }
    }
    Ok(())
}

/// Find machines not referenced by any resource.
fn find_unused_machines(config: &types::ForjarConfig) -> Vec<String> {
    let mut used: HashSet<String> = HashSet::new();
    for resource in config.resources.values() {
        for m in resource.machine.to_vec() { used.insert(m); }
    }
    let mut unused: Vec<String> = config.machines.keys()
        .filter(|m| !used.contains(m.as_str()))
        .cloned().collect();
    unused.sort();
    unused
}


/// FJ-777: Verify resource tags follow naming conventions (kebab-case).
pub(crate) fn cmd_validate_check_tag_consistency(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let bad = find_bad_tags(&config);
    if json {
        let items: Vec<String> = bad.iter()
            .map(|(r, t)| format!("{{\"resource\":\"{}\",\"tag\":\"{}\"}}", r, t))
            .collect();
        println!("{{\"tag_violations\":[{}]}}", items.join(","));
    } else if bad.is_empty() {
        println!("All resource tags follow naming conventions.");
    } else {
        println!("Tag naming violations ({}):", bad.len());
        for (r, t) in &bad { println!("  {} — tag \"{}\" (expected kebab-case)", r, t); }
    }
    Ok(())
}

/// FJ-781: Verify all depends_on targets reference existing resources.
pub(crate) fn cmd_validate_check_dependency_exists(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let bad = find_missing_deps(&config);
    if json {
        let items: Vec<String> = bad.iter()
            .map(|(r, d)| format!("{{\"resource\":\"{}\",\"missing_dep\":\"{}\"}}", r, d))
            .collect();
        println!("{{\"missing_dependencies\":[{}]}}", items.join(","));
    } else if bad.is_empty() {
        println!("All dependency references are valid.");
    } else {
        println!("Missing dependency targets ({}):", bad.len());
        for (r, d) in &bad { println!("  {} → {} (not defined)", r, d); }
    }
    Ok(())
}

/// Find resources referencing dependencies that don't exist.
fn find_missing_deps(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let names: HashSet<&str> = config.resources.keys().map(|k| k.as_str()).collect();
    let mut bad = Vec::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !names.contains(dep.as_str()) {
                bad.push((name.clone(), dep.clone()));
            }
        }
    }
    bad.sort();
    bad
}


/// FJ-785: Detect resources targeting the same file path on the same machine.
pub(crate) fn cmd_validate_check_path_conflicts_strict(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let conflicts = find_strict_path_conflicts(&config);
    if json {
        let items: Vec<String> = conflicts.iter()
            .map(|(m, p, rs)| format!("{{\"machine\":\"{}\",\"path\":\"{}\",\"resources\":{:?}}}", m, p, rs))
            .collect();
        println!("{{\"path_conflicts\":[{}]}}", items.join(","));
    } else if conflicts.is_empty() {
        println!("No file path conflicts detected.");
    } else {
        println!("File path conflicts ({}):", conflicts.len());
        for (m, p, rs) in &conflicts {
            println!("  {} on {} — resources: {}", p, m, rs.join(", "));
        }
    }
    Ok(())
}

/// Find resources that share the same path on the same machine.
fn find_strict_path_conflicts(config: &types::ForjarConfig) -> Vec<(String, String, Vec<String>)> {
    let mut path_map: HashMap<(String, String), Vec<String>> = HashMap::new();
    for (name, resource) in &config.resources {
        if let Some(ref p) = resource.path {
            for m in resource.machine.to_vec() {
                path_map.entry((m, p.clone())).or_default().push(name.clone());
            }
        }
    }
    let mut conflicts: Vec<(String, String, Vec<String>)> = path_map.into_iter()
        .filter(|(_, rs)| rs.len() > 1)
        .map(|((m, p), mut rs)| { rs.sort(); (m, p, rs) })
        .collect();
    conflicts.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    conflicts
}


/// Check if a string is kebab-case.
fn is_kebab_case(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Find resources with tags that aren't kebab-case.
fn find_bad_tags(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut bad = Vec::new();
    for (name, resource) in &config.resources {
        for tag in &resource.tags {
            if !is_kebab_case(tag) {
                bad.push((name.clone(), tag.clone()));
            }
        }
    }
    bad.sort();
    bad
}
