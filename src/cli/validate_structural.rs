//! Structural validation checks.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

/// Scan text for unresolved template references across all namespaces.
fn find_unresolved_templates(
    text: &str,
    name: &str,
    config: &types::ForjarConfig,
    unresolved: &mut Vec<(String, String)>,
) {
    let mut start = 0;
    while let Some(pos) = text[start..].find("{{") {
        let abs_pos = start + pos + 2;
        if let Some(end) = text[abs_pos..].find("}}") {
            let var = text[abs_pos..abs_pos + end].trim();
            check_template_var(var, name, config, unresolved);
            start = abs_pos + end + 2;
        } else {
            break;
        }
    }
}

/// Validate a single template variable against the config.
fn check_template_var(
    var: &str, rname: &str, config: &types::ForjarConfig,
    unresolved: &mut Vec<(String, String)>,
) {
    let bad = if let Some(k) = var.strip_prefix("params.") {
        !config.params.contains_key(k)
    } else if var.starts_with("machine.") {
        var.split('.').nth(1).is_some_and(|m| !config.machines.contains_key(m))
    } else if let Some(k) = var.strip_prefix("data.") {
        !config.params.contains_key(&format!("__data__{k}"))
    } else {
        false // secrets.* and func() are runtime-resolved
    };
    if bad { unresolved.push((rname.to_string(), var.to_string())); }
}

/// Collect all templateable string fields from a resource.
fn resource_template_fields(res: &types::Resource) -> Vec<&str> {
    [&res.content, &res.path, &res.target, &res.owner, &res.name,
     &res.command, &res.image, &res.source, &res.schedule]
        .iter().filter_map(|o| o.as_deref()).collect()
}

// ── FJ-421: validate --check-templates ──

pub(crate) fn cmd_validate_check_templates(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut unresolved: Vec<(String, String)> = Vec::new();

    for (name, res) in &config.resources {
        for field in resource_template_fields(res) {
            find_unresolved_templates(field, name, &config, &mut unresolved);
        }
    }

    if json {
        let entries: Vec<String> = unresolved
            .iter()
            .map(|(r, v)| format!("{{\"resource\":\"{r}\",\"variable\":\"{v}\"}}"))
            .collect();
        println!(
            "{{\"valid\":{},\"unresolved\":[{}],\"count\":{}}}",
            unresolved.is_empty(),
            entries.join(","),
            unresolved.len()
        );
    } else if unresolved.is_empty() {
        println!("{} All template variables resolve", green("✓"));
    } else {
        println!(
            "{} {} unresolved template variable(s):",
            red("✗"),
            unresolved.len()
        );
        for (r, v) in &unresolved {
            println!("  {} resource '{}': {{{{{}}}}}", red("•"), r, v);
        }
    }
    if unresolved.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} unresolved template variable(s)",
            unresolved.len()
        ))
    }
}

// ── FJ-441: validate --check-secrets ──

pub(crate) fn cmd_validate_check_secrets(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let secret_patterns = [
        "password:",
        "secret:",
        "api_key:",
        "token:",
        "private_key:",
        "aws_secret",
        "AKIA", // AWS access key prefix
        "ghp_", // GitHub PAT
        "sk-",  // OpenAI/Stripe key prefix
    ];

    let mut findings = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let trimmed = line.trim().to_lowercase();
        if trimmed.starts_with('#') {
            continue;
        }
        for pattern in &secret_patterns {
            if trimmed.contains(&pattern.to_lowercase()) {
                findings.push((i + 1, pattern.to_string(), line.trim().to_string()));
            }
        }
    }

    if json {
        let items: Vec<String> = findings
            .iter()
            .map(|(line, pat, _)| format!("{{\"line\":{line},\"pattern\":\"{pat}\"}}"))
            .collect();
        let findings_json = format!("[{}]", items.join(","));
        println!(
            "{{\"check_secrets\":true,\"findings\":{},\"ok\":{}}}",
            findings_json,
            findings.is_empty()
        );
    } else if findings.is_empty() {
        println!("{} No hardcoded secrets detected.", green("✓"));
    } else {
        println!(
            "{} {} potential secret(s) detected:",
            red("✗"),
            findings.len()
        );
        for (line, pattern, text) in &findings {
            println!("  line {line}: pattern '{pattern}' in: {text}");
        }
    }
    if findings.is_empty() {
        Ok(())
    } else {
        Err(format!("{} potential secrets found", findings.len()))
    }
}

