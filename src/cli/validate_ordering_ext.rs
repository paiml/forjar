//! Validate ordering extensions (Phase 91+) — naming conventions, idempotency.
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-989: Enforce naming conventions on resource names.
pub(crate) fn cmd_validate_check_resource_naming_convention_strict(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let violations = find_naming_violations(&config);
    print_naming_violations(&violations, json);
    Ok(())
}
fn find_naming_violations(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut v = Vec::new();
    for name in config.resources.keys() {
        let ok = name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
        if !ok { v.push((name.clone(), "neither snake_case nor kebab-case".to_string())); }
    }
    v
}
fn print_naming_violations(violations: &[(String, String)], json: bool) {
    if json {
        let items: Vec<String> = violations.iter()
            .map(|(n, r)| format!("{{\"resource\":\"{}\",\"reason\":\"{}\"}}", n, r)).collect();
        println!("{{\"naming_violations\":[{}]}}", items.join(","));
    } else if violations.is_empty() {
        println!("All resource names follow naming conventions.");
    } else {
        println!("Naming convention violations:");
        for (n, r) in violations { println!("  {} — {}", n, r); }
    }
}
/// FJ-993: Warn if resources lack idempotency annotations.
pub(crate) fn cmd_validate_check_resource_idempotency_annotations(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let names: Vec<String> = config.resources.keys().cloned().collect();
    print_idempotency_warnings(&names, json);
    Ok(())
}
fn print_idempotency_warnings(warnings: &[String], json: bool) {
    if json {
        let items: Vec<String> = warnings.iter()
            .map(|n| format!("{{\"resource\":\"{}\",\"hint\":\"no idempotency annotation\"}}", n)).collect();
        println!("{{\"idempotency_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resources have idempotency annotations.");
    } else {
        println!("Resources without idempotency annotations: {}", warnings.len());
        for n in warnings { println!("  {} — no annotation", n); }
    }
}
/// FJ-997: Warn if resource content exceeds size threshold.
pub(crate) fn cmd_validate_check_resource_content_size_limit(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let threshold = 10240_usize; // 10KB default
    let warnings = find_oversized_resources(&config, threshold);
    print_size_warnings(&warnings, threshold, json);
    Ok(())
}
fn find_oversized_resources(config: &types::ForjarConfig, threshold: usize) -> Vec<(String, usize)> {
    let mut w = Vec::new();
    for (name, res) in &config.resources {
        let size = res.content.as_ref().map_or(0, |c| c.len());
        if size > threshold { w.push((name.clone(), size)); }
    }
    w.sort_by(|a, b| b.1.cmp(&a.1));
    w
}
fn print_size_warnings(warnings: &[(String, usize)], threshold: usize, json: bool) {
    if json {
        let items: Vec<String> = warnings.iter()
            .map(|(n, s)| format!("{{\"resource\":\"{}\",\"size\":{},\"threshold\":{}}}", n, s, threshold)).collect();
        println!("{{\"size_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resource content within size limits ({} bytes).", threshold);
    } else {
        println!("Resources exceeding {} byte limit:", threshold);
        for (n, s) in warnings { println!("  {} — {} bytes", n, s); }
    }
}
/// FJ-1001: Warn if any resource exceeds max fan-in or fan-out.
pub(crate) fn cmd_validate_check_resource_dependency_fan_limit(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let max_fan = 10_usize;
    let warnings = find_fan_violations(&config, max_fan);
    print_fan_warnings(&warnings, max_fan, json);
    Ok(())
}
fn find_fan_violations(config: &types::ForjarConfig, max_fan: usize) -> Vec<(String, usize, &'static str)> {
    let mut fan_in: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut w = Vec::new();
    for (name, res) in &config.resources {
        let fan_out = res.depends_on.len();
        if fan_out > max_fan { w.push((name.clone(), fan_out, "fan-out")); }
        for dep in &res.depends_on {
            *fan_in.entry(dep.as_str()).or_insert(0) += 1;
        }
    }
    for (name, &count) in &fan_in {
        if count > max_fan { w.push((name.to_string(), count, "fan-in")); }
    }
    w.sort_by(|a, b| b.1.cmp(&a.1));
    w
}
fn print_fan_warnings(warnings: &[(String, usize, &str)], max_fan: usize, json: bool) {
    if json {
        let items: Vec<String> = warnings.iter()
            .map(|(n, c, d)| format!("{{\"resource\":\"{}\",\"count\":{},\"direction\":\"{}\",\"limit\":{}}}", n, c, d, max_fan)).collect();
        println!("{{\"fan_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resources within fan-in/fan-out limit ({}).", max_fan);
    } else {
        println!("Resources exceeding fan limit ({}):", max_fan);
        for (n, c, d) in warnings { println!("  {} — {} {}", n, c, d); }
    }
}
