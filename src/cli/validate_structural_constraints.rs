//! Structural validation: naming, overlaps, limits, circular refs.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

// ── FJ-481: validate --check-naming ──

pub(crate) fn cmd_validate_check_naming(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut violations: Vec<String> = Vec::new();
    for name in config.resources.keys() {
        let is_kebab = !name.is_empty()
            && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !name.contains("--")
            && !name.ends_with('-');
        if !is_kebab {
            violations.push(format!(
                "'{name}' is not kebab-case (expected: lowercase letters, digits, hyphens)"
            ));
        }
    }
    if json {
        let result = serde_json::json!({
            "naming_violations": violations,
            "valid": violations.is_empty(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else if violations.is_empty() {
        println!(
            "{} All resource names follow kebab-case convention",
            green("✓")
        );
    } else {
        println!("{} Naming violations ({}):", red("✗"), violations.len());
        for v in &violations {
            println!("  - {v}");
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!("{} naming violations", violations.len()))
    }
}

/// Build a map from (machine, path) to resource names.
fn build_path_map(
    config: &types::ForjarConfig,
) -> std::collections::HashMap<(String, String), Vec<String>> {
    let mut path_map: std::collections::HashMap<(String, String), Vec<String>> =
        std::collections::HashMap::new();
    for (name, res) in &config.resources {
        if let Some(path) = &res.path {
            let machine = match &res.machine {
                types::MachineTarget::Single(s) => s.clone(),
                types::MachineTarget::Multiple(ms) => ms.first().cloned().unwrap_or_default(),
            };
            path_map
                .entry((machine, path.clone()))
                .or_default()
                .push(name.clone());
        }
    }
    path_map
}

/// Find overlapping paths from the path map.
fn find_overlaps(
    path_map: &std::collections::HashMap<(String, String), Vec<String>>,
) -> Vec<String> {
    let mut overlaps: Vec<String> = Vec::new();
    for ((machine, path), names) in path_map {
        if names.len() > 1 {
            overlaps.push(format!(
                "path '{}' on machine '{}' used by: {}",
                path,
                machine,
                names.join(", ")
            ));
        }
    }
    overlaps.sort();
    overlaps
}

// ── FJ-491: validate --check-overlaps ──

pub(crate) fn cmd_validate_check_overlaps(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let path_map = build_path_map(&config);
    let overlaps = find_overlaps(&path_map);
    if json {
        let result =
            serde_json::json!({ "overlaps": overlaps, "has_overlaps": !overlaps.is_empty() });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else if overlaps.is_empty() {
        println!("{} No resource overlaps detected", green("✓"));
    } else {
        println!("{} Resource overlaps ({}):", red("✗"), overlaps.len());
        for o in &overlaps {
            println!("  - {o}");
        }
    }
    if overlaps.is_empty() {
        Ok(())
    } else {
        Err(format!("{} overlap(s) detected", overlaps.len()))
    }
}

// ── FJ-501: validate --check-limits ──

pub(crate) fn cmd_validate_check_limits(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for res in config.resources.values() {
        let machine_name = match &res.machine {
            types::MachineTarget::Single(s) => s.clone(),
            types::MachineTarget::Multiple(ms) => ms.first().cloned().unwrap_or_default(),
        };
        let key = format!("{}:{:?}", machine_name, res.resource_type);
        *counts.entry(key).or_insert(0) += 1;
    }
    let max_per_type = 50;
    let mut violations: Vec<String> = Vec::new();
    for (key, count) in &counts {
        if *count > max_per_type {
            violations.push(format!(
                "{key} has {count} resources (limit: {max_per_type})"
            ));
        }
    }
    if json {
        let result = serde_json::json!({ "limits": counts, "violations": violations, "valid": violations.is_empty() });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else if violations.is_empty() {
        println!(
            "{} All resource counts within limits (max {} per machine/type)",
            green("✓"),
            max_per_type
        );
    } else {
        println!("{} Resource limit violations:", red("✗"));
        for v in &violations {
            println!("  - {v}");
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!("{} limit violations", violations.len()))
    }
}

/// FJ-631: Detect circular template/param references.
pub(crate) fn cmd_validate_check_circular_refs(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut cycles: Vec<String> = Vec::new();

    for name in config.resources.keys() {
        if has_circular_dep(name, &config) {
            cycles.push(name.clone());
        }
    }

    if json {
        let items: Vec<String> = cycles.iter().map(|c| format!(r#""{c}""#)).collect();
        println!(
            r#"{{"circular_refs":[{}],"count":{}}}"#,
            items.join(","),
            cycles.len()
        );
    } else if cycles.is_empty() {
        println!("No circular references detected");
    } else {
        println!("Circular references found ({}):", cycles.len());
        for c in &cycles {
            println!("  {c} (circular dependency)");
        }
        return Err(format!("{} circular reference(s)", cycles.len()));
    }
    Ok(())
}

/// Check if a resource has a circular dependency back to itself via DFS.
fn has_circular_dep(name: &str, config: &types::ForjarConfig) -> bool {
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![name.to_string()];
    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            if current == name {
                return true;
            }
            continue;
        }
        if let Some(resource) = config.resources.get(&current) {
            for dep in &resource.depends_on {
                stack.push(dep.clone());
            }
        }
    }
    false
}

/// FJ-641: Validate naming conventions across resources
pub(crate) fn cmd_validate_check_naming_conventions(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;

    let mut violations = Vec::new();
    for (name, _resource) in &config.resources {
        check_naming_convention(name, &mut violations);
    }

    if json {
        println!(
            r#"{{"violations_count":{},"violations":[{}]}}"#,
            violations.len(),
            violations
                .iter()
                .map(|v| format!(r#""{}""#, v.replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(",")
        );
    } else if violations.is_empty() {
        println!("All resource names follow naming conventions");
    } else {
        println!("Naming convention violations ({}):", violations.len());
        for v in &violations {
            println!("  - {v}");
        }
    }
    Ok(())
}

/// Check a single resource name against naming conventions.
fn check_naming_convention(name: &str, violations: &mut Vec<String>) {
    let is_kebab = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !is_kebab {
        violations.push(format!(
            "Resource '{name}': name should be kebab-case (lowercase, hyphens only)"
        ));
    }
    if name.starts_with('-') || name.ends_with('-') {
        violations.push(format!(
            "Resource '{name}': name should not start or end with hyphen"
        ));
    }
    if name.contains("--") {
        violations.push(format!(
            "Resource '{name}': name should not contain consecutive hyphens"
        ));
    }
}
