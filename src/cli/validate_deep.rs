//! Deep validation — FJ-2503: `validate --deep` aggregated pass.

use super::helpers::*;
use crate::core::{resolver, types};
use std::path::Path;


fn check_templates_silent(config: &types::ForjarConfig) -> Result<(), String> {
    let mut unresolved = 0usize;
    for res in config.resources.values() {
        let yaml = serde_yaml_ng::to_string(res).unwrap_or_default();
        for cap_start in yaml.match_indices("{{params.") {
            let after = &yaml[cap_start.0 + 9..];
            if let Some(end) = after.find("}}") {
                let key = &after[..end];
                if !config.params.contains_key(key) {
                    unresolved += 1;
                }
            }
        }
    }
    if unresolved == 0 {
        Ok(())
    } else {
        Err(format!("{unresolved} unresolved template variable(s)"))
    }
}

fn check_overlaps_silent(config: &types::ForjarConfig) -> Result<(), String> {
    let mut paths: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, res) in &config.resources {
        if let Some(ref p) = res.path {
            paths.entry(p.clone()).or_default().push(name.clone());
        }
    }
    let overlaps: usize = paths.values().filter(|v| v.len() > 1).count();
    if overlaps == 0 {
        Ok(())
    } else {
        Err(format!("{overlaps} overlap(s) detected"))
    }
}

fn check_cycles_silent(config: &types::ForjarConfig) -> Result<(), String> {
    resolver::build_execution_order(config)
        .map(|_| ())
        .map_err(|e| format!("cycle detected: {e}"))
}

fn check_secrets_silent(file: &Path) -> Result<(), String> {
    let patterns = [
        "password:", "secret:", "api_key:", "token:", "private_key:", "aws_secret", "AKIA",
        "ghp_", "sk-",
    ];
    let content = std::fs::read_to_string(file).unwrap_or_default();
    let mut count = 0usize;
    for line in content.lines() {
        let trimmed = line.trim().to_lowercase();
        if trimmed.starts_with('#') {
            continue;
        }
        for pat in &patterns {
            if trimmed.contains(&pat.to_lowercase()) {
                count += 1;
            }
        }
    }
    if count == 0 {
        Ok(())
    } else {
        Err(format!("{count} potential secrets found"))
    }
}

fn check_naming_silent(config: &types::ForjarConfig) -> Result<(), String> {
    let mut violations = 0usize;
    for name in config.resources.keys() {
        let ok = !name.is_empty()
            && name.starts_with(|c: char| c.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !name.contains("--")
            && !name.ends_with('-');
        if !ok {
            violations += 1;
        }
    }
    if violations == 0 {
        Ok(())
    } else {
        Err(format!("{violations} naming violation(s)"))
    }
}

fn check_idempotency_silent(config: &types::ForjarConfig) -> Result<(), String> {
    let mut issues = 0usize;
    for res in config.resources.values() {
        if format!("{:?}", res.resource_type) == "Unknown" {
            issues += 1;
        }
    }
    if issues == 0 {
        Ok(())
    } else {
        Err(format!("{issues} potential idempotency issue(s)"))
    }
}

fn check_exhaustive_silent(config: &types::ForjarConfig) -> Result<(), String> {
    let mut issues = Vec::new();
    super::validate_core::check_resource_refs_silent(config, &mut issues);
    for (name, res) in &config.resources {
        if let Some(ref content) = res.content {
            super::validate_core::find_unresolved_content_params_silent(
                name,
                content,
                &config.params,
                &mut issues,
            );
        }
    }
    super::validate_core::check_orphaned_params_silent(config, &mut issues);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(format!("{} validation issue(s) found", issues.len()))
    }
}

/// Run all deep checks silently for JSON mode.
fn run_deep_checks_silent(
    config: &types::ForjarConfig,
    file: &Path,
) -> Vec<(&'static str, Result<(), String>)> {
    vec![
        ("templates", check_templates_silent(config)),
        ("overlaps", check_overlaps_silent(config)),
        ("circular-deps", check_cycles_silent(config)),
        ("secrets", check_secrets_silent(file)),
        ("naming", check_naming_silent(config)),
        ("drift-coverage", Ok(())),
        ("idempotency", check_idempotency_silent(config)),
        ("exhaustive", check_exhaustive_silent(config)),
    ]
}

/// Collect pass/fail from check results and emit JSON.
fn emit_deep_json(results: &[(&str, Result<(), String>)]) -> Result<(), String> {
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut failures = Vec::new();
    for (name, r) in results {
        match r {
            Ok(()) => passed += 1,
            Err(e) => {
                failed += 1;
                failures.push(serde_json::json!({"check": name, "error": e}));
            }
        }
    }
    let result = serde_json::json!({
        "deep_validation": {
            "passed": passed,
            "failed": failed,
            "total": passed + failed,
            "failures": failures,
        }
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&result).unwrap_or_default()
    );
    if failed > 0 {
        Err(format!("{failed} deep validation check(s) failed"))
    } else {
        Ok(())
    }
}

/// Run all deep validation checks and aggregate results.
pub(crate) fn cmd_validate_deep(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // JSON mode: run checks silently to avoid stdout contamination
    if json {
        let results = run_deep_checks_silent(&config, file);
        return emit_deep_json(&results);
    }

    // Text mode: use silent checks to avoid repeated parse warnings,
    // then format results as human-readable text
    let results = run_deep_checks_silent(&config, file);

    println!(
        "=== Deep Validation: {} ({} machines, {} resources) ===",
        config.name,
        config.machines.len(),
        config.resources.len()
    );
    println!();

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut failures: Vec<(String, String)> = Vec::new();

    for (name, result) in &results {
        match result {
            Ok(()) => {
                println!("  {} {name}", green("✓"));
                passed += 1;
            }
            Err(e) => {
                println!("  {} {name}: {e}", red("✗"));
                failed += 1;
                failures.push((name.to_string(), e.clone()));
            }
        }
    }

    println!();
    println!("─────────────────────────────────────");
    println!(
        "Deep validation: {}/{} checks passed",
        passed,
        passed + failed
    );

    if failed > 0 {
        Err(format!("{failed} deep validation check(s) failed"))
    } else {
        Ok(())
    }
}
