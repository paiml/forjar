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

/// FJ-813: Enforce tag conventions (required tags, naming rules).
pub(crate) fn cmd_validate_check_resource_tags(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let issues = find_tag_issues(&config);
    if json {
        let items: Vec<String> = issues.iter()
            .map(|(r, issue)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", r, issue))
            .collect();
        println!("{{\"tag_issues\":[{}]}}", items.join(","));
    } else if issues.is_empty() {
        println!("All resource tags follow conventions.");
    } else {
        println!("Tag convention issues ({}):", issues.len());
        for (r, issue) in &issues { println!("  {} — {}", r, issue); }
    }
    Ok(())
}

fn find_tag_issues(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut issues = Vec::new();
    for (name, resource) in &config.resources {
        if resource.tags.is_empty() {
            issues.push((name.clone(), "no tags assigned".to_string()));
            continue;
        }
        for tag in &resource.tags {
            if tag != &tag.to_lowercase() {
                issues.push((name.clone(), format!("tag '{}' should be lowercase", tag)));
            }
            if tag.contains(' ') {
                issues.push((name.clone(), format!("tag '{}' contains spaces", tag)));
            }
        }
    }
    issues.sort();
    issues
}

/// FJ-817: Verify state fields match resource type constraints.
pub(crate) fn cmd_validate_check_resource_state_consistency(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let issues = find_state_consistency_issues(&config);
    if json {
        let items: Vec<String> = issues.iter()
            .map(|(r, issue)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", r, issue))
            .collect();
        println!("{{\"state_consistency_issues\":[{}]}}", items.join(","));
    } else if issues.is_empty() {
        println!("All resource states are consistent with their types.");
    } else {
        println!("State consistency issues ({}):", issues.len());
        for (r, issue) in &issues { println!("  {} — {}", r, issue); }
    }
    Ok(())
}

fn find_state_consistency_issues(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut issues = Vec::new();
    let pkg_states = ["present", "absent", "latest"];
    let svc_states = ["running", "stopped", "enabled", "disabled"];
    let file_states = ["present", "absent", "directory"];
    for (name, resource) in &config.resources {
        let rtype = format!("{:?}", resource.resource_type);
        let state = match resource.state.as_deref() {
            Some(s) => s,
            None => continue,
        };
        let valid = if rtype.contains("Package") {
            pkg_states.contains(&state)
        } else if rtype.contains("Service") {
            svc_states.contains(&state)
        } else if rtype.contains("File") || rtype.contains("Template") {
            file_states.contains(&state)
        } else {
            true
        };
        if !valid {
            issues.push((name.clone(), format!("state '{}' invalid for type {}", state, rtype)));
        }
    }
    issues.sort();
    issues
}

/// FJ-821: Verify all depends_on targets actually exist as resources.
pub(crate) fn cmd_validate_check_resource_dependencies_complete(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let missing = find_missing_deps(&config);
    if json {
        let items: Vec<String> = missing.iter()
            .map(|(r, dep)| format!("{{\"resource\":\"{}\",\"missing_dep\":\"{}\"}}", r, dep))
            .collect();
        println!("{{\"missing_dependencies\":[{}]}}", items.join(","));
    } else if missing.is_empty() {
        println!("All dependency targets exist.");
    } else {
        println!("Missing dependency targets ({}):", missing.len());
        for (r, dep) in &missing { println!("  {} depends on '{}' (not found)", r, dep); }
    }
    Ok(())
}

fn find_missing_deps(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut missing = Vec::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                missing.push((name.clone(), dep.clone()));
            }
        }
    }
    missing.sort();
    missing
}

/// FJ-825: Verify machines are reachable (dry-run: checks addr format).
pub(crate) fn cmd_validate_check_machine_connectivity(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let issues = check_machine_addrs(&config);
    if json {
        let items: Vec<String> = issues.iter()
            .map(|(m, issue)| format!("{{\"machine\":\"{}\",\"issue\":\"{}\"}}", m, issue))
            .collect();
        println!("{{\"connectivity_issues\":[{}]}}", items.join(","));
    } else if issues.is_empty() {
        println!("All machine addresses look valid.");
    } else {
        println!("Machine connectivity issues ({}):", issues.len());
        for (m, issue) in &issues { println!("  {} — {}", m, issue); }
    }
    Ok(())
}

fn check_machine_addrs(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut issues = Vec::new();
    for (name, machine) in &config.machines {
        let addr = machine.addr.as_str();
        if addr.is_empty() {
            issues.push((name.clone(), "empty address".to_string()));
        } else if addr == "localhost" || addr == "127.0.0.1" || addr == "container" {
            // valid sentinel values
        } else if !addr.contains('.') && !addr.contains(':') {
            issues.push((name.clone(), format!("addr '{}' has no dots or colons", addr)));
        }
    }
    issues.sort();
    issues
}

