//! Show, explain, compare, template.

use super::helpers::*;
use crate::core::{codegen, parser, resolver, types};
use std::path::Path;

/// Strip null values, empty arrays, empty maps, and false booleans from a JSON value.
pub(crate) fn strip_defaults(val: &mut serde_json::Value) {
    match val {
        serde_json::Value::Object(map) => {
            map.retain(|_, v| {
                !v.is_null()
                    && *v != serde_json::Value::Bool(false)
                    && *v != serde_json::json!([])
                    && *v != serde_json::json!({})
            });
            for v in map.values_mut() {
                strip_defaults(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                strip_defaults(v);
            }
        }
        _ => {}
    }
}

pub(crate) fn cmd_show(
    file: &Path,
    resource_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let mut config = parse_and_validate(file)?;

    // Resolve templates in all resources
    for (_id, resource) in config.resources.iter_mut() {
        *resource =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;
    }

    if let Some(resource_id) = resource_filter {
        let resource = config
            .resources
            .get(resource_id)
            .ok_or_else(|| format!("resource '{resource_id}' not found"))?;
        if json {
            let mut val = serde_json::to_value(resource).map_err(|e| format!("JSON error: {e}"))?;
            strip_defaults(&mut val);
            println!(
                "{}",
                serde_json::to_string_pretty(&val).map_err(|e| format!("JSON error: {e}"))?
            );
        } else {
            let output =
                serde_yaml_ng::to_string(resource).map_err(|e| format!("YAML error: {e}"))?;
            println!("{resource_id}:\n{output}");
        }
    } else if json {
        let mut val = serde_json::to_value(&config).map_err(|e| format!("JSON error: {e}"))?;
        strip_defaults(&mut val);
        println!(
            "{}",
            serde_json::to_string_pretty(&val).map_err(|e| format!("JSON error: {e}"))?
        );
    } else {
        let output = serde_yaml_ng::to_string(&config).map_err(|e| format!("YAML error: {e}"))?;
        println!("{output}");
    }

    Ok(())
}

/// Detect the transport type for a machine by name and config.
fn detect_transport_type(
    machine_name: &str,
    machines: &indexmap::IndexMap<String, types::Machine>,
) -> &'static str {
    if machine_name == "local" || machine_name == "localhost" {
        return "local";
    }
    if let Some(m) = machines.get(machine_name) {
        if m.is_container_transport() {
            return "container";
        }
        if m.addr == "127.0.0.1" || m.addr == "localhost" {
            return "local";
        }
    }
    "ssh"
}

/// Build JSON output for cmd_explain.
#[allow(clippy::too_many_arguments)]
fn explain_json(
    resource_id: &str,
    resource: &types::Resource,
    machine_name: &str,
    transport_type: &str,
    apply_script: &Option<String>,
    check_script: &Option<String>,
    machines: &indexmap::IndexMap<String, types::Machine>,
) -> Result<(), String> {
    let mut info = serde_json::json!({
        "resource": resource_id,
        "type": resource.resource_type,
        "machine": machine_name,
        "transport": transport_type,
        "depends_on": resource.depends_on,
        "tags": resource.tags,
    });
    if let Some(ref rg) = resource.resource_group {
        info["resource_group"] = serde_json::json!(rg);
    }
    if let Some(ref script) = apply_script {
        info["apply_script"] = serde_json::json!(script);
    }
    if let Some(ref script) = check_script {
        info["check_script"] = serde_json::json!(script);
    }
    if let Some(m) = machines.get(machine_name) {
        info["addr"] = serde_json::json!(m.addr);
        if let Some(ref key) = m.ssh_key {
            info["ssh_key"] = serde_json::json!(key);
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&info).map_err(|e| format!("JSON error: {e}"))?
    );
    Ok(())
}

/// FJ-271: Explain a resource — show raw YAML, resolved templates, codegen script, transport.
pub(crate) fn cmd_explain(file: &Path, resource_id: &str, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let resource = config
        .resources
        .get(resource_id)
        .ok_or_else(|| format!("resource '{resource_id}' not found"))?;

    let machine_name = match &resource.machine {
        types::MachineTarget::Single(m) => m.clone(),
        types::MachineTarget::Multiple(ms) => ms.first().cloned().unwrap_or_default(),
    };
    let resolved =
        resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;

    let transport_type = detect_transport_type(&machine_name, &config.machines);

    let apply_script = codegen::apply_script(&resolved).ok();
    let check_script = codegen::check_script(&resolved).ok();

    if json {
        return explain_json(
            resource_id,
            resource,
            &machine_name,
            transport_type,
            &apply_script,
            &check_script,
            &config.machines,
        );
    }

    // Text output
    let raw_yaml =
        serde_yaml_ng::to_string(resource).map_err(|e| format!("serialize error: {e}"))?;
    println!("{}", bold("1. Raw Resource Definition"));
    println!("{}", dim("─────────────────────────────"));
    println!("{raw_yaml}");

    println!("{}", bold("2. After Template Resolution"));
    println!("{}", dim("─────────────────────────────"));
    let resolved_yaml =
        serde_yaml_ng::to_string(&resolved).map_err(|e| format!("serialize error: {e}"))?;
    println!("{resolved_yaml}");

    println!("{}", bold("3. Generated Shell Script"));
    println!("{}", dim("─────────────────────────────"));
    match apply_script {
        Some(ref script) => println!("{script}"),
        None => println!("{}", red("codegen error")),
    }

    println!("{}", bold("4. Transport"));
    println!("{}", dim("─────────────────────────────"));
    println!("machine: {machine_name}");
    println!("transport: {transport_type}");
    if let Some(m) = config.machines.get(&machine_name) {
        println!("addr: {}", m.addr);
        if let Some(ref key) = m.ssh_key {
            println!("ssh_key: {key}");
        }
    }

    if !resource.depends_on.is_empty() {
        println!();
        println!("{}", bold("5. Dependencies"));
        println!("{}", dim("─────────────────────────────"));
        for dep in &resource.depends_on {
            println!("  → {dep}");
        }
    }

    Ok(())
}

