//! Fleet reporting.

use super::helpers::*;
use crate::core::{resolver, state, types};
use crate::tripwire::eventlog;
use std::path::Path;

/// FJ-341: Audit trail — who applied what, when, from which config.
pub(crate) fn cmd_audit(
    state_dir: &Path,
    machine_filter: Option<&str>,
    limit: usize,
    json: bool,
) -> Result<(), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut all_events: Vec<(String, types::TimestampedEvent)> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        if !entry.path().is_dir() {
            continue;
        }

        let log_path = eventlog::event_log_path(state_dir, &name);
        if !log_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&log_path)
            .map_err(|e| format!("cannot read {}: {}", log_path.display(), e))?;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<types::TimestampedEvent>(line) {
                all_events.push((name.clone(), event));
            }
        }
    }

    // Sort by timestamp descending
    all_events.sort_by(|a, b| b.1.ts.cmp(&a.1.ts));
    all_events.truncate(limit);

    if json {
        let json_events: Vec<serde_json::Value> = all_events
            .iter()
            .map(|(machine, ev)| {
                serde_json::json!({
                    "machine": machine,
                    "timestamp": ev.ts,
                    "event": format!("{:?}", ev.event),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json_events).unwrap_or_default()
        );
    } else {
        if all_events.is_empty() {
            println!("No audit events found.");
            return Ok(());
        }
        println!("Audit trail (last {limit} events):\n");
        for (machine, ev) in &all_events {
            println!("  {} [{}] {:?}", ev.ts, machine, ev.event);
        }
    }

    Ok(())
}

/// Check a single resource for compliance violations.
fn check_resource_compliance(
    id: &str,
    res: &types::Resource,
    violations: &mut Vec<serde_json::Value>,
) {
    if res.resource_type == crate::core::types::ResourceType::File {
        if res.mode.is_none() {
            violations.push(serde_json::json!({
                "resource": id,
                "rule": "file-mode-required",
                "severity": "warning",
                "message": format!("File resource '{}' has no mode set", id),
            }));
        }
        if res.owner.is_none() {
            violations.push(serde_json::json!({
                "resource": id,
                "rule": "file-owner-required",
                "severity": "warning",
                "message": format!("File resource '{}' has no owner set", id),
            }));
        }
    }

    if res.resource_type == crate::core::types::ResourceType::Service && res.enabled.is_none() {
        violations.push(serde_json::json!({
            "resource": id,
            "rule": "service-enabled-required",
            "severity": "warning",
            "message": format!("Service resource '{}' does not set 'enabled' explicitly", id),
        }));
    }

    if let Some(ref path) = res.path {
        if (path.starts_with("/etc/") || path.starts_with("/usr/")) && res.owner.is_none() {
            violations.push(serde_json::json!({
                "resource": id,
                "rule": "system-path-owner-required",
                "severity": "error",
                "message": format!("Resource '{}' writes to system path '{}' without explicit owner", id, path),
            }));
        }
    }
}

/// Output compliance violations in JSON or text format.
fn output_compliance_results(violations: &[serde_json::Value], json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(violations).unwrap_or_else(|_| "[]".to_string())
        );
    } else if violations.is_empty() {
        println!("{} All compliance checks passed.", green("✓"));
    } else {
        println!("Compliance violations:\n");
        for v in violations {
            let icon = if v["severity"] == "error" {
                red("✗")
            } else {
                yellow("!")
            };
            println!(
                "  {} [{}] {}",
                icon,
                v["rule"].as_str().unwrap_or("?"),
                v["message"].as_str().unwrap_or("?"),
            );
        }
        println!(
            "\n{} {} violation(s) found",
            yellow("Total:"),
            violations.len()
        );
    }
}

// FJ-351: Validate infrastructure against policy rules
pub(crate) fn cmd_compliance(file: &Path, json: bool) -> Result<(), String> {
    let mut config = parse_and_validate(file)?;
    resolver::resolve_data_sources(&mut config)?;
    let mut violations = Vec::new();

    for (id, res) in &config.resources {
        check_resource_compliance(id, res, &mut violations);
    }

    output_compliance_results(&violations, json);
    Ok(())
}

// FJ-352: Export state to external formats
/// Collect all resources from state dir, optionally filtered by machine.
fn collect_export_resources(
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<Vec<(String, String, types::ResourceLock)>, String> {
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {e}"))?;
    let mut all_resources = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            for (id, rl) in lock.resources {
                all_resources.push((id, lock.machine.clone(), rl));
            }
        }
    }
    Ok(all_resources)
}

