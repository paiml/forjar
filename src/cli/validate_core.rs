//! Core validation command.

use super::helpers::*;
use crate::core::{resolver, types};
use std::path::Path;

/// Check machine references exist.
fn check_machine_refs(config: &types::ForjarConfig, errors: &mut Vec<String>) {
    for (id, resource) in &config.resources {
        for machine_name in resource.machine.to_vec() {
            if !config.machines.contains_key(&machine_name) {
                errors.push(format!(
                    "{id}: references undefined machine '{machine_name}'"
                ));
            }
        }
    }
}

/// Check depends_on targets exist.
fn check_deps_exist(config: &types::ForjarConfig, errors: &mut Vec<String>) {
    for (id, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                errors.push(format!("{id}: depends_on '{dep}' does not exist"));
            }
        }
    }
}

/// Check that file resource paths are absolute.
fn check_paths_absolute(config: &types::ForjarConfig, errors: &mut Vec<String>) {
    for (id, resource) in &config.resources {
        if let Some(ref path) = resource.path {
            if !path.starts_with('/') && !path.starts_with("{{") {
                errors.push(format!("{id}: path '{path}' is not absolute"));
            }
        }
    }
}

/// Check that template vars resolve.
fn check_templates_resolve(config: &types::ForjarConfig, errors: &mut Vec<String>) {
    // Inject data source defaults so {{data.*}} templates resolve during validation
    let mut params = config.params.clone();
    for (key, ds) in &config.data {
        let val = ds.default.clone().unwrap_or_default();
        params.insert(
            format!("__data__{key}"),
            serde_yaml_ng::Value::String(val),
        );
    }
    for (id, resource) in &config.resources {
        if let Err(e) =
            resolver::resolve_resource_templates(resource, &params, &config.machines)
        {
            errors.push(format!("{id}: template error: {e}"));
        }
    }
}

/// Warn on unused params.
fn check_unused_params(config: &types::ForjarConfig, errors: &mut Vec<String>) {
    let mut used_params = std::collections::HashSet::new();
    // Serialize resources and machines to search for param references
    let mut haystack = String::new();
    for resource in config.resources.values() {
        haystack.push_str(&serde_yaml_ng::to_string(resource).unwrap_or_default());
    }
    for machine in config.machines.values() {
        haystack.push_str(&serde_yaml_ng::to_string(machine).unwrap_or_default());
    }
    for key in config.params.keys() {
        if haystack.contains(&format!("params.{key}")) {
            used_params.insert(key.clone());
        }
    }
    for key in config.params.keys() {
        if !used_params.contains(key) {
            errors.push(format!("param '{key}' is defined but never referenced"));
        }
    }
}

/// Run strict validation checks, collecting errors.
fn run_strict_checks(config: &types::ForjarConfig) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    check_machine_refs(config, &mut errors);
    check_deps_exist(config, &mut errors);
    if let Err(e) = resolver::build_execution_order(config) {
        errors.push(format!("dependency cycle: {e}"));
    }
    check_paths_absolute(config, &mut errors);
    check_templates_resolve(config, &mut errors);
    check_unused_params(config, &mut errors);
    if config.description.is_none() {
        errors.push("project has no description field".to_string());
    }
    errors
}

