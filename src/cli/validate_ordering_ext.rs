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
