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
    for (id, resource) in &config.resources {
        if let Err(e) =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)
        {
            errors.push(format!("{id}: template error: {e}"));
        }
    }
}

/// Warn on unused params.
fn check_unused_params(config: &types::ForjarConfig, errors: &mut Vec<String>) {
    let mut used_params = std::collections::HashSet::new();
    for resource in config.resources.values() {
        let yaml = serde_yaml_ng::to_string(resource).unwrap_or_default();
        for key in config.params.keys() {
            if yaml.contains(&format!("params.{key}")) {
                used_params.insert(key.clone());
            }
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

// ── FJ-2503: validate --deep ──

type CheckFn = fn(&Path, bool) -> Result<(), String>;

/// Run all deep validation checks and aggregate results.
pub(crate) fn cmd_validate_deep(file: &Path, json: bool) -> Result<(), String> {
    use super::validate_quality::*;
    use super::validate_structural::*;

    // First run the base validation
    let config = parse_and_validate(file)?;

    let checks: &[(&str, CheckFn)] = &[
        ("templates", cmd_validate_check_templates),
        ("overlaps", cmd_validate_check_overlaps),
        ("circular-deps", cmd_validate_check_cycles_deep),
        ("secrets", cmd_validate_check_secrets),
        ("naming", cmd_validate_check_naming),
        ("drift-coverage", cmd_validate_check_drift_coverage),
        ("idempotency", cmd_validate_check_idempotency),
    ];

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut failures: Vec<(String, String)> = Vec::new();

    if !json {
        println!(
            "=== Deep Validation: {} ({} machines, {} resources) ===",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
        println!();
    }

    for (name, check_fn) in checks {
        match check_fn(file, false) {
            Ok(()) => {
                passed += 1;
            }
            Err(e) => {
                failed += 1;
                failures.push((name.to_string(), e));
            }
        }
        if !json {
            println!();
        }
    }

    // Also run exhaustive cross-reference check
    match cmd_validate_exhaustive(file, false) {
        Ok(()) => passed += 1,
        Err(e) => {
            failed += 1;
            failures.push(("exhaustive".to_string(), e));
        }
    }

    if json {
        let result = serde_json::json!({
            "deep_validation": {
                "passed": passed,
                "failed": failed,
                "total": passed + failed,
                "failures": failures.iter().map(|(n, e)| {
                    serde_json::json!({"check": n, "error": e})
                }).collect::<Vec<_>>(),
            }
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else {
        println!("─────────────────────────────────────");
        println!(
            "Deep validation: {}/{} checks passed",
            passed,
            passed + failed
        );
        if !failures.is_empty() {
            for (name, _) in &failures {
                println!("  {} {name}", red("✗"));
            }
        }
    }

    if failed > 0 {
        Err(format!("{failed} deep validation check(s) failed"))
    } else {
        Ok(())
    }
}