/// Build adjacency list from config.
fn build_adj_list(config: &types::ForjarConfig) -> std::collections::HashMap<&str, Vec<&str>> {
    let mut adj: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for (name, res) in &config.resources {
        adj.entry(name.as_str()).or_default();
        for dep in &res.depends_on {
            adj.entry(name.as_str()).or_default().push(dep.as_str());
        }
    }
    adj
}

/// Compute transitive closure via Floyd-Warshall and find self-reachable nodes.
fn find_deep_cycles<'a>(
    names: &[&'a str],
    adj: &std::collections::HashMap<&str, Vec<&'a str>>,
) -> Vec<String> {
    let mut reachable: std::collections::HashMap<(&str, &str), bool> =
        std::collections::HashMap::new();
    for &n in names {
        for dep in adj.get(n).unwrap_or(&vec![]) {
            reachable.insert((n, dep), true);
        }
    }
    for &k in names {
        for &i in names {
            for &j in names {
                if reachable.contains_key(&(i, k)) && reachable.contains_key(&(k, j)) {
                    reachable.insert((i, j), true);
                }
            }
        }
    }
    let mut cycles: Vec<String> = Vec::new();
    for &n in names {
        if reachable.contains_key(&(n, n)) {
            cycles.push(n.to_string());
        }
    }
    cycles.sort();
    cycles
}

// ── FJ-471: validate --check-cycles-deep ──

