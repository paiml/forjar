//! Ownership & quality validation — naming conventions, idempotency, documentation, secrets, tags.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-869: Enforce naming patterns across resources.
pub(crate) fn cmd_validate_check_resource_naming_convention(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let violations = find_naming_convention_violations(&config);
    if json {
        let items: Vec<String> = violations.iter()
            .map(|(n, reason)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", n, reason)).collect();
        println!("{{\"naming_convention_violations\":[{}]}}", items.join(","));
    } else if violations.is_empty() {
        println!("All resources follow naming conventions.");
    } else {
        println!("Naming convention violations:");
        for (n, reason) in &violations { println!("  {} — {}", n, reason); }
    }
    Ok(())
}

fn find_naming_convention_violations(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut violations = Vec::new();
    for name in config.resources.keys() {
        if name.chars().any(|c| c.is_uppercase()) {
            violations.push((name.clone(), "contains uppercase characters".to_string()));
        } else if name.contains(' ') {
            violations.push((name.clone(), "contains spaces".to_string()));
        } else if name.starts_with('-') || name.ends_with('-') {
            violations.push((name.clone(), "starts or ends with hyphen".to_string()));
        } else if name.contains("__") {
            violations.push((name.clone(), "contains double underscore".to_string()));
        }
    }
    violations.sort_by(|a, b| a.0.cmp(&b.0));
    violations
}

/// FJ-873: Verify resources are idempotent-safe.
pub(crate) fn cmd_validate_check_resource_idempotency(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let warnings = find_idempotency_concerns(&config);
    if json {
        let items: Vec<String> = warnings.iter()
            .map(|(n, reason)| format!("{{\"resource\":\"{}\",\"concern\":\"{}\"}}", n, reason)).collect();
        println!("{{\"idempotency_concerns\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resources appear idempotent-safe.");
    } else {
        println!("Idempotency concerns:");
        for (n, reason) in &warnings { println!("  {} — {}", n, reason); }
    }
    Ok(())
}

fn find_idempotency_concerns(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut concerns = Vec::new();
    for (name, resource) in &config.resources {
        if let Some(ref content) = resource.content {
            if content.contains("$(date") || content.contains("$(hostname") {
                concerns.push((name.clone(), "content uses dynamic shell substitution".to_string()));
            }
        }
        if let Some(ref st) = resource.state {
            if st == "absent" && !resource.triggers.is_empty() {
                concerns.push((name.clone(), "absent resource has triggers".to_string()));
            }
        }
    }
    concerns.sort_by(|a, b| a.0.cmp(&b.0));
    concerns
}

/// FJ-877: Check resource documentation — ensure resources have descriptions or comments.
pub(crate) fn cmd_validate_check_resource_documentation(file: &Path, json: bool) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| e.to_string())?;
    let undocumented = find_undocumented_resources(&config);
    if json {
        let items: Vec<String> = undocumented.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"undocumented_resources\":[{}]}}", items.join(","));
    } else if undocumented.is_empty() {
        println!("All resources have documentation.");
    } else {
        println!("Resources missing documentation:");
        for name in &undocumented { println!("  {}", name); }
    }
    Ok(())
}

fn find_undocumented_resources(config: &types::ForjarConfig) -> Vec<String> {
    let mut missing: Vec<String> = config.resources.keys()
        .filter(|name| {
            let r = &config.resources[*name];
            r.tags.is_empty() && r.content.is_none()
        })
        .cloned().collect();
    missing.sort();
    missing
}

/// FJ-881: Check resource ownership — ensure resources have owner tags or group assignment.
pub(crate) fn cmd_validate_check_resource_ownership(file: &Path, json: bool) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| e.to_string())?;
    let unowned = find_unowned_resources(&config);
    if json {
        let items: Vec<String> = unowned.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"unowned_resources\":[{}]}}", items.join(","));
    } else if unowned.is_empty() {
        println!("All resources have ownership assigned.");
    } else {
        println!("Resources missing ownership (no tags or resource_group):");
        for name in &unowned { println!("  {}", name); }
    }
    Ok(())
}

