//! Path and mount validation.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// FJ-671: Detect overlapping file paths across resources
pub(crate) fn cmd_validate_check_path_conflicts(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;

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
            println!("  - {c}");
        }
    }
    Ok(())
}

/// Scan text for unresolved template variables.
pub(super) fn find_undefined_vars(
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
                .map(|(r, v)| format!("{{\"resource\":\"{r}\",\"var\":\"{v}\"}}"))
                .collect::<Vec<_>>()
                .join(",")
        );
    } else if undefined.is_empty() {
        println!("All template variables are defined.");
    } else {
        println!("Undefined template variables:");
        for (resource, var) in &undefined {
            println!("  {resource} -> {var}");
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
                    "{{\"directory\":\"{dir}\",\"resource\":\"{name}\",\"mode\":\"{mode}\"}}"
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
            println!("  {name} in {dir} — mode {mode}");
        }
    }
    Ok(())
}

/// Collect directory-to-mode mappings from resources.
pub(super) fn collect_dir_modes(
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
pub(super) fn find_mode_inconsistencies(
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
                let res: Vec<String> = resources.iter().map(|r| format!("\"{r}\"")).collect();
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
            .map(|(a, b)| format!("{{\"resource_a\":\"{a}\",\"resource_b\":\"{b}\"}}"))
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
            println!("  {a} <-> {b}");
        }
    }
    Ok(())
}

pub(super) use super::validate_paths_b::*;