pub(crate) fn cmd_validate_check_cycles_deep(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let adj = build_adj_list(&config);
    let names: Vec<&str> = adj.keys().copied().collect();
    let cycles = find_deep_cycles(&names, &adj);
    if json {
        let result = serde_json::json!({
            "deep_cycles": cycles,
            "has_cycles": !cycles.is_empty(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else if cycles.is_empty() {
        println!(
            "{} No indirect cycles detected (transitive closure clean)",
            green("✓")
        );
    } else {
        println!(
            "{} Indirect cycles detected in {} resource(s):",
            red("✗"),
            cycles.len()
        );
        for c in &cycles {
            println!("  - {c}");
        }
    }
    if cycles.is_empty() {
        Ok(())
    } else {
        Err(format!("{} resource(s) in cycles", cycles.len()))
    }
}

// ── FJ-481: validate --check-naming ──

pub(crate) fn cmd_validate_check_naming(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut violations: Vec<String> = Vec::new();
    for name in config.resources.keys() {
        let is_kebab = !name.is_empty()
            && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !name.contains("--")
            && !name.ends_with('-');
        if !is_kebab {
            violations.push(format!(
                "'{name}' is not kebab-case (expected: lowercase letters, digits, hyphens)"
            ));
        }
    }
    if json {
        let result = serde_json::json!({
            "naming_violations": violations,
            "valid": violations.is_empty(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else if violations.is_empty() {
        println!(
            "{} All resource names follow kebab-case convention",
            green("✓")
        );
    } else {
        println!("{} Naming violations ({}):", red("✗"), violations.len());
        for v in &violations {
            println!("  - {v}");
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!("{} naming violations", violations.len()))
    }
}

/// Build a map from (machine, path) to resource names.
fn build_path_map(
    config: &types::ForjarConfig,
) -> std::collections::HashMap<(String, String), Vec<String>> {
    let mut path_map: std::collections::HashMap<(String, String), Vec<String>> =
        std::collections::HashMap::new();
    for (name, res) in &config.resources {
        if let Some(path) = &res.path {
            let machine = match &res.machine {
                types::MachineTarget::Single(s) => s.clone(),
                types::MachineTarget::Multiple(ms) => ms.first().cloned().unwrap_or_default(),
            };
            path_map
                .entry((machine, path.clone()))
                .or_default()
                .push(name.clone());
        }
    }
    path_map
}

/// Find overlapping paths from the path map.
fn find_overlaps(
    path_map: &std::collections::HashMap<(String, String), Vec<String>>,
) -> Vec<String> {
    let mut overlaps: Vec<String> = Vec::new();
    for ((machine, path), names) in path_map {
        if names.len() > 1 {
            overlaps.push(format!(
                "path '{}' on machine '{}' used by: {}",
                path,
                machine,
                names.join(", ")
            ));
        }
    }
    overlaps.sort();
    overlaps
}

// ── FJ-491: validate --check-overlaps ──

pub(crate) fn cmd_validate_check_overlaps(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let path_map = build_path_map(&config);
    let overlaps = find_overlaps(&path_map);
    if json {
        let result =
            serde_json::json!({ "overlaps": overlaps, "has_overlaps": !overlaps.is_empty() });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else if overlaps.is_empty() {
        println!("{} No resource overlaps detected", green("✓"));
    } else {
        println!("{} Resource overlaps ({}):", red("✗"), overlaps.len());
        for o in &overlaps {
            println!("  - {o}");
        }
    }
    if overlaps.is_empty() {
        Ok(())
    } else {
        Err(format!("{} overlap(s) detected", overlaps.len()))
    }
}

// ── FJ-501: validate --check-limits ──

pub(crate) fn cmd_validate_check_limits(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for res in config.resources.values() {
        let machine_name = match &res.machine {
            types::MachineTarget::Single(s) => s.clone(),
            types::MachineTarget::Multiple(ms) => ms.first().cloned().unwrap_or_default(),
        };
        let key = format!("{}:{:?}", machine_name, res.resource_type);
        *counts.entry(key).or_insert(0) += 1;
    }
    let max_per_type = 50;
    let mut violations: Vec<String> = Vec::new();
    for (key, count) in &counts {
        if *count > max_per_type {
            violations.push(format!(
                "{key} has {count} resources (limit: {max_per_type})"
            ));
        }
    }
    if json {
        let result = serde_json::json!({ "limits": counts, "violations": violations, "valid": violations.is_empty() });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else if violations.is_empty() {
        println!(
            "{} All resource counts within limits (max {} per machine/type)",
            green("✓"),
            max_per_type
        );
    } else {
        println!("{} Resource limit violations:", red("✗"));
        for v in &violations {
            println!("  - {v}");
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!("{} limit violations", violations.len()))
    }
}

/// FJ-631: Detect circular template/param references.
pub(crate) fn cmd_validate_check_circular_refs(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let mut cycles: Vec<String> = Vec::new();

    for name in config.resources.keys() {
        if has_circular_dep(name, &config) {
            cycles.push(name.clone());
        }
    }

    if json {
        let items: Vec<String> = cycles.iter().map(|c| format!(r#""{c}""#)).collect();
        println!(
            r#"{{"circular_refs":[{}],"count":{}}}"#,
            items.join(","),
            cycles.len()
        );
    } else if cycles.is_empty() {
        println!("No circular references detected");
    } else {
        println!("Circular references found ({}):", cycles.len());
        for c in &cycles {
            println!("  {c} (circular dependency)");
        }
    }
    Ok(())
}

/// Check if a resource has a circular dependency back to itself via DFS.
fn has_circular_dep(name: &str, config: &types::ForjarConfig) -> bool {
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![name.to_string()];
    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            if current == name {
                return true;
            }
            continue;
        }
        if let Some(resource) = config.resources.get(&current) {
            for dep in &resource.depends_on {
                stack.push(dep.clone());
            }
        }
    }
    false
}

/// FJ-641: Validate naming conventions across resources
pub(crate) fn cmd_validate_check_naming_conventions(file: &Path, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("Read error: {e}"))?;
    let config: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| format!("Parse error: {e}"))?;

    let mut violations = Vec::new();
    for (name, _resource) in &config.resources {
        check_naming_convention(name, &mut violations);
    }

    if json {
        println!(
            r#"{{"violations_count":{},"violations":[{}]}}"#,
            violations.len(),
            violations
                .iter()
                .map(|v| format!(r#""{}""#, v.replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(",")
        );
    } else if violations.is_empty() {
        println!("All resource names follow naming conventions");
    } else {
        println!("Naming convention violations ({}):", violations.len());
        for v in &violations {
            println!("  - {v}");
        }
    }
    Ok(())
}

/// Check a single resource name against naming conventions.
fn check_naming_convention(name: &str, violations: &mut Vec<String>) {
    let is_kebab = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !is_kebab {
        violations.push(format!(
            "Resource '{name}': name should be kebab-case (lowercase, hyphens only)"
        ));
    }
    if name.starts_with('-') || name.ends_with('-') {
        violations.push(format!(
            "Resource '{name}': name should not start or end with hyphen"
        ));
    }
    if name.contains("--") {
        violations.push(format!(
            "Resource '{name}': name should not contain consecutive hyphens"
        ));
    }
}