// FJ-363: Compare two config files
pub(crate) fn cmd_compare(file1: &Path, file2: &Path, json: bool) -> Result<(), String> {
    let config1 = parse_and_validate(file1)?;
    let config2 = parse_and_validate(file2)?;

    let keys1: std::collections::HashSet<&String> = config1.resources.keys().collect();
    let keys2: std::collections::HashSet<&String> = config2.resources.keys().collect();

    let only_in_1: Vec<&&String> = keys1.difference(&keys2).collect();
    let only_in_2: Vec<&&String> = keys2.difference(&keys1).collect();
    let in_both: Vec<&&String> = keys1.intersection(&keys2).collect();

    let mut changed = Vec::new();
    for key in &in_both {
        let r1 = &config1.resources[**key];
        let r2 = &config2.resources[**key];
        // Compare by hashing the serialized forms
        let s1 = format!("{r1:?}");
        let s2 = format!("{r2:?}");
        if s1 != s2 {
            changed.push(**key);
        }
    }

    if json {
        let result = serde_json::json!({
            "only_in_first": only_in_1,
            "only_in_second": only_in_2,
            "changed": changed,
            "unchanged": in_both.len() - changed.len(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Comparing {} vs {}\n", file1.display(), file2.display());
        for k in &only_in_1 {
            println!("  {} {} (only in {})", red("-"), k, file1.display());
        }
        for k in &only_in_2 {
            println!("  {} {} (only in {})", green("+"), k, file2.display());
        }
        for k in &changed {
            println!("  {} {} (modified)", yellow("~"), k);
        }
        let unchanged = in_both.len() - changed.len();
        if unchanged > 0 {
            println!("  {} {} resource(s) unchanged", dim("="), unchanged);
        }
    }

    Ok(())
}

// FJ-371: Expand recipe template to stdout
pub(crate) fn cmd_template(recipe: &Path, vars: &[String], json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(recipe)
        .map_err(|e| format!("cannot read recipe {}: {}", recipe.display(), e))?;

    // Parse vars into map
    let mut var_map = std::collections::HashMap::new();
    for v in vars {
        if let Some((key, val)) = v.split_once('=') {
            var_map.insert(key.to_string(), val.to_string());
        }
    }

    // Simple template expansion: replace {{inputs.KEY}} with value
    let mut expanded = content.clone();
    for (key, val) in &var_map {
        let pattern = format!("{{{{inputs.{key}}}}}");
        expanded = expanded.replace(&pattern, val);
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "recipe": recipe.display().to_string(),
                "vars": var_map,
                "expanded": expanded,
            })
        );
    } else {
        println!("{expanded}");
    }

    Ok(())
}

// FJ-220 + FJ-3200: Evaluate policy rules and report violations.
pub(crate) fn cmd_policy(file: &Path, json: bool, sarif: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let result = parser::evaluate_policies_full(&config);

    if sarif {
        println!("{}", parser::policy_check_to_sarif(&result));
    } else if json {
        println!("{}", parser::policy_check_to_json(&result));
    } else if result.violations.is_empty() {
        println!("All {} policy rules passed.", config.policies.len());
        return Ok(());
    } else {
        for v in &result.violations {
            let sev = match v.severity {
                types::PolicySeverity::Error => "ERROR",
                types::PolicySeverity::Warning => "WARN",
                types::PolicySeverity::Info => "INFO",
            };
            let id = v.policy_id.as_deref().unwrap_or("-");
            println!("  [{sev}] [{id}] {}: {}", v.resource_id, v.rule_message);
            if let Some(ref rem) = v.remediation {
                println!("         fix: {rem}");
            }
        }
        println!();
        let e = result.error_count();
        let w = result.warning_count();
        if e > 0 {
            println!("Policy check failed: {e} error(s), {w} warning(s)");
        } else {
            println!("Policy check passed with {w} warning(s)");
        }
    }

    if result.has_blocking_violations() {
        return Err(format!(
            "policy violations block apply ({} error(s))",
            result.error_count()
        ));
    }

    Ok(())
}

fn print_single_output(k: &str, v: &str, json: bool) {
    if json {
        println!("{}", serde_json::json!({ k: v }));
    } else {
        println!("{v}");
    }
}

fn print_all_outputs(
    resolved: &indexmap::IndexMap<String, String>,
    outputs: &indexmap::IndexMap<String, types::OutputValue>,
    json: bool,
) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&resolved).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        for (k, v) in resolved {
            if let Some(desc) = outputs.get(k).and_then(|o| o.description.as_deref()) {
                println!("{k}: {v} ({desc})");
            } else {
                println!("{k}: {v}");
            }
        }
    }
}

pub(crate) fn cmd_output(file: &Path, key: Option<&str>, json: bool) -> Result<(), String> {
    use crate::core::state;

    let config = parse_and_validate(file)?;

    if config.outputs.is_empty() {
        if json {
            println!("{{}}");
        } else {
            println!("No outputs defined.");
        }
        return Ok(());
    }

    let resolved = state::resolve_outputs(&config);

    if let Some(k) = key {
        match resolved.get(k) {
            Some(v) => print_single_output(k, v, json),
            None => return Err(format!("output '{k}' not defined")),
        }
    } else {
        print_all_outputs(&resolved, &config.outputs, json);
    }

    Ok(())
}
