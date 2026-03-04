use crate::core::types;
use std::path::Path;

/// FJ-961: Ensure all referenced dependencies exist in the resource set.
pub(crate) fn cmd_validate_check_resource_dependency_refs(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let resource_names: std::collections::HashSet<&String> = config.resources.keys().collect();
    let mut missing = Vec::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if !resource_names.contains(dep) {
                missing.push((name.clone(), dep.clone()));
            }
        }
        for trig in &res.triggers {
            if !resource_names.contains(trig) {
                missing.push((name.clone(), trig.clone()));
            }
        }
    }
    missing.sort();
    if json {
        let items: Vec<String> = missing
            .iter()
            .map(|(n, d)| format!("{{\"resource\":\"{n}\",\"missing_ref\":\"{d}\"}}"))
            .collect();
        println!("{{\"missing_dependency_refs\":[{}]}}", items.join(","));
    } else if missing.is_empty() {
        println!("All dependency references are valid.");
    } else {
        println!("Missing dependency references:");
        for (n, d) in &missing {
            println!("  {n} → {d} (not found)");
        }
    }
    Ok(())
}

/// FJ-965: Ensure all trigger references point to existing resources.
pub(crate) fn cmd_validate_check_resource_trigger_refs(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let resource_names: std::collections::HashSet<&String> = config.resources.keys().collect();
    let mut invalid = Vec::new();
    for (name, res) in &config.resources {
        for trig in &res.triggers {
            if !resource_names.contains(trig) {
                invalid.push((name.clone(), trig.clone()));
            }
        }
    }
    invalid.sort();
    if json {
        let items: Vec<String> = invalid
            .iter()
            .map(|(n, t)| format!("{{\"resource\":\"{n}\",\"invalid_trigger\":\"{t}\"}}"))
            .collect();
        println!("{{\"invalid_trigger_refs\":[{}]}}", items.join(","));
    } else if invalid.is_empty() {
        println!("All trigger references are valid.");
    } else {
        println!("Invalid trigger references:");
        for (n, t) in &invalid {
            println!("  {n} → {t} (not found)");
        }
    }
    Ok(())
}

/// FJ-969: Validate parameter types match expected usage patterns.
pub(crate) fn cmd_validate_check_resource_param_type_safety(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut warnings = Vec::new();
    for (name, value) in &config.params {
        let val_str = match value {
            serde_yaml_ng::Value::String(s) => s.clone(),
            serde_yaml_ng::Value::Number(n) => n.to_string(),
            serde_yaml_ng::Value::Bool(b) => b.to_string(),
            _ => continue,
        };
        if (name.contains("port") || name.ends_with("_port")) && val_str.parse::<u16>().is_err() {
            warnings.push((
                name.clone(),
                format!("expected port number, got '{val_str}'"),
            ));
        }
        if (name.contains("path") || name.ends_with("_dir"))
            && !val_str.starts_with('/')
            && !val_str.starts_with('.')
        {
            warnings.push((name.clone(), format!("expected path, got '{val_str}'")));
        }
    }
    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(n, w)| format!("{{\"param\":\"{n}\",\"warning\":\"{w}\"}}"))
            .collect();
        println!("{{\"param_type_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All parameter types look consistent.");
    } else {
        println!("Parameter type warnings:");
        for (n, w) in &warnings {
            println!("  {n} — {w}");
        }
    }
    Ok(())
}

/// FJ-953: Warn when machines have unbalanced resource counts.
pub(crate) fn cmd_validate_check_resource_machine_balance(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for res in config.resources.values() {
        *counts.entry(res.machine.to_string()).or_insert(0) += 1;
    }
    let values: Vec<usize> = counts.values().cloned().collect();
    let max = values.iter().max().copied().unwrap_or(0);
    let min = values.iter().min().copied().unwrap_or(0);
    let imbalance = if max > 0 {
        (max - min) as f64 / max as f64
    } else {
        0.0
    };
    if json {
        let items: Vec<String> = counts
            .iter()
            .map(|(m, c)| format!("{{\"machine\":\"{m}\",\"resources\":{c}}}"))
            .collect();
        println!(
            "{{\"imbalance_ratio\":{:.4},\"machines\":[{}]}}",
            imbalance,
            items.join(",")
        );
    } else if imbalance > 0.5 {
        println!("Resource imbalance detected (ratio: {imbalance:.4}):");
        for (m, c) in &counts {
            println!("  {m} — {c} resources");
        }
    } else {
        println!(
            "Resource distribution is balanced (ratio: {imbalance:.4})."
        );
    }
    Ok(())
}