fn find_unowned_resources(config: &types::ForjarConfig) -> Vec<String> {
    let mut missing: Vec<String> = config.resources.keys()
        .filter(|name| {
            let r = &config.resources[*name];
            r.tags.is_empty() && r.resource_group.is_none()
        })
        .cloned().collect();
    missing.sort();
    missing
}

/// FJ-885: Detect secrets accidentally exposed in resource content.
pub(crate) fn cmd_validate_check_resource_secret_exposure(file: &Path, json: bool) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| e.to_string())?;
    let exposures = find_secret_exposures(&config);
    if json {
        let items: Vec<String> = exposures.iter()
            .map(|(n, reason)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", n, reason)).collect();
        println!("{{\"secret_exposures\":[{}]}}", items.join(","));
    } else if exposures.is_empty() {
        println!("No secret exposures detected.");
    } else {
        println!("Potential secret exposures:");
        for (n, reason) in &exposures { println!("  {} — {}", n, reason); }
    }
    Ok(())
}

fn find_secret_exposures(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let patterns = ["password", "secret", "api_key", "apikey", "token", "private_key"];
    let mut exposures = Vec::new();
    for (name, resource) in &config.resources {
        if let Some(ref content) = resource.content {
            let lower = content.to_lowercase();
            for pat in &patterns {
                if lower.contains(pat) {
                    exposures.push((name.clone(), format!("content may contain '{}'", pat)));
                    break;
                }
            }
        }
    }
    exposures.sort_by(|a, b| a.0.cmp(&b.0));
    exposures
}

/// FJ-889: Enforce tag naming standards across resources.
pub(crate) fn cmd_validate_check_resource_tag_standards(file: &Path, json: bool) -> Result<(), String> {
    let raw = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| e.to_string())?;
    let violations = find_tag_standard_violations(&config);
    if json {
        let items: Vec<String> = violations.iter()
            .map(|(n, tag, reason)| format!("{{\"resource\":\"{}\",\"tag\":\"{}\",\"issue\":\"{}\"}}", n, tag, reason)).collect();
        println!("{{\"tag_standard_violations\":[{}]}}", items.join(","));
    } else if violations.is_empty() {
        println!("All resource tags follow naming standards.");
    } else {
        println!("Tag naming standard violations:");
        for (n, tag, reason) in &violations { println!("  {} tag '{}' — {}", n, tag, reason); }
    }
    Ok(())
}

fn find_tag_standard_violations(config: &types::ForjarConfig) -> Vec<(String, String, String)> {
    let mut violations = Vec::new();
    for (name, resource) in &config.resources {
        for tag in &resource.tags {
            if tag.chars().any(|c| c.is_uppercase()) {
                violations.push((name.clone(), tag.clone(), "contains uppercase".to_string()));
            } else if tag.contains(' ') {
                violations.push((name.clone(), tag.clone(), "contains spaces".to_string()));
            }
        }
    }
    violations.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    violations
}

/// FJ-893: Detect resources that could enable privilege escalation.
pub(crate) fn cmd_validate_check_resource_privilege_escalation(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let risks = find_privilege_escalation_risks(&cfg);
    if json {
        let items: Vec<String> = risks.iter()
            .map(|(n, r)| format!("{{\"resource\":\"{}\",\"risk\":\"{}\"}}", n, r))
            .collect();
        println!("{{\"privilege_escalation_risks\":[{}]}}", items.join(","));
    } else if risks.is_empty() {
        println!("No privilege escalation risks detected.");
    } else {
        println!("Privilege escalation risks:");
        for (n, r) in &risks { println!("  {} — {}", n, r); }
    }
    Ok(())
}

