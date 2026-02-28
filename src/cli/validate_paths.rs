//! Path and mount validation.

use crate::core::types;
use std::path::Path;
use super::helpers::*;
use std::collections::HashMap;


/// FJ-671: Detect overlapping file paths across resources
pub(crate) fn cmd_validate_check_path_conflicts(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

    let mut path_owners: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, resource) in &config.resources {
        if let Some(ref path) = resource.path {
            path_owners
                .entry(path.clone())
                .or_default()
                .push(name.clone());
        }
    }

    let mut conflicts = Vec::new();
    for (path, owners) in &path_owners {
        if owners.len() > 1 {
            conflicts.push(format!("Path '{}' claimed by: {}", path, owners.join(", ")));
        }
    }

    if json {
        print!("{{\"conflicts\":[");
        for (i, c) in conflicts.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(r#""{}""#, c.replace('"', "\\\""));
        }
        println!("]}}");
    } else if conflicts.is_empty() {
        println!("No file path conflicts detected");
    } else {
        println!("Path conflicts ({}):", conflicts.len());
        for c in &conflicts {
            println!("  - {}", c);
        }
    }
    Ok(())
}


/// Scan text for unresolved template variables.
fn find_undefined_vars(
    field: &str,
    name: &str,
    params: &std::collections::HashSet<String>,
    undefined: &mut Vec<(String, String)>,
) {
    let mut rest = field;
    while let Some(start) = rest.find("{{") {
        if let Some(end) = rest[start..].find("}}") {
            let var = rest[start + 2..start + end].trim();
            let key = var.strip_prefix("params.").unwrap_or(var);
            if !params.contains(key) {
                undefined.push((name.to_string(), key.to_string()));
            }
            rest = &rest[start + end + 2..];
        } else {
            break;
        }
    }
}

/// FJ-691: Validate all template variables are defined
pub(crate) fn cmd_validate_check_template_vars(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut undefined: Vec<(String, String)> = Vec::new();
    let params: std::collections::HashSet<String> = cfg.params.keys().cloned().collect();
    for (name, resource) in &cfg.resources {
        let fields = [
            resource.path.as_deref(),
            resource.content.as_deref(),
            resource.owner.as_deref(),
        ];
        for field in fields.into_iter().flatten() {
            find_undefined_vars(field, name, &params, &mut undefined);
        }
    }
    if json {
        println!(
            "{{\"check\":\"template_vars\",\"undefined_count\":{},\"undefined\":[{}]}}",
            undefined.len(),
            undefined
                .iter()
                .map(|(r, v)| format!("{{\"resource\":\"{}\",\"var\":\"{}\"}}", r, v))
                .collect::<Vec<_>>()
                .join(",")
        );
    } else if undefined.is_empty() {
        println!("All template variables are defined.");
    } else {
        println!("Undefined template variables:");
        for (resource, var) in &undefined {
            println!("  {} -> {}", resource, var);
        }
    }
    Ok(())
}


/// FJ-701: Validate file mode consistency across resources
pub(crate) fn cmd_validate_check_mode_consistency(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let dir_modes = collect_dir_modes(&cfg);
    let inconsistencies = find_mode_inconsistencies(&dir_modes);

    if json {
        let entries: Vec<String> = inconsistencies
            .iter()
            .map(|(dir, name, mode)| {
                format!(
                    "{{\"directory\":\"{}\",\"resource\":\"{}\",\"mode\":\"{}\"}}",
                    dir, name, mode
                )
            })
            .collect();
        println!(
            "{{\"check\":\"mode_consistency\",\"inconsistency_count\":{},\"details\":[{}]}}",
            inconsistencies.len(),
            entries.join(",")
        );
    } else if inconsistencies.is_empty() {
        println!("All file modes are consistent.");
    } else {
        println!("File mode inconsistencies found:");
        for (dir, name, mode) in &inconsistencies {
            println!("  {} in {} — mode {}", name, dir, mode);
        }
    }
    Ok(())
}

