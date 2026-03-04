//! Linting.

use super::helpers::*;
use crate::core::{codegen, types};
use std::path::Path;

fn lint_unused_machines(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut referenced = std::collections::HashSet::new();
    for resource in config.resources.values() {
        for m in resource.machine.to_vec() {
            referenced.insert(m);
        }
    }
    for machine_name in config.machines.keys() {
        if !referenced.contains(machine_name) {
            warnings.push(format!(
                "machine '{machine_name}' is defined but not referenced by any resource"
            ));
        }
    }
    warnings
}

fn lint_untagged_resources(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut untagged = 0usize;
    for (id, resource) in &config.resources {
        if resource.tags.is_empty() {
            untagged += 1;
            if config.resources.len() > 3 {
                warnings.push(format!("resource '{id}' has no tags"));
            }
        }
    }
    if untagged > 0 && config.resources.len() > 3 && untagged == config.resources.len() {
        warnings.retain(|w| !w.starts_with("resource '") || !w.ends_with("has no tags"));
        warnings.push(format!(
            "all {untagged} resources have no tags — consider adding tags for selective filtering"
        ));
    }
    warnings
}

fn lint_duplicate_content(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut content_map: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for (id, resource) in &config.resources {
        if let Some(ref content) = resource.content {
            content_map
                .entry(content.as_str())
                .or_default()
                .push(id.as_str());
        }
    }
    for ids in content_map.values() {
        if ids.len() > 1 {
            warnings.push(format!(
                "resources {} have identical content — consider using a recipe or template",
                ids.join(", ")
            ));
        }
    }
    warnings
}

fn lint_dependency_issues(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (id, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                warnings.push(format!(
                    "resource '{id}' depends on '{dep}' which does not exist"
                ));
            }
        }
        let my_machines: std::collections::HashSet<String> =
            resource.machine.to_vec().into_iter().collect();
        for dep in &resource.depends_on {
            if let Some(dep_resource) = config.resources.get(dep) {
                let dep_machines: std::collections::HashSet<String> =
                    dep_resource.machine.to_vec().into_iter().collect();
                if my_machines.is_disjoint(&dep_machines) {
                    warnings.push(format!(
                        "resource '{id}' depends on '{dep}' but they target different machines"
                    ));
                }
            }
        }
    }
    warnings
}

fn lint_empty_packages(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (id, resource) in &config.resources {
        if resource.resource_type == types::ResourceType::Package && resource.packages.is_empty() {
            warnings.push(format!("package resource '{id}' has no packages listed"));
        }
    }
    warnings
}

fn lint_strict_rules(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for (id, resource) in &config.resources {
        if resource.resource_type == types::ResourceType::File
            && resource.owner.as_deref() == Some("root")
            && !resource.tags.iter().any(|t| t == "system")
        {
            warnings.push(format!(
                "strict: file '{id}' is owned by root — tag as 'system' or use a non-root owner"
            ));
        }
    }
    for (id, resource) in &config.resources {
        if resource.tags.is_empty() {
            warnings.push(format!("strict: resource '{id}' has no tags"));
        }
    }
    for (name, machine) in &config.machines {
        if let Some(ref container) = machine.container {
            if container.privileged {
                warnings.push(format!(
                    "strict: machine '{name}' uses privileged container mode"
                ));
            }
        }
    }
    for (name, machine) in &config.machines {
        if machine.addr != "127.0.0.1"
            && machine.addr != "localhost"
            && machine.addr != "container"
            && machine.ssh_key.is_none()
        {
            warnings.push(format!(
                "strict: machine '{name}' has no ssh_key configured"
            ));
        }
    }
    warnings
}

fn lint_scripts(config: &types::ForjarConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut script_errors = 0usize;
    let mut script_warnings_count = 0usize;
    for (id, resource) in &config.resources {
        for (kind, result) in [
            ("check", codegen::check_script(resource)),
            ("apply", codegen::apply_script(resource)),
            ("state_query", codegen::state_query_script(resource)),
        ] {
            if let Ok(script) = result {
                let lint_result = crate::core::purifier::lint_script(&script);
                for d in &lint_result.diagnostics {
                    use bashrs::linter::Severity;
                    match d.severity {
                        Severity::Error => {
                            script_errors += 1;
                            warnings.push(format!(
                                "bashrs: {}/{} [{}] {}",
                                id, kind, d.code, d.message
                            ));
                        }
                        _ => {
                            script_warnings_count += 1;
                        }
                    }
                }
            }
        }
    }
    if script_errors > 0 || script_warnings_count > 0 {
        warnings.push(format!(
            "bashrs script lint: {} error(s), {} warning(s) across {} resources",
            script_errors,
            script_warnings_count,
            config.resources.len()
        ));
    }
    warnings
}

pub(crate) fn lint_auto_fix(file: &Path) -> Result<Vec<String>, String> {
    let mut fixes_applied = Vec::new();
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {}", file.display(), e))?;
    let mut doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("YAML parse error: {e}"))?;
    if let Some(serde_yaml_ng::Value::Mapping(map)) = doc.get_mut("resources") {
        let mut pairs: Vec<_> = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        pairs.sort_by(|(a, _), (b, _)| a.as_str().unwrap_or("").cmp(b.as_str().unwrap_or("")));
        *map = serde_yaml_ng::Mapping::new();
        for (k, v) in pairs {
            map.insert(k, v);
        }
        fixes_applied.push("sorted resource keys alphabetically".to_string());
    }
    if !fixes_applied.is_empty() {
        let normalized =
            serde_yaml_ng::to_string(&doc).map_err(|e| format!("serialization error: {e}"))?;
        std::fs::write(file, normalized)
            .map_err(|e| format!("cannot write {}: {}", file.display(), e))?;
    }
    Ok(fixes_applied)
}

pub(crate) fn cmd_lint(file: &Path, json: bool, strict: bool, fix: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let mut warnings: Vec<String> = Vec::new();
    warnings.extend(lint_unused_machines(&config));
    warnings.extend(lint_untagged_resources(&config));
    warnings.extend(lint_duplicate_content(&config));
    warnings.extend(lint_dependency_issues(&config));
    warnings.extend(lint_empty_packages(&config));
    if strict {
        warnings.extend(lint_strict_rules(&config));
    }
    warnings.extend(lint_scripts(&config));

    if json {
        let report = serde_json::json!({
            "warnings": warnings.len(),
            "findings": warnings,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{output}");
    } else if warnings.is_empty() {
        println!("No lint warnings found.");
    } else {
        for w in &warnings {
            println!("  warn: {w}");
        }
        if fix {
            let fixes = lint_auto_fix(file)?;
            for f in &fixes {
                println!("  {}: {}", green("fixed"), f);
            }
            if !fixes.is_empty() {
                println!("Wrote normalized config to {}", file.display());
            }
        }
        println!();
        println!("Lint: {} warning(s)", warnings.len());
    }

    Ok(())
}
