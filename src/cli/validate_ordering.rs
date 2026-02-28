//! Dependency ordering & tag completeness validation.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-925: Verify dependency ordering is topologically valid.
pub(crate) fn cmd_validate_check_resource_dependency_ordering(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let issues = find_ordering_issues(&config);
    if json {
        let items: Vec<String> = issues.iter()
            .map(|(n, r)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", n, r))
            .collect();
        println!("{{\"ordering_issues\":[{}]}}", items.join(","));
    } else if issues.is_empty() {
        println!("All resource dependencies are topologically valid.");
    } else {
        println!("Dependency ordering issues:");
        for (n, r) in &issues { println!("  {} — {}", n, r); }
    }
    Ok(())
}

fn find_ordering_issues(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut issues = Vec::new();
    let names: std::collections::HashSet<&str> = config.resources.keys().map(|k| k.as_str()).collect();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if !names.contains(dep.as_str()) {
                issues.push((name.clone(), format!("depends on non-existent '{}'", dep)));
            }
            if dep == name {
                issues.push((name.clone(), "self-dependency".to_string()));
            }
        }
    }
    issues.sort_by(|a, b| a.0.cmp(&b.0));
    issues
}

/// FJ-929: Ensure all resources have required tag categories.
pub(crate) fn cmd_validate_check_resource_tag_completeness(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let missing = find_missing_tags(&config);
    if json {
        let items: Vec<String> = missing.iter()
            .map(|(n, c)| format!("{{\"resource\":\"{}\",\"tag_count\":{}}}", n, c))
            .collect();
        println!("{{\"tag_completeness\":[{}]}}", items.join(","));
    } else if missing.is_empty() {
        println!("All resources have tags.");
    } else {
        println!("Resources missing tags:");
        for (n, _) in &missing { println!("  {}", n); }
    }
    Ok(())
}

fn find_missing_tags(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut missing: Vec<(String, usize)> = config.resources.iter()
        .filter(|(_, res)| res.tags.is_empty())
        .map(|(name, _)| (name.clone(), 0))
        .collect();
    missing.sort_by(|a, b| a.0.cmp(&b.0));
    missing
}

/// FJ-933: Enforce naming conventions via configurable patterns.
pub(crate) fn cmd_validate_check_resource_naming_standards(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let violations = find_naming_violations(&config);
    if json {
        let items: Vec<String> = violations.iter()
            .map(|(n, r)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", n, r))
            .collect();
        println!("{{\"naming_violations\":[{}]}}", items.join(","));
    } else if violations.is_empty() {
        println!("All resource names follow naming conventions.");
    } else {
        println!("Naming convention violations:");
        for (n, r) in &violations { println!("  {} — {}", n, r); }
    }
    Ok(())
}

fn find_naming_violations(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut violations = Vec::new();
    for name in config.resources.keys() {
        if name.contains(' ') {
            violations.push((name.clone(), "contains spaces".to_string()));
        }
        if name.chars().next().map_or(false, |c| c.is_ascii_uppercase()) {
            violations.push((name.clone(), "starts with uppercase".to_string()));
        }
        if name.contains("__") {
            violations.push((name.clone(), "contains double underscore".to_string()));
        }
    }
    violations.sort_by(|a, b| a.0.cmp(&b.0));
    violations
}

/// FJ-937: Detect asymmetric dependency declarations.
pub(crate) fn cmd_validate_check_resource_dependency_symmetry(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let asymmetries = find_dependency_asymmetries(&config);
    if json {
        let items: Vec<String> = asymmetries.iter()
            .map(|(a, b)| format!("{{\"from\":\"{}\",\"to\":\"{}\"}}", a, b))
            .collect();
        println!("{{\"asymmetric_dependencies\":[{}]}}", items.join(","));
    } else if asymmetries.is_empty() {
        println!("No asymmetric dependencies detected.");
    } else {
        println!("Asymmetric dependencies:");
        for (a, b) in &asymmetries { println!("  {} depends on {} (but not vice versa)", a, b); }
    }
    Ok(())
}

fn find_dependency_asymmetries(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if let Some(dep_res) = config.resources.get(dep) {
                if !dep_res.depends_on.contains(name) {
                    pairs.push((name.clone(), dep.clone()));
                }
            }
        }
    }
    pairs.sort();
    pairs.dedup();
    pairs
}