/// Collect directory-to-mode mappings from resources.
fn collect_dir_modes(
    cfg: &types::ForjarConfig,
) -> std::collections::HashMap<String, Vec<(String, String)>> {
    let mut dir_modes: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();
    for (name, resource) in &cfg.resources {
        if let Some(ref path) = resource.path {
            if let Some(ref mode) = resource.mode {
                let parent = std::path::Path::new(path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                dir_modes
                    .entry(parent)
                    .or_default()
                    .push((name.clone(), mode.clone()));
            }
        }
    }
    dir_modes
}

/// Find inconsistencies in file modes within the same directory.
fn find_mode_inconsistencies(
    dir_modes: &std::collections::HashMap<String, Vec<(String, String)>>,
) -> Vec<(String, String, String)> {
    let mut inconsistencies: Vec<(String, String, String)> = Vec::new();
    for (dir, entries) in dir_modes {
        if entries.len() > 1 {
            let modes: std::collections::HashSet<&str> =
                entries.iter().map(|(_, m)| m.as_str()).collect();
            if modes.len() > 1 {
                for (name, mode) in entries {
                    inconsistencies.push((dir.clone(), name.clone(), mode.clone()));
                }
            }
        }
    }
    inconsistencies
}


/// FJ-711: Validate user/group consistency across resources
pub(crate) fn cmd_validate_check_group_consistency(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut owner_groups: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, resource) in &cfg.resources {
        if let Some(ref owner) = resource.owner {
            owner_groups
                .entry(owner.clone())
                .or_default()
                .push(name.clone());
        }
    }
    if json {
        let entries: Vec<String> = owner_groups
            .iter()
            .map(|(owner, resources)| {
                let res: Vec<String> = resources.iter().map(|r| format!("\"{}\"", r)).collect();
                format!(
                    "{{\"owner\":\"{}\",\"resource_count\":{},\"resources\":[{}]}}",
                    owner,
                    resources.len(),
                    res.join(",")
                )
            })
            .collect();
        println!(
            "{{\"check\":\"group_consistency\",\"owners\":[{}]}}",
            entries.join(",")
        );
    } else if owner_groups.is_empty() {
        println!("No owner fields specified in resources.");
    } else {
        println!("Owner/group consistency:");
        for (owner, resources) in &owner_groups {
            println!(
                "  {} — {} resource(s): {}",
                owner,
                resources.len(),
                resources.join(", ")
            );
        }
    }
    Ok(())
}


/// FJ-721: Validate mount point paths don't conflict
pub(crate) fn cmd_validate_check_mount_points(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let mut mount_paths: Vec<(String, String)> = Vec::new();
    for (name, resource) in &cfg.resources {
        if format!("{:?}", resource.resource_type).contains("Mount") {
            if let Some(ref path) = resource.path {
                mount_paths.push((name.clone(), path.clone()));
            }
        }
    }
    let conflicts = find_mount_conflicts(&mount_paths);
    if json {
        let entries: Vec<String> = conflicts
            .iter()
            .map(|(a, b)| format!("{{\"resource_a\":\"{}\",\"resource_b\":\"{}\"}}", a, b))
            .collect();
        println!(
            "{{\"check\":\"mount_points\",\"conflict_count\":{},\"conflicts\":[{}]}}",
            conflicts.len(),
            entries.join(",")
        );
    } else if conflicts.is_empty() {
        println!("No mount point conflicts found.");
    } else {
        println!("Mount point conflicts:");
        for (a, b) in &conflicts {
            println!("  {} <-> {}", a, b);
        }
    }
    Ok(())
}

/// Find mount point conflicts (overlapping paths).
fn find_mount_conflicts(mount_paths: &[(String, String)]) -> Vec<(String, String)> {
    let mut conflicts: Vec<(String, String)> = Vec::new();
    for i in 0..mount_paths.len() {
        for j in (i + 1)..mount_paths.len() {
            let (ref n1, ref p1) = mount_paths[i];
            let (ref n2, ref p2) = mount_paths[j];
            if p1 == p2
                || p1.starts_with(&format!("{}/", p2))
                || p2.starts_with(&format!("{}/", p1))
            {
                conflicts.push((n1.clone(), n2.clone()));
            }
        }
    }
    conflicts
}

/// Validate a cron field token against min/max range.
fn validate_cron_field(field: &str, min: u32, max: u32) -> bool {
    if field == "*" { return true; }
    for part in field.split(',') {
        let part = part.split('/').next().unwrap_or(part);
        if part == "*" { continue; }
        if let Some((a, b)) = part.split_once('-') {
            let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) else { return false; };
            if a < min || b > max || a > b { return false; }
        } else {
            let Ok(v) = part.parse::<u32>() else { return false; };
            if v < min || v > max { return false; }
        }
    }
    true
}

