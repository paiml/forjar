//! Advanced validation — orphan resources, machine arch, health conflicts, overlaps.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::collections::HashSet;
use std::path::Path;

/// FJ-797: Detect resources not referenced by any depends_on chain.
pub(crate) fn cmd_validate_check_orphan_resources(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let orphans = find_orphan_resources(&config);
    if json {
        let items: Vec<String> = orphans.iter().map(|o| format!("\"{}\"", o)).collect();
        println!("{{\"orphan_resources\":[{}]}}", items.join(","));
    } else if orphans.is_empty() {
        println!("No orphan resources (all participate in dependency chains).");
    } else {
        println!("Orphan resources ({}, not depended on and have no deps):", orphans.len());
        for o in &orphans { println!("  {}", o); }
    }
    Ok(())
}

/// Find resources that neither depend on anything nor are depended upon.
fn find_orphan_resources(config: &types::ForjarConfig) -> Vec<String> {
    let mut depended_on: HashSet<&str> = HashSet::new();
    let mut has_deps: HashSet<&str> = HashSet::new();
    for (name, resource) in &config.resources {
        if !resource.depends_on.is_empty() {
            has_deps.insert(name.as_str());
            for dep in &resource.depends_on {
                depended_on.insert(dep.as_str());
            }
        }
    }
    let mut orphans: Vec<String> = config.resources.keys()
        .filter(|n| !has_deps.contains(n.as_str()) && !depended_on.contains(n.as_str()))
        .cloned().collect();
    orphans.sort();
    orphans
}

/// FJ-801: Validate machine architecture fields are consistent.
pub(crate) fn cmd_validate_check_machine_arch(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let valid_archs = ["x86_64", "aarch64", "arm64", "armv7", "riscv64", "ppc64le", "s390x"];
    let mut bad: Vec<(String, String)> = Vec::new();
    for (name, machine) in &config.machines {
        let arch = machine.arch.as_str();
        if !valid_archs.contains(&arch) {
            bad.push((name.clone(), arch.to_string()));
        }
    }
    bad.sort();
    if json {
        let items: Vec<String> = bad.iter()
            .map(|(m, a)| format!("{{\"machine\":\"{}\",\"arch\":\"{}\"}}", m, a))
            .collect();
        println!("{{\"invalid_architectures\":[{}]}}", items.join(","));
    } else if bad.is_empty() {
        println!("All machine architectures are valid.");
    } else {
        println!("Invalid architectures ({}):", bad.len());
        for (m, a) in &bad { println!("  {} — \"{}\" (expected: {})", m, a, valid_archs.join(", ")); }
    }
    Ok(())
}

/// FJ-805: Detect resources with conflicting health indicators.
pub(crate) fn cmd_validate_check_resource_health_conflicts(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let conflicts = find_health_conflicts(&config);
    if json {
        let items: Vec<String> = conflicts.iter()
            .map(|(r, reason)| format!("{{\"resource\":\"{}\",\"conflict\":\"{}\"}}", r, reason))
            .collect();
        println!("{{\"health_conflicts\":[{}]}}", items.join(","));
    } else if conflicts.is_empty() {
        println!("No resource health conflicts detected.");
    } else {
        println!("Resource health conflicts ({}):", conflicts.len());
        for (r, reason) in &conflicts { println!("  {} — {}", r, reason); }
    }
    Ok(())
}

fn find_health_conflicts(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut conflicts = Vec::new();
    for (name, resource) in &config.resources {
        let rtype = format!("{:?}", resource.resource_type);
        let has_service_state = resource.state.as_deref() == Some("running")
            || resource.state.as_deref() == Some("stopped");
        let is_service = rtype.contains("Service");
        if has_service_state && !is_service {
            conflicts.push((name.clone(), format!("{} has service state but is type {}", name, rtype)));
        }
        if is_service && resource.state.as_deref() == Some("absent") {
            conflicts.push((name.clone(), "service with state=absent is contradictory".to_string()));
        }
    }
    conflicts.sort();
    conflicts
}

/// FJ-809: Detect resources with overlapping scope on same machine.
pub(crate) fn cmd_validate_check_resource_overlap(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let overlaps = find_resource_overlaps(&config);
    if json {
        let items: Vec<String> = overlaps.iter()
            .map(|(a, b, m)| format!("{{\"resource_a\":\"{}\",\"resource_b\":\"{}\",\"machine\":\"{}\"}}", a, b, m))
            .collect();
        println!("{{\"resource_overlaps\":[{}]}}", items.join(","));
    } else if overlaps.is_empty() {
        println!("No overlapping resources detected.");
    } else {
        println!("Overlapping resources ({}):", overlaps.len());
        for (a, b, m) in &overlaps { println!("  {} <-> {} on {}", a, b, m); }
    }
    Ok(())
}

fn find_resource_overlaps(config: &types::ForjarConfig) -> Vec<(String, String, String)> {
    let mut overlaps = Vec::new();
    let names: Vec<&String> = config.resources.keys().collect();
    for i in 0..names.len() {
        for j in (i + 1)..names.len() {
            let ra = &config.resources[names[i]];
            let rb = &config.resources[names[j]];
            let ma = ra.machine.to_vec();
            let mb = rb.machine.to_vec();
            let shared: Vec<&String> = ma.iter().filter(|m| mb.contains(m)).collect();
            if shared.is_empty() { continue; }
            let same_type = std::mem::discriminant(&ra.resource_type) == std::mem::discriminant(&rb.resource_type);
            let same_path = ra.path.is_some() && ra.path == rb.path;
            if same_type && same_path {
                for m in shared {
                    overlaps.push((names[i].clone(), names[j].clone(), m.clone()));
                }
            }
        }
    }
    overlaps
}
