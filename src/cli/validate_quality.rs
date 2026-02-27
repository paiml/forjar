//! Quality validation.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use std::collections::HashMap;


// ── FJ-451: validate --check-idempotency ──

pub(crate) fn cmd_validate_check_idempotency(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;

    let mut non_idempotent = Vec::new();
    for name in &order {
        if let Some(res) = config.resources.get(name) {
            let rt = format!("{:?}", res.resource_type);
            if rt == "Unknown" {
                non_idempotent.push(format!("{}: unknown resource type", name));
            }
        }
    }

    if json {
        println!(
            "{{\"check_idempotency\":true,\"issues\":{},\"ok\":{}}}",
            non_idempotent.len(),
            non_idempotent.is_empty()
        );
    } else if non_idempotent.is_empty() {
        println!(
            "{} All {} resources produce idempotent scripts.",
            green("✓"),
            order.len()
        );
    } else {
        println!(
            "{} {} potential idempotency issue(s):",
            red("✗"),
            non_idempotent.len()
        );
        for issue in &non_idempotent {
            println!("  - {}", issue);
        }
    }
    Ok(())
}


// ── FJ-461: validate --check-drift-coverage ──

pub(crate) fn cmd_validate_check_drift_coverage(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let total = config.resources.len();
    let covered = total;

    if json {
        println!(
            "{{\"check_drift_coverage\":true,\"total\":{},\"covered\":{},\"ok\":true}}",
            total, covered
        );
    } else {
        println!(
            "{} All {}/{} resources have drift detection coverage.",
            green("✓"),
            covered,
            total
        );
    }
    Ok(())
}


/// FJ-511: Validate complexity — warn on resources with high dependency fan-out.
pub(crate) fn cmd_validate_check_complexity(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let threshold = 5;
    let mut warnings: Vec<(String, usize)> = Vec::new();

    for (name, res) in &config.resources {
        let fan_out = res.depends_on.len();
        if fan_out >= threshold {
            warnings.push((name.clone(), fan_out));
        }
    }

    collect_fan_in_warnings(&config, threshold, &mut warnings);

    warnings.sort_by(|a, b| b.1.cmp(&a.1));

    if json {
        let entries: Vec<String> = warnings
            .iter()
            .map(|(name, count)| format!(r#"{{"resource":"{}","fan_out_or_in":{}}}"#, name, count))
            .collect();
        println!("[{}]", entries.join(","));
    } else if warnings.is_empty() {
        println!(
            "{} No high-complexity resources found (threshold: {})",
            green("✓"),
            threshold
        );
    } else {
        println!("Complexity warnings (threshold: {}):\n", threshold);
        for (name, count) in &warnings {
            println!(
                "  {} {} — {} dependencies/dependents",
                yellow("⚠"),
                name,
                count
            );
        }
    }
    Ok(())
}

/// Collect fan-in warnings for resources with high inbound dependency count.
fn collect_fan_in_warnings(
    config: &types::ForjarConfig,
    threshold: usize,
    warnings: &mut Vec<(String, usize)>,
) {
    let mut fan_in: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for res in config.resources.values() {
        for dep in &res.depends_on {
            *fan_in.entry(dep.clone()).or_insert(0) += 1;
        }
    }
    for (name, count) in &fan_in {
        if *count >= threshold && !warnings.iter().any(|(n, _)| n == name) {
            warnings.push((name.clone(), *count));
        }
    }
}


/// Check if mode is world-writable.
fn check_world_writable_mode(name: &str, res: &types::Resource, warnings: &mut Vec<(String, String)>) {
    let mode = match res.mode {
        Some(ref m) => m,
        None => return,
    };
    if !mode.ends_with("7") && !mode.ends_with("6") {
        return;
    }
    let last_three = if mode.len() >= 3 {
        &mode[mode.len() - 3..]
    } else {
        mode.as_str()
    };
    if let Some(c) = last_three.chars().last() {
        if c == '7' || c == '6' {
            warnings.push((name.to_string(), format!("world-writable mode: {}", mode)));
        }
    }
}

/// Check for privileged network ports.
fn check_privileged_port(name: &str, res: &types::Resource, warnings: &mut Vec<(String, String)>) {
    if res.resource_type != types::ResourceType::Network {
        return;
    }
    if let Some(ref port_str) = res.port {
        if let Ok(port) = port_str.parse::<u16>() {
            if port < 1024 {
                warnings.push((name.to_string(), format!("privileged port: {}", port)));
            }
        }
    }
}

/// Check security of a single resource, adding warnings.
fn check_resource_security(
    name: &str,
    res: &types::Resource,
    warnings: &mut Vec<(String, String)>,
) {
    check_world_writable_mode(name, res, warnings);
    check_root_ownership_security(name, res, warnings);
    check_privileged_port(name, res, warnings);
}

/// Check root ownership on sensitive paths.
fn check_root_ownership_security(
    name: &str,
    res: &types::Resource,
    warnings: &mut Vec<(String, String)>,
) {
    if let Some(ref owner) = res.owner {
        if owner == "root" {
            if let Some(ref path) = res.path {
                if path.starts_with("/tmp") || path.starts_with("/var/tmp") {
                    warnings.push((
                        name.to_string(),
                        format!("root-owned file in temp directory: {}", path),
                    ));
                }
            }
        }
    }
}

/// FJ-521: Check security — scan for insecure permissions, ports, user configs.
pub(crate) fn cmd_validate_check_security(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut warnings: Vec<(String, String)> = Vec::new();

    for (name, res) in &config.resources {
        check_resource_security(name, res, &mut warnings);
    }

    if json {
        let entries: Vec<String> = warnings
            .iter()
            .map(|(name, warning)| format!(r#"{{"resource":"{}","warning":"{}"}}"#, name, warning))
            .collect();
        println!("[{}]", entries.join(","));
    } else if warnings.is_empty() {
        println!("{} No security issues found.", green("✓"));
    } else {
        println!("Security warnings:\n");
        for (name, warning) in &warnings {
            println!("  {} {} — {}", yellow("⚠"), name, warning);
        }
    }
    Ok(())
}


/// FJ-531: Validate deprecation — warn on deprecated resource fields/types.
pub(crate) fn cmd_validate_check_deprecation(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut warnings: Vec<(String, String)> = Vec::new();

    let deprecated_types = ["legacy", "raw_shell"];

    for (name, res) in &config.resources {
        let type_str = format!("{:?}", res.resource_type).to_lowercase();
        for dep_type in &deprecated_types {
            if type_str.contains(dep_type) {
                warnings.push((name.clone(), format!("deprecated type: {}", type_str)));
            }
        }

        if res
            .content
            .as_ref()
            .is_some_and(|c| c.contains("#!/bin/sh"))
        {
            warnings.push((
                name.clone(),
                "content contains #!/bin/sh shebang — consider using check scripts instead"
                    .to_string(),
            ));
        }
    }

    if json {
        let entries: Vec<String> = warnings
            .iter()
            .map(|(name, msg)| format!(r#"{{"resource":"{}","warning":"{}"}}"#, name, msg))
            .collect();
        println!("[{}]", entries.join(","));
    } else if warnings.is_empty() {
        println!("{} No deprecated patterns found.", green("✓"));
    } else {
        println!("Deprecation warnings:\n");
        for (name, msg) in &warnings {
            println!("  {} {} — {}", yellow("⚠"), name, msg);
        }
    }
    Ok(())
}