/// Format resources as Ansible inventory YAML.
fn format_ansible(all_resources: &[(String, String, types::ResourceLock)]) -> String {
    let mut machines: std::collections::BTreeMap<&str, Vec<&str>> =
        std::collections::BTreeMap::new();
    for (id, machine, _rl) in all_resources {
        machines
            .entry(machine.as_str())
            .or_default()
            .push(id.as_str());
    }
    let mut lines = vec!["all:".to_string(), "  hosts:".to_string()];
    for (machine, resources) in &machines {
        lines.push(format!("    {machine}:"));
        lines.push("      forjar_resources:".to_string());
        for res in resources {
            lines.push(format!("        - {res}"));
        }
    }
    lines.join("\n")
}

pub(crate) fn cmd_export(
    state_dir: &Path,
    format: &str,
    machine_filter: Option<&str>,
    output: Option<&Path>,
) -> Result<(), String> {
    let all_resources = collect_export_resources(state_dir, machine_filter)?;

    let content = match format {
        "csv" => {
            let mut lines = vec!["resource,machine,type,status,hash,applied_at".to_string()];
            for (id, machine, rl) in &all_resources {
                lines.push(format!(
                    "{},{},{:?},{:?},{},{}",
                    id,
                    machine,
                    rl.resource_type,
                    rl.status,
                    rl.hash,
                    rl.applied_at.as_deref().unwrap_or("-")
                ));
            }
            lines.join("\n")
        }
        "terraform" => {
            let mut blocks = Vec::new();
            for (id, _machine, rl) in &all_resources {
                blocks.push(format!(
                    "# {}\nimport {{\n  to = forjar_resource.{}\n  id = \"{}\"\n}}",
                    id, id, rl.hash
                ));
            }
            blocks.join("\n\n")
        }
        "ansible" => format_ansible(&all_resources),
        _ => {
            return Err(format!(
                "Unknown export format '{format}'. Supported: csv, terraform, ansible"
            ))
        }
    };

    if let Some(output_path) = output {
        std::fs::write(output_path, &content).map_err(|e| e.to_string())?;
        println!(
            "Exported {} resources to {}",
            all_resources.len(),
            output_path.display()
        );
    } else {
        println!("{content}");
    }

    Ok(())
}

/// Check for unused parameters in config.
fn check_unused_params(
    config: &types::ForjarConfig,
    config_str: &str,
    suggestions: &mut Vec<serde_json::Value>,
) {
    for key in config.params.keys() {
        let pattern = format!("{{{{params.{key}}}}}");
        if !config_str.contains(&pattern) {
            suggestions.push(serde_json::json!({
                "type": "unused-param",
                "severity": "info",
                "message": format!("Parameter '{}' is defined but never referenced", key),
            }));
        }
    }
}

/// Check for missing depends_on on resources sharing a machine.
fn check_missing_dependencies(
    config: &types::ForjarConfig,
    suggestions: &mut Vec<serde_json::Value>,
) {
    let mut machine_resources: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (id, res) in &config.resources {
        let machine_name = match &res.machine {
            types::MachineTarget::Single(s) => s.clone(),
            types::MachineTarget::Multiple(ms) => ms.first().cloned().unwrap_or_default(),
        };
        machine_resources
            .entry(machine_name)
            .or_default()
            .push(id.clone());
    }
    for resources in machine_resources.values() {
        if resources.len() <= 1 {
            continue;
        }
        for id in resources {
            let res = &config.resources[id];
            if res.depends_on.is_empty() {
                let has_dependent = config.resources.values().any(|r| r.depends_on.contains(id));
                if !has_dependent && resources.len() > 2 {
                    suggestions.push(serde_json::json!({
                        "type": "no-dependencies",
                        "severity": "info",
                        "message": format!("Resource '{}' has no depends_on and nothing depends on it — verify ordering", id),
                    }));
                }
            }
        }
    }
}

/// Output suggestions in JSON or text format.
fn output_suggestions(suggestions: &[serde_json::Value], json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(suggestions).unwrap_or_else(|_| "[]".to_string())
        );
    } else if suggestions.is_empty() {
        println!("{} No suggestions — config looks good.", green("✓"));
    } else {
        println!("Suggestions:\n");
        for s in suggestions {
            println!(
                "  {} [{}] {}",
                dim("→"),
                s["type"].as_str().unwrap_or("?"),
                s["message"].as_str().unwrap_or("?"),
            );
        }
        println!("\n{} {} suggestion(s)", dim("Total:"), suggestions.len());
    }
}

// FJ-361: Suggest improvements to config
pub(crate) fn cmd_suggest(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut suggestions = Vec::new();

    let config_str = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    check_unused_params(&config, &config_str, &mut suggestions);
    check_missing_dependencies(&config, &mut suggestions);
    output_suggestions(&suggestions, json);

    Ok(())
}