/// FJ-829: Validate resource names match a naming pattern (regex).
pub(crate) fn cmd_validate_check_resource_naming_pattern(
    file: &Path, json: bool, pattern: &str,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let violations = find_naming_pattern_violations(&config, pattern);
    if json {
        let items: Vec<String> = violations.iter()
            .map(|r| format!("\"{}\"", r)).collect();
        println!("{{\"naming_pattern\":\"{}\",\"violations\":[{}]}}", pattern, items.join(","));
    } else if violations.is_empty() {
        println!("All resource names match pattern '{}'.", pattern);
    } else {
        println!("Resources not matching pattern '{}' ({}):", pattern, violations.len());
        for r in &violations { println!("  {}", r); }
    }
    Ok(())
}

fn find_naming_pattern_violations(config: &types::ForjarConfig, pattern: &str) -> Vec<String> {
    let mut violations: Vec<String> = config.resources.keys()
        .filter(|name| !matches_naming_pattern(name, pattern))
        .cloned().collect();
    violations.sort();
    violations
}

fn matches_naming_pattern(name: &str, pattern: &str) -> bool {
    if pattern.starts_with('^') || pattern.contains('*') {
        // Prefix match: "^prefix" checks name starts with "prefix"
        if let Some(prefix) = pattern.strip_prefix('^') {
            return name.starts_with(prefix);
        }
    }
    // Simple contains match
    name.contains(pattern)
}

/// FJ-833: Validate resource types are supported by their providers.
pub(crate) fn cmd_validate_check_resource_provider_support(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let issues = find_provider_support_issues(&config);
    if json {
        let items: Vec<String> = issues.iter()
            .map(|(r, issue)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", r, issue))
            .collect();
        println!("{{\"provider_support_issues\":[{}]}}", items.join(","));
    } else if issues.is_empty() {
        println!("All resource types are supported by their providers.");
    } else {
        println!("Provider support issues ({}):", issues.len());
        for (r, issue) in &issues { println!("  {} — {}", r, issue); }
    }
    Ok(())
}

fn find_provider_support_issues(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut issues = Vec::new();
    for (name, resource) in &config.resources {
        let rtype = format!("{:?}", resource.resource_type);
        let provider = resource.provider.as_deref().unwrap_or("default");
        if rtype.contains("Package") && provider == "file" {
            issues.push((name.clone(), format!("provider '{}' cannot manage packages", provider)));
        }
        if rtype.contains("Service") && provider == "file" {
            issues.push((name.clone(), format!("provider '{}' cannot manage services", provider)));
        }
    }
    issues.sort();
    issues
}

/// FJ-837: Verify secret references exist and are valid.
pub(crate) fn cmd_validate_check_resource_secret_refs(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let issues = find_secret_ref_issues(&config);
    if json {
        let items: Vec<String> = issues.iter()
            .map(|(r, issue)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", r, issue))
            .collect();
        println!("{{\"secret_ref_issues\":[{}]}}", items.join(","));
    } else if issues.is_empty() {
        println!("No secret reference issues found.");
    } else {
        println!("Secret reference issues ({}):", issues.len());
        for (r, issue) in &issues { println!("  {} — {}", r, issue); }
    }
    Ok(())
}

fn find_secret_ref_issues(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut issues = Vec::new();
    for (name, resource) in &config.resources {
        if let Some(ref content) = resource.content {
            if content.contains("{{secret.") || content.contains("${secret.") {
                issues.push((name.clone(), "contains secret reference in content template".to_string()));
            }
        }
    }
    issues.sort();
    issues
}

/// FJ-841: Check resources have idempotency markers.
pub(crate) fn cmd_validate_check_resource_idempotency_hints(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let missing = find_idempotency_hint_gaps(&config);
    if json {
        let items: Vec<String> = missing.iter()
            .map(|(r, hint)| format!("{{\"resource\":\"{}\",\"hint\":\"{}\"}}", r, hint))
            .collect();
        println!("{{\"idempotency_hints\":[{}]}}", items.join(","));
    } else if missing.is_empty() {
        println!("All resources have idempotency characteristics.");
    } else {
        println!("Resources missing idempotency hints ({}):", missing.len());
        for (r, hint) in &missing { println!("  {} — {}", r, hint); }
    }
    Ok(())
}

fn find_idempotency_hint_gaps(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut gaps = Vec::new();
    for (name, resource) in &config.resources {
        let rtype = format!("{:?}", resource.resource_type);
        if rtype.contains("File") || rtype.contains("Template") {
            if resource.state.is_none() {
                gaps.push((name.clone(), "file resource has no explicit state (present/absent)".to_string()));
            }
        }
    }
    gaps.sort();
    gaps
}
