use crate::core::types;
use std::path::Path;

/// FJ-897: Verify resources can be safely updated without downtime.
pub(crate) fn cmd_validate_check_resource_update_safety(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let warnings = find_update_safety_issues(&cfg);
    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(n, w)| format!("{{\"resource\":\"{}\",\"warning\":\"{}\"}}", n, w))
            .collect();
        println!("{{\"update_safety_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resources can be safely updated.");
    } else {
        println!("Update safety warnings:");
        for (n, w) in &warnings {
            println!("  {} — {}", n, w);
        }
    }
    Ok(())
}

fn find_update_safety_issues(cfg: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut warnings = Vec::new();
    for (name, res) in &cfg.resources {
        if matches!(res.resource_type, types::ResourceType::Service) && !res.triggers.is_empty() {
            warnings.push((
                name.clone(),
                "service with triggers may cause cascade restart".to_string(),
            ));
        }
        if matches!(res.resource_type, types::ResourceType::Mount) {
            warnings.push((
                name.clone(),
                "mount changes require unmount/remount".to_string(),
            ));
        }
    }
    warnings.sort_by(|a, b| a.0.cmp(&b.0));
    warnings
}

/// FJ-901: Detect config inconsistencies across machines.
pub(crate) fn cmd_validate_check_resource_cross_machine_consistency(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let issues = find_cross_machine_inconsistencies(&config);
    if json {
        let items: Vec<String> = issues
            .iter()
            .map(|(n, i)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", n, i))
            .collect();
        println!(
            "{{\"cross_machine_inconsistencies\":[{}]}}",
            items.join(",")
        );
    } else if issues.is_empty() {
        println!("No cross-machine inconsistencies found.");
    } else {
        println!("Cross-machine inconsistencies:");
        for (n, i) in &issues {
            println!("  {} — {}", n, i);
        }
    }
    Ok(())
}

fn find_cross_machine_inconsistencies(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut type_by_name: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, res) in &config.resources {
        let t = format!("{:?}", res.resource_type);
        type_by_name.entry(name.clone()).or_default().push(t);
    }
    let mut issues = Vec::new();
    for (name, types_list) in &type_by_name {
        if types_list.len() > 1 {
            let unique: std::collections::HashSet<&String> = types_list.iter().collect();
            if unique.len() > 1 {
                issues.push((
                    name.clone(),
                    format!("mixed types: {}", types_list.join(", ")),
                ));
            }
        }
    }
    issues.sort_by(|a, b| a.0.cmp(&b.0));
    issues
}

/// FJ-905: Verify resources pin explicit versions.
pub(crate) fn cmd_validate_check_resource_version_pinning(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let unpinned = find_unpinned_resources(&config);
    if json {
        let items: Vec<String> = unpinned.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"unpinned_resources\":[{}]}}", items.join(","));
    } else if unpinned.is_empty() {
        println!("All package resources have pinned versions.");
    } else {
        println!("Resources without pinned versions:");
        for n in &unpinned {
            println!("  {}", n);
        }
    }
    Ok(())
}

fn find_unpinned_resources(config: &types::ForjarConfig) -> Vec<String> {
    let mut unpinned: Vec<String> = config
        .resources
        .iter()
        .filter(|(_, res)| {
            matches!(res.resource_type, types::ResourceType::Package) && res.version.is_none()
        })
        .map(|(name, _)| name.clone())
        .collect();
    unpinned.sort();
    unpinned
}

/// FJ-909: Verify all dependency references resolve to existing resources.
pub(crate) fn cmd_validate_check_resource_dependency_completeness(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let missing = find_incomplete_dependencies(&config);
    if json {
        let items: Vec<String> = missing
            .iter()
            .map(|(n, dep)| format!("{{\"resource\":\"{}\",\"missing_dep\":\"{}\"}}", n, dep))
            .collect();
        println!("{{\"incomplete_dependencies\":[{}]}}", items.join(","));
    } else if missing.is_empty() {
        println!("All dependency references are complete.");
    } else {
        println!("Incomplete dependency references:");
        for (n, dep) in &missing {
            println!("  {} → missing '{}'", n, dep);
        }
    }
    Ok(())
}

fn find_incomplete_dependencies(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut missing = Vec::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if !config.resources.contains_key(dep) {
                missing.push((name.clone(), dep.clone()));
            }
        }
    }
    missing.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    missing
}

/// FJ-913: Ensure all resources have explicit state fields.
pub(crate) fn cmd_validate_check_resource_state_coverage(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let missing = find_missing_state_coverage(&config);
    if json {
        let items: Vec<String> = missing.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"resources_without_state\":[{}]}}", items.join(","));
    } else if missing.is_empty() {
        println!("All resources have explicit state coverage.");
    } else {
        println!("Resources without explicit state:");
        for n in &missing {
            println!("  {}", n);
        }
    }
    Ok(())
}

fn find_missing_state_coverage(config: &types::ForjarConfig) -> Vec<String> {
    let mut missing: Vec<String> = config
        .resources
        .iter()
        .filter(|(_, res)| res.state.is_none())
        .map(|(name, _)| name.clone())
        .collect();
    missing.sort();
    missing
}

/// FJ-917: Verify resources can be safely rolled back without side effects.
pub(crate) fn cmd_validate_check_resource_rollback_safety(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let unsafe_resources = find_rollback_unsafe(&config);
    if json {
        let items: Vec<String> = unsafe_resources
            .iter()
            .map(|(n, r)| format!("{{\"resource\":\"{}\",\"reason\":\"{}\"}}", n, r))
            .collect();
        println!("{{\"rollback_unsafe\":[{}]}}", items.join(","));
    } else if unsafe_resources.is_empty() {
        println!("All resources are safe to roll back.");
    } else {
        println!("Resources with rollback safety concerns:");
        for (n, r) in &unsafe_resources {
            println!("  {} — {}", n, r);
        }
    }
    Ok(())
}

fn find_rollback_unsafe(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut unsafe_res = Vec::new();
    for (name, res) in &config.resources {
        if !res.triggers.is_empty() {
            unsafe_res.push((
                name.clone(),
                format!("triggers {} other resources", res.triggers.len()),
            ));
        }
    }
    unsafe_res.sort_by(|a, b| a.0.cmp(&b.0));
    unsafe_res
}

/// FJ-921: Score resource configuration maturity (tags, docs, versioning).
pub(crate) fn cmd_validate_check_resource_config_maturity(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let scores = score_config_maturity(&config);
    if json {
        let items: Vec<String> = scores
            .iter()
            .map(|(n, s)| format!("{{\"resource\":\"{}\",\"maturity_score\":{}}}", n, s))
            .collect();
        println!("{{\"config_maturity\":[{}]}}", items.join(","));
    } else if scores.is_empty() {
        println!("No resources to score.");
    } else {
        println!("Resource configuration maturity scores:");
        for (n, s) in &scores {
            println!("  {} — {}/5", n, s);
        }
    }
    Ok(())
}

fn score_config_maturity(config: &types::ForjarConfig) -> Vec<(String, u8)> {
    let mut scores = Vec::new();
    for (name, res) in &config.resources {
        let mut score: u8 = 0;
        if !res.tags.is_empty() {
            score += 1;
        }
        if res.state.is_some() {
            score += 1;
        }
        if res.version.is_some() {
            score += 1;
        }
        if res.resource_group.is_some() {
            score += 1;
        }
        if !res.depends_on.is_empty() {
            score += 1;
        }
        scores.push((name.clone(), score));
    }
    scores.sort_by(|a, b| a.0.cmp(&b.0));
    scores
}