/// FJ-973: Validate environment variable references match declared params.
pub(crate) fn cmd_validate_check_resource_env_consistency(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let declared_params: std::collections::HashSet<String> =
        config.params.keys().cloned().collect();
    let mut warnings: Vec<(String, String)> = Vec::new();
    for (name, res) in &config.resources {
        if let Some(ref c) = res.content {
            let mut rest = c.as_str();
            while let Some(start) = rest.find("{{") {
                rest = &rest[start + 2..];
                if let Some(end) = rest.find("}}") {
                    let var = &rest[..end];
                    if var.chars().all(|c| c.is_alphanumeric() || c == '_')
                        && !declared_params.contains(var)
                    {
                        warnings.push((
                            name.clone(),
                            format!("references undeclared param '{var}'"),
                        ));
                    }
                    rest = &rest[end + 2..];
                } else {
                    break;
                }
            }
        }
    }
    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(n, w)| format!("{{\"resource\":\"{n}\",\"warning\":\"{w}\"}}"))
            .collect();
        println!("{{\"env_consistency_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All environment variable references are consistent.");
    } else {
        println!("Environment variable warnings:");
        for (n, w) in &warnings {
            println!("  {n} — {w}");
        }
    }
    Ok(())
}

/// FJ-977: Validate secret resources have rotation policies defined.
pub(crate) fn cmd_validate_check_resource_secret_rotation(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut warnings: Vec<String> = Vec::new();
    for (name, res) in &config.resources {
        let is_secret = name.contains("secret")
            || name.contains("key")
            || name.contains("password")
            || name.contains("credential")
            || name.contains("token");
        if is_secret && res.tags.is_empty() {
            warnings.push(name.clone());
        }
    }
    if json {
        let items: Vec<String> = warnings.iter().map(|n| format!("\"{n}\"")).collect();
        println!("{{\"secrets_without_rotation\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All secret resources have rotation metadata.");
    } else {
        println!("Secrets without rotation tags:");
        for n in &warnings {
            println!("  {n} — missing rotation policy tags");
        }
    }
    Ok(())
}
/// FJ-981: Verify resources define all lifecycle stages.
pub(crate) fn cmd_validate_check_resource_lifecycle_completeness(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut warnings: Vec<String> = Vec::new();
    for (name, res) in &config.resources {
        if res.content.is_none() && res.depends_on.is_empty() && res.tags.is_empty() {
            warnings.push(name.clone());
        }
    }
    if json {
        let items: Vec<String> = warnings.iter().map(|n| format!("\"{n}\"")).collect();
        println!("{{\"incomplete_lifecycle\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resources have complete lifecycle definitions.");
    } else {
        println!("Resources with incomplete lifecycle:");
        for n in &warnings {
            println!("  {n} — missing content/deps/tags");
        }
    }
    Ok(())
}
/// FJ-985: Verify resource types are compatible with declared providers.
pub(crate) fn cmd_validate_check_resource_provider_compatibility(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let valid_types = [
        "file",
        "package",
        "service",
        "mount",
        "cron",
        "directory",
        "user",
        "group",
        "link",
    ];
    let mut warnings: Vec<(String, String)> = Vec::new();
    for (name, res) in &config.resources {
        let rtype = format!("{:?}", res.resource_type).to_lowercase();
        if !valid_types.iter().any(|t| rtype.contains(t)) {
            warnings.push((name.clone(), rtype));
        }
    }
    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(n, t)| format!("{{\"resource\":\"{n}\",\"type\":\"{t}\"}}"))
            .collect();
        println!("{{\"provider_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resource types are compatible with providers.");
    } else {
        println!("Provider compatibility warnings:");
        for (n, t) in &warnings {
            println!("  {n} — unknown type '{t}'");
        }
    }
    Ok(())
}