pub(crate) fn cmd_validate(
    file: &Path,
    strict: bool,
    json: bool,
    dry_expand: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Always detect circular dependencies — a cycle makes the config unusable
    if let Err(e) = resolver::build_execution_order(&config) {
        return Err(format!("dependency cycle: {e}"));
    }

    // FJ-330: Show fully expanded config after template resolution
    if dry_expand {
        let mut expanded = config.clone();
        for (_id, resource) in expanded.resources.iter_mut() {
            *resource = resolver::resolve_resource_templates(
                resource,
                &expanded.params,
                &expanded.machines,
            )?;
        }
        let yaml =
            serde_yaml_ng::to_string(&expanded).map_err(|e| format!("serialization error: {e}"))?;
        println!("{yaml}");
        return Ok(());
    }

    let errors = if strict {
        run_strict_checks(&config)
    } else {
        Vec::new()
    };

    let valid = errors.is_empty();

    if json {
        let output = serde_json::json!({
            "valid": valid,
            "name": config.name,
            "machines": config.machines.len(),
            "resources": config.resources.len(),
            "strict": strict,
            "errors": errors,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {e}"))?
        );
        if !valid {
            return Err(format!(
                "strict validation failed: {} error(s)",
                errors.len()
            ));
        }
    } else {
        if !valid {
            for e in &errors {
                eprintln!("  {}", red(e));
            }
            return Err(format!(
                "strict validation failed: {} error(s)",
                errors.len()
            ));
        }
        println!(
            "OK: {} ({} machines, {} resources)",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
    }

    Ok(())
}

// ── FJ-391: validate --exhaustive ──

/// Find unresolved param references in resource content.
pub(crate) fn find_unresolved_content_params_silent(
    name: &str,
    content: &str,
    params: &std::collections::HashMap<String, serde_yaml_ng::Value>,
    issues: &mut Vec<String>,
) {
    find_unresolved_content_params(name, content, params, issues);
}
fn find_unresolved_content_params(
    name: &str,
    content: &str,
    params: &std::collections::HashMap<String, serde_yaml_ng::Value>,
    issues: &mut Vec<String>,
) {
    let mut start = 0;
    while let Some(pos) = content[start..].find("{{params.") {
        let abs_pos = start + pos + 9;
        if let Some(end) = content[abs_pos..].find("}}") {
            let key = &content[abs_pos..abs_pos + end];
            if !params.contains_key(key) {
                issues.push(format!(
                    "resource '{name}' references unknown param '{key}'"
                ));
            }
            start = abs_pos + end + 2;
        } else {
            break;
        }
    }
}

/// Check resource references: machines and dependencies.
pub(crate) fn check_resource_refs_silent(config: &types::ForjarConfig, issues: &mut Vec<String>) {
    check_resource_refs(config, issues);
}
fn check_resource_refs(config: &types::ForjarConfig, issues: &mut Vec<String>) {
    for (name, res) in &config.resources {
        if let types::MachineTarget::Single(ref m) = res.machine {
            if !config.machines.contains_key(m) {
                issues.push(format!(
                    "resource '{name}' references unknown machine '{m}'"
                ));
            }
        }
        for dep in &res.depends_on {
            if !config.resources.contains_key(dep) {
                issues.push(format!(
                    "resource '{name}' depends on unknown resource '{dep}'"
                ));
            }
        }
    }
}

/// Check for orphaned params (defined but never used).
pub(crate) fn check_orphaned_params_silent(config: &types::ForjarConfig, issues: &mut Vec<String>) {
    check_orphaned_params(config, issues);
}
fn check_orphaned_params(config: &types::ForjarConfig, issues: &mut Vec<String>) {
    for param_key in config.params.keys() {
        let yaml_str = serde_yaml_ng::to_string(config).unwrap_or_default();
        let needle = format!("{{{{params.{param_key}}}}}");
        if !yaml_str.contains(&needle) {
            issues.push(format!(
                "param '{param_key}' is defined but never referenced"
            ));
        }
    }
}

pub(crate) fn cmd_validate_exhaustive(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut issues: Vec<String> = Vec::new();

    check_resource_refs(&config, &mut issues);

    // Check params referenced in templates exist
    for (name, res) in &config.resources {
        if let Some(ref content) = res.content {
            find_unresolved_content_params(name, content, &config.params, &mut issues);
        }
    }

    check_orphaned_params(&config, &mut issues);

    if json {
        println!(
            "{{\"valid\":{},\"issues\":{},\"issue_count\":{}}}",
            issues.is_empty(),
            serde_json::to_string(&issues).unwrap_or_else(|_| "[]".to_string()),
            issues.len()
        );
    } else if issues.is_empty() {
        println!("{} Exhaustive validation passed", green("✓"));
    } else {
        println!(
            "{} Exhaustive validation found {} issue(s):",
            red("✗"),
            issues.len()
        );
        for issue in &issues {
            println!("  {} {}", red("•"), issue);
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(format!("{} validation issue(s) found", issues.len()))
    }
}

// cmd_validate_deep moved to validate_deep.rs
