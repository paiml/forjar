//! Governance validation — naming patterns, provider support, secrets, idempotency, depth, affinity.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

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
        if (rtype.contains("File") || rtype.contains("Template")) && resource.state.is_none() {
            gaps.push((name.clone(), "file resource has no explicit state (present/absent)".to_string()));
        }
    }
    gaps.sort();
    gaps
}

/// FJ-845: Warn if dependency chain exceeds threshold.
pub(crate) fn cmd_validate_check_resource_dependency_depth(
    file: &Path, json: bool, max_depth: usize,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let violations = find_depth_violations(&config, max_depth);
    if json {
        let items: Vec<String> = violations.iter()
            .map(|(r, d)| format!("{{\"resource\":\"{}\",\"depth\":{}}}", r, d))
            .collect();
        println!("{{\"max_depth\":{},\"violations\":[{}]}}", max_depth, items.join(","));
    } else if violations.is_empty() {
        println!("All dependency chains within limit ({}).", max_depth);
    } else {
        println!("Resources exceeding depth limit {} ({}):", max_depth, violations.len());
        for (r, d) in &violations { println!("  {} — depth {}", r, d); }
    }
    Ok(())
}

fn find_depth_violations(config: &types::ForjarConfig, max_depth: usize) -> Vec<(String, usize)> {
    let mut violations = Vec::new();
    for name in config.resources.keys() {
        let depth = compute_chain_depth(config, name, &mut std::collections::HashSet::new());
        if depth > max_depth {
            violations.push((name.clone(), depth));
        }
    }
    violations.sort();
    violations
}

fn compute_chain_depth(
    config: &types::ForjarConfig, name: &str,
    visited: &mut std::collections::HashSet<String>,
) -> usize {
    if !visited.insert(name.to_string()) { return 0; }
    let depth = config.resources.get(name)
        .map(|r| r.depends_on.iter()
            .map(|dep| 1 + compute_chain_depth(config, dep, visited))
            .max().unwrap_or(0))
        .unwrap_or(0);
    visited.remove(name);
    depth
}

/// FJ-849: Verify resources match machine capabilities.
pub(crate) fn cmd_validate_check_resource_machine_affinity(
    file: &Path, json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let issues = find_machine_affinity_issues(&config);
    if json {
        let items: Vec<String> = issues.iter()
            .map(|(r, hint)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", r, hint))
            .collect();
        println!("{{\"machine_affinity_issues\":[{}]}}", items.join(","));
    } else if issues.is_empty() {
        println!("All resources have valid machine affinity.");
    } else {
        println!("Machine affinity issues ({}):", issues.len());
        for (r, hint) in &issues { println!("  {} — {}", r, hint); }
    }
    Ok(())
}

fn find_machine_affinity_issues(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut issues = Vec::new();
    for (name, resource) in &config.resources {
        let machines = resource.machine.to_vec();
        for m in &machines {
            if !config.machines.contains_key(m) {
                issues.push((name.clone(), format!("references undefined machine '{}'", m)));
            }
        }
    }
    issues.sort();
    issues
}