fn find_privilege_escalation_risks(cfg: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut risks = Vec::new();
    let priv_patterns = ["chmod +s", "setuid", "setgid", "sudoers", "NOPASSWD", "cap_sys_admin"];
    for (name, res) in &cfg.resources {
        if let Some(ref content) = res.content {
            for pat in &priv_patterns {
                if content.contains(pat) {
                    risks.push((name.clone(), format!("contains '{}'", pat)));
                }
            }
        }
        if let Some(ref path) = res.path {
            let p = path.to_lowercase();
            if p.contains("sudoers") || p.contains("/etc/shadow") {
                risks.push((name.clone(), format!("targets sensitive path '{}'", path)));
            }
        }
    }
    risks.sort_by(|a, b| a.0.cmp(&b.0));
    risks
}

/// FJ-897: Verify resources can be safely updated without downtime.
pub(crate) fn cmd_validate_check_resource_update_safety(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let warnings = find_update_safety_issues(&cfg);
    if json {
        let items: Vec<String> = warnings.iter()
            .map(|(n, w)| format!("{{\"resource\":\"{}\",\"warning\":\"{}\"}}", n, w))
            .collect();
        println!("{{\"update_safety_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resources can be safely updated.");
    } else {
        println!("Update safety warnings:");
        for (n, w) in &warnings { println!("  {} — {}", n, w); }
    }
    Ok(())
}

fn find_update_safety_issues(cfg: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut warnings = Vec::new();
    for (name, res) in &cfg.resources {
        if matches!(res.resource_type, types::ResourceType::Service) && !res.triggers.is_empty() {
            warnings.push((name.clone(), "service with triggers may cause cascade restart".to_string()));
        }
        if matches!(res.resource_type, types::ResourceType::Mount) {
            warnings.push((name.clone(), "mount changes require unmount/remount".to_string()));
        }
    }
    warnings.sort_by(|a, b| a.0.cmp(&b.0));
    warnings
}

/// FJ-901: Detect config inconsistencies across machines.
pub(crate) fn cmd_validate_check_resource_cross_machine_consistency(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let issues = find_cross_machine_inconsistencies(&config);
    if json {
        let items: Vec<String> = issues.iter()
            .map(|(n, i)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", n, i))
            .collect();
        println!("{{\"cross_machine_inconsistencies\":[{}]}}", items.join(","));
    } else if issues.is_empty() {
        println!("No cross-machine inconsistencies found.");
    } else {
        println!("Cross-machine inconsistencies:");
        for (n, i) in &issues { println!("  {} — {}", n, i); }
    }
    Ok(())
}

fn find_cross_machine_inconsistencies(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut type_by_name: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for (name, res) in &config.resources {
        let t = format!("{:?}", res.resource_type);
        type_by_name.entry(name.clone()).or_default().push(t);
    }
    let mut issues = Vec::new();
    for (name, types_list) in &type_by_name {
        if types_list.len() > 1 {
            let unique: std::collections::HashSet<&String> = types_list.iter().collect();
            if unique.len() > 1 {
                issues.push((name.clone(), format!("mixed types: {}", types_list.join(", "))));
            }
        }
    }
    issues.sort_by(|a, b| a.0.cmp(&b.0));
    issues
}

/// FJ-905: Verify resources pin explicit versions.
pub(crate) fn cmd_validate_check_resource_version_pinning(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let unpinned = find_unpinned_resources(&config);
    if json {
        let items: Vec<String> = unpinned.iter()
            .map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"unpinned_resources\":[{}]}}", items.join(","));
    } else if unpinned.is_empty() {
        println!("All package resources have pinned versions.");
    } else {
        println!("Resources without pinned versions:");
        for n in &unpinned { println!("  {}", n); }
    }
    Ok(())
}

fn find_unpinned_resources(config: &types::ForjarConfig) -> Vec<String> {
    let mut unpinned: Vec<String> = config.resources.iter()
        .filter(|(_, res)| matches!(res.resource_type, types::ResourceType::Package) && res.version.is_none())
        .map(|(name, _)| name.clone())
        .collect();
    unpinned.sort();
    unpinned
}