/// FJ-731: Validate cron schedule expressions in resources.
pub(crate) fn cmd_validate_check_cron_syntax(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;

    let mut issues: Vec<(String, String)> = Vec::new();
    for (name, resource) in &config.resources {
        if let Some(ref sched) = resource.schedule {
            let parts: Vec<&str> = sched.split_whitespace().collect();
            if parts.len() != 5 {
                issues.push((name.clone(), format!("Expected 5 fields, got {}", parts.len())));
                continue;
            }
            let ranges = [(0u32, 59u32), (0, 23), (1, 31), (1, 12), (0, 6)];
            let labels = ["minute", "hour", "day-of-month", "month", "day-of-week"];
            for (i, (min, max)) in ranges.iter().enumerate() {
                if !validate_cron_field(parts[i], *min, *max) {
                    issues.push((name.clone(), format!("Invalid {} field: '{}'", labels[i], parts[i])));
                }
            }
        }
    }

    if json {
        let entries: Vec<String> = issues
            .iter()
            .map(|(n, m)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", n, m))
            .collect();
        println!("{{\"cron_issues\":[{}]}}", entries.join(","));
    } else if issues.is_empty() {
        println!("All cron schedules are valid.");
    } else {
        println!("Cron syntax issues ({}):", issues.len());
        for (name, msg) in &issues {
            println!("  {} — {}", name, msg);
        }
    }
    Ok(())
}


/// Extract env var names from {{env.VAR}} references in text.
fn extract_env_refs(content: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let marker = "{{env.";
    let mut pos = 0;
    while let Some(start) = content[pos..].find(marker) {
        let var_start = pos + start + marker.len();
        if let Some(end) = content[var_start..].find("}}") {
            let var = &content[var_start..var_start + end];
            if var.chars().all(|c| c.is_alphanumeric() || c == '_') && !refs.contains(&var.to_string()) {
                refs.push(var.to_string());
            }
        }
        pos = var_start;
    }
    refs
}

/// FJ-741: Verify all {{env.*}} references have matching environment variables.
pub(crate) fn cmd_validate_check_env_refs(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let refs = extract_env_refs(&content);
    let missing: Vec<&String> = refs.iter().filter(|v| std::env::var(v).is_err()).collect();
    if json {
        let items: Vec<String> = missing.iter().map(|v| format!("\"{}\"", v)).collect();
        println!("{{\"missing_env_refs\":[{}]}}", items.join(","));
    } else if missing.is_empty() {
        println!("All env references are satisfied.");
    } else {
        println!("Missing env vars ({}):", missing.len());
        for var in &missing { println!("  {}", var); }
    }
    Ok(())
}


/// FJ-745: Enforce resource naming pattern (kebab-case check or prefix match).
pub(crate) fn cmd_validate_check_resource_names(
    file: &Path, json: bool, pattern: &str,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let violations = find_naming_violations(&config, pattern);
    if json {
        let items: Vec<String> = violations.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"naming_violations\":[{}]}}", items.join(","));
    } else if violations.is_empty() {
        println!("All resource names match pattern: {}", pattern);
    } else {
        println!("Resource naming violations ({}):", violations.len());
        for name in &violations { println!("  {} — does not match '{}'", name, pattern); }
    }
    Ok(())
}

/// Check resource names against a pattern (prefix match or kebab-case).
fn find_naming_violations(config: &types::ForjarConfig, pattern: &str) -> Vec<String> {
    let mut violations: Vec<String> = Vec::new();
    for name in config.resources.keys() {
        let matches = if pattern == "kebab-case" {
            name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        } else {
            name.starts_with(pattern)
        };
        if !matches { violations.push(name.clone()); }
    }
    violations.sort();
    violations
}


/// FJ-749: Warn if resource count exceeds threshold per machine.
pub(crate) fn cmd_validate_check_resource_count(
    file: &Path, json: bool, limit: usize,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let counts = count_resources_per_machine(&config);
    let over: Vec<(&String, &usize)> = counts.iter().filter(|(_, c)| **c > limit).collect();
    if json {
        let items: Vec<String> = over.iter()
            .map(|(m, c)| format!("{{\"machine\":\"{}\",\"count\":{},\"limit\":{}}}", m, c, limit))
            .collect();
        println!("{{\"resource_count_violations\":[{}]}}", items.join(","));
    } else if over.is_empty() {
        println!("All machines within resource limit ({}).", limit);
    } else {
        println!("Resource count violations (limit: {}):", limit);
        for (m, c) in &over { println!("  {} — {} resources (over by {})", m, c, *c - limit); }
    }
    Ok(())
}

/// Count resources targeting each machine.
fn count_resources_per_machine(config: &types::ForjarConfig) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for resource in config.resources.values() {
        for m in resource.machine.to_vec() { *counts.entry(m).or_default() += 1; }
    }
    counts
}


/// FJ-753: Detect duplicate file paths across resources on same machine.
pub(crate) fn cmd_validate_check_duplicate_paths(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {}", e))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {}", e))?;
    let dupes = find_duplicate_paths(&config);
    if json {
        let items: Vec<String> = dupes.iter()
            .map(|(p, names)| format!("{{\"path\":\"{}\",\"resources\":{:?}}}", p, names))
            .collect();
        println!("{{\"duplicate_paths\":[{}]}}", items.join(","));
    } else if dupes.is_empty() {
        println!("No duplicate file paths detected.");
    } else {
        println!("Duplicate paths ({}):", dupes.len());
        for (path, names) in &dupes { println!("  {} — {}", path, names.join(", ")); }
    }
    Ok(())
}

/// Find paths claimed by multiple resources.
fn find_duplicate_paths(config: &types::ForjarConfig) -> Vec<(String, Vec<String>)> {
    let mut path_map: HashMap<String, Vec<String>> = HashMap::new();
    for (name, resource) in &config.resources {
        if let Some(ref p) = resource.path {
            path_map.entry(p.clone()).or_default().push(name.clone());
        }
    }
    let mut dupes: Vec<(String, Vec<String>)> = path_map.into_iter()
        .filter(|(_, v)| v.len() > 1)
        .collect();
    dupes.sort_by(|a, b| a.0.cmp(&b.0));
    dupes
}
