//! Policy and connectivity validation.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use std::collections::HashMap;


/// Check no_root_owner policy.
fn check_no_root_owner(rule_name: &str, config: &types::ForjarConfig, violations: &mut Vec<String>) {
    for (name, res) in &config.resources {
        if res.owner.as_deref() == Some("root") {
            violations.push(format!(
                "[{}] resource '{}' has owner 'root'",
                rule_name, name
            ));
        }
    }
}

/// Check require_tags policy.
fn check_require_tags(rule_name: &str, config: &types::ForjarConfig, violations: &mut Vec<String>) {
    for (name, res) in &config.resources {
        if res.tags.is_empty() {
            violations.push(format!("[{}] resource '{}' has no tags", rule_name, name));
        }
    }
}

/// Check require_depends_on policy.
fn check_require_depends_on(rule_name: &str, config: &types::ForjarConfig, violations: &mut Vec<String>) {
    for (name, res) in &config.resources {
        if res.depends_on.is_empty() && res.resource_type != types::ResourceType::Package {
            violations.push(format!(
                "[{}] resource '{}' has no depends_on",
                rule_name, name
            ));
        }
    }
}

/// Check a single policy rule against the config.
fn check_policy_rule(
    rule_name: &str,
    rule_type: &str,
    config: &types::ForjarConfig,
    violations: &mut Vec<String>,
) {
    match rule_type {
        "no_root_owner" => check_no_root_owner(rule_name, config, violations),
        "require_tags" => check_require_tags(rule_name, config, violations),
        "require_depends_on" => check_require_depends_on(rule_name, config, violations),
        _ => {
            violations.push(format!("unknown policy check: '{}'", rule_type));
        }
    }
}

// ── FJ-401: validate --policy-file ──

pub(crate) fn cmd_validate_policy_file(file: &Path, policy_file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let policy_content = std::fs::read_to_string(policy_file).map_err(|e| {
        format!(
            "Failed to read policy file {}: {}",
            policy_file.display(),
            e
        )
    })?;
    let policy: serde_yaml_ng::Value = serde_yaml_ng::from_str(&policy_content)
        .map_err(|e| format!("Failed to parse policy file: {}", e))?;

    let mut violations: Vec<String> = Vec::new();

    if let Some(rules) = policy.get("rules").and_then(|v| v.as_sequence()) {
        for rule in rules {
            let rule_name = rule
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed");
            let rule_type = rule.get("check").and_then(|v| v.as_str()).unwrap_or("");
            check_policy_rule(rule_name, rule_type, &config, &mut violations);
        }
    }

    if json {
        println!(
            "{{\"valid\":{},\"violations\":{},\"count\":{}}}",
            violations.is_empty(),
            serde_json::to_string(&violations).unwrap_or_else(|_| "[]".to_string()),
            violations.len()
        );
    } else if violations.is_empty() {
        println!("{} Policy validation passed", green("✓"));
    } else {
        println!(
            "{} Policy validation found {} violation(s):",
            red("✗"),
            violations.len()
        );
        for v in &violations {
            println!("  {} {}", red("•"), v);
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!("{} policy violation(s) found", violations.len()))
    }
}


// ── FJ-411: validate --check-connectivity ──

pub(crate) fn cmd_validate_connectivity(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut results: Vec<(String, String, bool)> = Vec::new();

    for (name, machine) in &config.machines {
        let addr = &machine.addr;
        let reachable = std::net::TcpStream::connect_timeout(
            &format!("{}:22", addr)
                .parse()
                .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 22))),
            std::time::Duration::from_secs(3),
        )
        .is_ok();
        results.push((name.clone(), addr.clone(), reachable));
    }

    if json {
        let entries: Vec<String> = results
            .iter()
            .map(|(name, addr, ok)| {
                format!(
                    "{{\"machine\":\"{}\",\"addr\":\"{}\",\"reachable\":{}}}",
                    name, addr, ok
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else {
        println!("{}", bold("SSH Connectivity Check"));
        for (name, addr, ok) in &results {
            let status = if *ok {
                green("✓ reachable")
            } else {
                red("✗ unreachable")
            };
            println!("  {} ({}) — {}", name, addr, status);
        }
    }
    Ok(())
}


// ── FJ-431: validate --strict-deps ──

pub(crate) fn cmd_validate_strict_deps(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut violations = Vec::new();

    let decl_order: Vec<String> = config.resources.keys().cloned().collect();
    let pos_map: std::collections::HashMap<&str, usize> = decl_order
        .iter()
        .enumerate()
        .map(|(i, name)| (name.as_str(), i))
        .collect();

    for (name, res) in &config.resources {
        let my_pos = pos_map.get(name.as_str()).copied().unwrap_or(0);
        for dep in &res.depends_on {
            if let Some(&dep_pos) = pos_map.get(dep.as_str()) {
                if dep_pos > my_pos {
                    violations.push(format!(
                        "{} depends on {} but {} is declared later",
                        name, dep, dep
                    ));
                }
            }
        }
    }

    if json {
        println!(
            "{{\"strict_deps\":true,\"violations\":{},\"ok\":{}}}",
            violations.len(),
            violations.is_empty()
        );
    } else if violations.is_empty() {
        println!(
            "{} All dependencies reference resources declared earlier.",
            green("✓")
        );
    } else {
        println!(
            "{} {} dependency ordering violation(s):",
            red("✗"),
            violations.len()
        );
        for v in &violations {
            println!("  - {}", v);
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!("{} strict-dep violations", violations.len()))
    }
}
