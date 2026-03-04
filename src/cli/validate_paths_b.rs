use crate::core::types;
use std::collections::HashMap;
use std::path::Path;

/// Find mount point conflicts (overlapping paths).
pub(super) fn find_mount_conflicts(mount_paths: &[(String, String)]) -> Vec<(String, String)> {
    let mut conflicts: Vec<(String, String)> = Vec::new();
    for i in 0..mount_paths.len() {
        for j in (i + 1)..mount_paths.len() {
            let (ref n1, ref p1) = mount_paths[i];
            let (ref n2, ref p2) = mount_paths[j];
            if p1 == p2
                || p1.starts_with(&format!("{p2}/"))
                || p2.starts_with(&format!("{p1}/"))
            {
                conflicts.push((n1.clone(), n2.clone()));
            }
        }
    }
    conflicts
}

/// Validate a cron field token against min/max range.
fn validate_cron_field(field: &str, min: u32, max: u32) -> bool {
    if field == "*" {
        return true;
    }
    for part in field.split(',') {
        let part = part.split('/').next().unwrap_or(part);
        if part == "*" {
            continue;
        }
        if let Some((a, b)) = part.split_once('-') {
            let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) else {
                return false;
            };
            if a < min || b > max || a > b {
                return false;
            }
        } else {
            let Ok(v) = part.parse::<u32>() else {
                return false;
            };
            if v < min || v > max {
                return false;
            }
        }
    }
    true
}

/// FJ-731: Validate cron schedule expressions in resources.
pub(crate) fn cmd_validate_check_cron_syntax(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;

    let mut issues: Vec<(String, String)> = Vec::new();
    for (name, resource) in &config.resources {
        if let Some(ref sched) = resource.schedule {
            let parts: Vec<&str> = sched.split_whitespace().collect();
            if parts.len() != 5 {
                issues.push((
                    name.clone(),
                    format!("Expected 5 fields, got {}", parts.len()),
                ));
                continue;
            }
            let ranges = [(0u32, 59u32), (0, 23), (1, 31), (1, 12), (0, 6)];
            let labels = ["minute", "hour", "day-of-month", "month", "day-of-week"];
            for (i, (min, max)) in ranges.iter().enumerate() {
                if !validate_cron_field(parts[i], *min, *max) {
                    issues.push((
                        name.clone(),
                        format!("Invalid {} field: '{}'", labels[i], parts[i]),
                    ));
                }
            }
        }
    }

    if json {
        let entries: Vec<String> = issues
            .iter()
            .map(|(n, m)| format!("{{\"resource\":\"{n}\",\"issue\":\"{m}\"}}"))
            .collect();
        println!("{{\"cron_issues\":[{}]}}", entries.join(","));
    } else if issues.is_empty() {
        println!("All cron schedules are valid.");
    } else {
        println!("Cron syntax issues ({}):", issues.len());
        for (name, msg) in &issues {
            println!("  {name} — {msg}");
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
            if var.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !refs.contains(&var.to_string())
            {
                refs.push(var.to_string());
            }
        }
        pos = var_start;
    }
    refs
}

/// FJ-741: Verify all {{env.*}} references have matching environment variables.
pub(crate) fn cmd_validate_check_env_refs(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let refs = extract_env_refs(&content);
    let missing: Vec<&String> = refs.iter().filter(|v| std::env::var(v).is_err()).collect();
    if json {
        let items: Vec<String> = missing.iter().map(|v| format!("\"{v}\"")).collect();
        println!("{{\"missing_env_refs\":[{}]}}", items.join(","));
    } else if missing.is_empty() {
        println!("All env references are satisfied.");
    } else {
        println!("Missing env vars ({}):", missing.len());
        for var in &missing {
            println!("  {var}");
        }
    }
    Ok(())
}

/// FJ-745: Enforce resource naming pattern (kebab-case check or prefix match).
pub(crate) fn cmd_validate_check_resource_names(
    file: &Path,
    json: bool,
    pattern: &str,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let violations = find_naming_violations(&config, pattern);
    if json {
        let items: Vec<String> = violations.iter().map(|n| format!("\"{n}\"")).collect();
        println!("{{\"naming_violations\":[{}]}}", items.join(","));
    } else if violations.is_empty() {
        println!("All resource names match pattern: {pattern}");
    } else {
        println!("Resource naming violations ({}):", violations.len());
        for name in &violations {
            println!("  {name} — does not match '{pattern}'");
        }
    }
    Ok(())
}

/// Check resource names against a pattern (prefix match or kebab-case).
fn find_naming_violations(config: &types::ForjarConfig, pattern: &str) -> Vec<String> {
    let mut violations: Vec<String> = Vec::new();
    for name in config.resources.keys() {
        let matches = if pattern == "kebab-case" {
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        } else {
            name.starts_with(pattern)
        };
        if !matches {
            violations.push(name.clone());
        }
    }
    violations.sort();
    violations
}

/// FJ-749: Warn if resource count exceeds threshold per machine.
pub(crate) fn cmd_validate_check_resource_count(
    file: &Path,
    json: bool,
    limit: usize,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let counts = count_resources_per_machine(&config);
    let over: Vec<(&String, &usize)> = counts.iter().filter(|(_, c)| **c > limit).collect();
    if json {
        let items: Vec<String> = over
            .iter()
            .map(|(m, c)| {
                format!(
                    "{{\"machine\":\"{m}\",\"count\":{c},\"limit\":{limit}}}"
                )
            })
            .collect();
        println!("{{\"resource_count_violations\":[{}]}}", items.join(","));
    } else if over.is_empty() {
        println!("All machines within resource limit ({limit}).");
    } else {
        println!("Resource count violations (limit: {limit}):");
        for (m, c) in &over {
            println!("  {} — {} resources (over by {})", m, c, *c - limit);
        }
    }
    Ok(())
}

/// Count resources targeting each machine.
fn count_resources_per_machine(config: &types::ForjarConfig) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for resource in config.resources.values() {
        for m in resource.machine.to_vec() {
            *counts.entry(m).or_default() += 1;
        }
    }
    counts
}

/// FJ-753: Detect duplicate file paths across resources on same machine.
pub(crate) fn cmd_validate_check_duplicate_paths(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;
    let dupes = find_duplicate_paths(&config);
    if json {
        let items: Vec<String> = dupes
            .iter()
            .map(|(p, names)| format!("{{\"path\":\"{p}\",\"resources\":{names:?}}}"))
            .collect();
        println!("{{\"duplicate_paths\":[{}]}}", items.join(","));
    } else if dupes.is_empty() {
        println!("No duplicate file paths detected.");
    } else {
        println!("Duplicate paths ({}):", dupes.len());
        for (path, names) in &dupes {
            println!("  {} — {}", path, names.join(", "));
        }
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
    let mut dupes: Vec<(String, Vec<String>)> =
        path_map.into_iter().filter(|(_, v)| v.len() > 1).collect();
    dupes.sort_by(|a, b| a.0.cmp(&b.0));
    dupes
}
