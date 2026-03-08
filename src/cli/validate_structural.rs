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
    while let Some(remaining) = text.get(start..) {
        let Some(pos) = remaining.find("{{") else {
            break;
        };
        let abs_pos = start + pos + 2;
        let Some(tail) = text.get(abs_pos..) else {
            break;
        };
        if let Some(end) = tail.find("}}") {
            let var = text.get(abs_pos..abs_pos + end).unwrap_or("").trim();
            check_template_var(var, name, config, unresolved);
            start = abs_pos + end + 2;
        } else {
            break;
        }
    }
}

/// Validate a single template variable against the config.
fn check_template_var(
    var: &str,
    rname: &str,
    config: &types::ForjarConfig,
    unresolved: &mut Vec<(String, String)>,
) {
    let bad = if let Some(k) = var.strip_prefix("params.") {
        !config.params.contains_key(k)
    } else if var.starts_with("machine.") {
        var.split('.')
            .nth(1)
            .is_some_and(|m| !config.machines.contains_key(m))
    } else if let Some(k) = var.strip_prefix("data.") {
        !config.data.contains_key(k) && !config.params.contains_key(&format!("__data__{k}"))
    } else {
        false // secrets.* and func() are runtime-resolved
    };
    if bad {
        unresolved.push((rname.to_string(), var.to_string()));
    }
}

/// Collect all templateable string fields from a resource.
fn resource_template_fields(res: &types::Resource) -> Vec<&str> {
    [
        &res.content,
        &res.path,
        &res.target,
        &res.owner,
        &res.name,
        &res.command,
        &res.image,
        &res.source,
        &res.schedule,
    ]
    .iter()
    .filter_map(|o| o.as_deref())
    .collect()
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
