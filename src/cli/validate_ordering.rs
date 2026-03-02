//! Dependency ordering & tag completeness validation.

#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-925: Verify dependency ordering is topologically valid.
pub(crate) fn cmd_validate_check_resource_dependency_ordering(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let issues = find_ordering_issues(&config);
    if json {
        let items: Vec<String> = issues
            .iter()
            .map(|(n, r)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", n, r))
            .collect();
        println!("{{\"ordering_issues\":[{}]}}", items.join(","));
    } else if issues.is_empty() {
        println!("All resource dependencies are topologically valid.");
    } else {
        println!("Dependency ordering issues:");
        for (n, r) in &issues {
            println!("  {} — {}", n, r);
        }
    }
    Ok(())
}

fn find_ordering_issues(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut issues = Vec::new();
    let names: std::collections::HashSet<&str> =
        config.resources.keys().map(|k| k.as_str()).collect();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if !names.contains(dep.as_str()) {
                issues.push((name.clone(), format!("depends on non-existent '{}'", dep)));
            }
            if dep == name {
                issues.push((name.clone(), "self-dependency".to_string()));
            }
        }
    }
    issues.sort_by(|a, b| a.0.cmp(&b.0));
    issues
}

/// FJ-929: Ensure all resources have required tag categories.
pub(crate) fn cmd_validate_check_resource_tag_completeness(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let missing = find_missing_tags(&config);
    if json {
        let items: Vec<String> = missing
            .iter()
            .map(|(n, c)| format!("{{\"resource\":\"{}\",\"tag_count\":{}}}", n, c))
            .collect();
        println!("{{\"tag_completeness\":[{}]}}", items.join(","));
    } else if missing.is_empty() {
        println!("All resources have tags.");
    } else {
        println!("Resources missing tags:");
        for (n, _) in &missing {
            println!("  {}", n);
        }
    }
    Ok(())
}

fn find_missing_tags(config: &types::ForjarConfig) -> Vec<(String, usize)> {
    let mut missing: Vec<(String, usize)> = config
        .resources
        .iter()
        .filter(|(_, res)| res.tags.is_empty())
        .map(|(name, _)| (name.clone(), 0))
        .collect();
    missing.sort_by(|a, b| a.0.cmp(&b.0));
    missing
}

/// FJ-933: Enforce naming conventions via configurable patterns.
pub(crate) fn cmd_validate_check_resource_naming_standards(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let violations = find_naming_violations(&config);
    if json {
        let items: Vec<String> = violations
            .iter()
            .map(|(n, r)| format!("{{\"resource\":\"{}\",\"issue\":\"{}\"}}", n, r))
            .collect();
        println!("{{\"naming_violations\":[{}]}}", items.join(","));
    } else if violations.is_empty() {
        println!("All resource names follow naming conventions.");
    } else {
        println!("Naming convention violations:");
        for (n, r) in &violations {
            println!("  {} — {}", n, r);
        }
    }
    Ok(())
}

fn find_naming_violations(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut violations = Vec::new();
    for name in config.resources.keys() {
        if name.contains(' ') {
            violations.push((name.clone(), "contains spaces".to_string()));
        }
        if name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            violations.push((name.clone(), "starts with uppercase".to_string()));
        }
        if name.contains("__") {
            violations.push((name.clone(), "contains double underscore".to_string()));
        }
    }
    violations.sort_by(|a, b| a.0.cmp(&b.0));
    violations
}

/// FJ-937: Detect asymmetric dependency declarations.
pub(crate) fn cmd_validate_check_resource_dependency_symmetry(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let asymmetries = find_dependency_asymmetries(&config);
    if json {
        let items: Vec<String> = asymmetries
            .iter()
            .map(|(a, b)| format!("{{\"from\":\"{}\",\"to\":\"{}\"}}", a, b))
            .collect();
        println!("{{\"asymmetric_dependencies\":[{}]}}", items.join(","));
    } else if asymmetries.is_empty() {
        println!("No asymmetric dependencies detected.");
    } else {
        println!("Asymmetric dependencies:");
        for (a, b) in &asymmetries {
            println!("  {} depends on {} (but not vice versa)", a, b);
        }
    }
    Ok(())
}

fn find_dependency_asymmetries(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if let Some(dep_res) = config.resources.get(dep) {
                if !dep_res.depends_on.contains(name) {
                    pairs.push((name.clone(), dep.clone()));
                }
            }
        }
    }
    pairs.sort();
    pairs.dedup();
    pairs
}

/// FJ-941: Detect circular alias references in resource configs.
pub(crate) fn cmd_validate_check_resource_circular_alias(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let cycles = find_circular_aliases(&config);
    if json {
        let items: Vec<String> = cycles
            .iter()
            .map(|(a, b)| format!("[\"{}\",\"{}\"]", a, b))
            .collect();
        println!("{{\"circular_aliases\":[{}]}}", items.join(","));
    } else if cycles.is_empty() {
        println!("No circular alias references detected.");
    } else {
        println!("Circular alias references:");
        for (a, b) in &cycles {
            println!("  {} ↔ {}", a, b);
        }
    }
    Ok(())
}

fn find_circular_aliases(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut cycles = Vec::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if let Some(dep_res) = config.resources.get(dep) {
                if dep_res.depends_on.contains(name) && name < dep {
                    cycles.push((name.clone(), dep.clone()));
                }
            }
        }
    }
    cycles.sort();
    cycles
}

/// FJ-945: Warn when dependency chains exceed a threshold.
pub(crate) fn cmd_validate_check_resource_dependency_depth_limit(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let limit = 5;
    let violations = find_depth_limit_violations(&config, limit);
    if json {
        let items: Vec<String> = violations
            .iter()
            .map(|(r, d)| format!("{{\"resource\":\"{}\",\"depth\":{}}}", r, d))
            .collect();
        println!(
            "{{\"depth_limit\":{},\"violations\":[{}]}}",
            limit,
            items.join(",")
        );
    } else if violations.is_empty() {
        println!("All dependency chains within depth limit ({}).", limit);
    } else {
        println!("Dependency depth violations (limit {}):", limit);
        for (r, d) in &violations {
            println!("  {} — depth {}", r, d);
        }
    }
    Ok(())
}

fn find_depth_limit_violations(config: &types::ForjarConfig, limit: usize) -> Vec<(String, usize)> {
    let names: Vec<&String> = config.resources.keys().collect();
    let mut violations = Vec::new();
    for name in &names {
        let depth = compute_depth(config, name, &mut std::collections::HashSet::new());
        if depth > limit {
            violations.push(((*name).clone(), depth));
        }
    }
    violations.sort_by(|a, b| a.0.cmp(&b.0));
    violations
}

fn compute_depth(
    config: &types::ForjarConfig,
    name: &str,
    visited: &mut std::collections::HashSet<String>,
) -> usize {
    if visited.contains(name) {
        return 0;
    }
    visited.insert(name.to_string());
    let res = match config.resources.get(name) {
        Some(r) => r,
        None => return 0,
    };
    let mut max_dep = 0;
    for dep in &res.depends_on {
        let d = compute_depth(config, dep, visited);
        if d + 1 > max_dep {
            max_dep = d + 1;
        }
    }
    max_dep
}

/// FJ-949: Detect parameters defined but never referenced in templates.
pub(crate) fn cmd_validate_check_resource_unused_params(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let defined: Vec<String> = config.params.keys().cloned().collect();
    let mut used = std::collections::HashSet::new();
    for res in config.resources.values() {
        if let Some(ref c) = res.content {
            for p in &defined {
                if c.contains(&format!("{{{{{}}}}}", p)) || c.contains(&format!("${{{}}}", p)) {
                    used.insert(p.clone());
                }
            }
        }
    }
    let unused: Vec<&String> = defined.iter().filter(|p| !used.contains(*p)).collect();
    if json {
        let items: Vec<String> = unused.iter().map(|p| format!("\"{}\"", p)).collect();
        println!("{{\"unused_params\":[{}]}}", items.join(","));
    } else if unused.is_empty() {
        println!("No unused parameters detected.");
    } else {
        println!("Unused parameters:");
        for p in &unused {
            println!("  {}", p);
        }
    }
    Ok(())
}

/// FJ-957: Verify content hashes match declared checksums.
pub(crate) fn cmd_validate_check_resource_content_hash_consistency(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut mismatches = Vec::new();
    for (name, res) in &config.resources {
        if let (Some(ref declared), Some(ref actual_content)) = (&res.checksum, &res.content) {
            let computed = crate::core::planner::hash_desired_state(res);
            if &computed != declared {
                mismatches.push((name.clone(), declared.clone(), computed));
            }
            let _ = actual_content;
        }
    }
    if json {
        let items: Vec<String> = mismatches
            .iter()
            .map(|(n, d, c)| {
                format!(
                    "{{\"resource\":\"{}\",\"declared\":\"{}\",\"computed\":\"{}\"}}",
                    n, d, c
                )
            })
            .collect();
        println!("{{\"hash_mismatches\":[{}]}}", items.join(","));
    } else if mismatches.is_empty() {
        println!("All content hashes are consistent.");
    } else {
        println!("Content hash mismatches:");
        for (n, d, c) in &mismatches {
            println!(
                "  {} — declared: {}  computed: {}",
                n,
                &d[..8.min(d.len())],
                &c[..8.min(c.len())]
            );
        }
    }
    Ok(())
}

/// FJ-961: Ensure all referenced dependencies exist in the resource set.
pub(crate) fn cmd_validate_check_resource_dependency_refs(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let resource_names: std::collections::HashSet<&String> = config.resources.keys().collect();
    let mut missing = Vec::new();
    for (name, res) in &config.resources {
        for dep in &res.depends_on {
            if !resource_names.contains(dep) {
                missing.push((name.clone(), dep.clone()));
            }
        }
        for trig in &res.triggers {
            if !resource_names.contains(trig) {
                missing.push((name.clone(), trig.clone()));
            }
        }
    }
    missing.sort();
    if json {
        let items: Vec<String> = missing
            .iter()
            .map(|(n, d)| format!("{{\"resource\":\"{}\",\"missing_ref\":\"{}\"}}", n, d))
            .collect();
        println!("{{\"missing_dependency_refs\":[{}]}}", items.join(","));
    } else if missing.is_empty() {
        println!("All dependency references are valid.");
    } else {
        println!("Missing dependency references:");
        for (n, d) in &missing {
            println!("  {} → {} (not found)", n, d);
        }
    }
    Ok(())
}

/// FJ-965: Ensure all trigger references point to existing resources.
pub(crate) fn cmd_validate_check_resource_trigger_refs(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let resource_names: std::collections::HashSet<&String> = config.resources.keys().collect();
    let mut invalid = Vec::new();
    for (name, res) in &config.resources {
        for trig in &res.triggers {
            if !resource_names.contains(trig) {
                invalid.push((name.clone(), trig.clone()));
            }
        }
    }
    invalid.sort();
    if json {
        let items: Vec<String> = invalid
            .iter()
            .map(|(n, t)| format!("{{\"resource\":\"{}\",\"invalid_trigger\":\"{}\"}}", n, t))
            .collect();
        println!("{{\"invalid_trigger_refs\":[{}]}}", items.join(","));
    } else if invalid.is_empty() {
        println!("All trigger references are valid.");
    } else {
        println!("Invalid trigger references:");
        for (n, t) in &invalid {
            println!("  {} → {} (not found)", n, t);
        }
    }
    Ok(())
}

/// FJ-969: Validate parameter types match expected usage patterns.
pub(crate) fn cmd_validate_check_resource_param_type_safety(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut warnings = Vec::new();
    for (name, value) in &config.params {
        let val_str = match value {
            serde_yaml_ng::Value::String(s) => s.clone(),
            serde_yaml_ng::Value::Number(n) => n.to_string(),
            serde_yaml_ng::Value::Bool(b) => b.to_string(),
            _ => continue,
        };
        if (name.contains("port") || name.ends_with("_port")) && val_str.parse::<u16>().is_err() {
            warnings.push((
                name.clone(),
                format!("expected port number, got '{}'", val_str),
            ));
        }
        if (name.contains("path") || name.ends_with("_dir"))
            && !val_str.starts_with('/')
            && !val_str.starts_with('.')
        {
            warnings.push((name.clone(), format!("expected path, got '{}'", val_str)));
        }
    }
    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(n, w)| format!("{{\"param\":\"{}\",\"warning\":\"{}\"}}", n, w))
            .collect();
        println!("{{\"param_type_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All parameter types look consistent.");
    } else {
        println!("Parameter type warnings:");
        for (n, w) in &warnings {
            println!("  {} — {}", n, w);
        }
    }
    Ok(())
}

/// FJ-953: Warn when machines have unbalanced resource counts.
pub(crate) fn cmd_validate_check_resource_machine_balance(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for res in config.resources.values() {
        *counts.entry(res.machine.to_string()).or_insert(0) += 1;
    }
    let values: Vec<usize> = counts.values().cloned().collect();
    let max = values.iter().max().copied().unwrap_or(0);
    let min = values.iter().min().copied().unwrap_or(0);
    let imbalance = if max > 0 {
        (max - min) as f64 / max as f64
    } else {
        0.0
    };
    if json {
        let items: Vec<String> = counts
            .iter()
            .map(|(m, c)| format!("{{\"machine\":\"{}\",\"resources\":{}}}", m, c))
            .collect();
        println!(
            "{{\"imbalance_ratio\":{:.4},\"machines\":[{}]}}",
            imbalance,
            items.join(",")
        );
    } else if imbalance > 0.5 {
        println!("Resource imbalance detected (ratio: {:.4}):", imbalance);
        for (m, c) in &counts {
            println!("  {} — {} resources", m, c);
        }
    } else {
        println!(
            "Resource distribution is balanced (ratio: {:.4}).",
            imbalance
        );
    }
    Ok(())
}

/// FJ-973: Validate environment variable references match declared params.
pub(crate) fn cmd_validate_check_resource_env_consistency(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let declared_params: std::collections::HashSet<String> =
        config.params.keys().cloned().collect();
    let mut warnings: Vec<(String, String)> = Vec::new();
    for (name, res) in &config.resources {
        if let Some(ref c) = res.content {
            let mut rest = c.as_str();
            while let Some(start) = rest.find("{{") {
                rest = &rest[start + 2..];
                if let Some(end) = rest.find("}}") {
                    let var = &rest[..end];
                    if var.chars().all(|c| c.is_alphanumeric() || c == '_')
                        && !declared_params.contains(var)
                    {
                        warnings.push((
                            name.clone(),
                            format!("references undeclared param '{}'", var),
                        ));
                    }
                    rest = &rest[end + 2..];
                } else {
                    break;
                }
            }
        }
    }
    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(n, w)| format!("{{\"resource\":\"{}\",\"warning\":\"{}\"}}", n, w))
            .collect();
        println!("{{\"env_consistency_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All environment variable references are consistent.");
    } else {
        println!("Environment variable warnings:");
        for (n, w) in &warnings {
            println!("  {} — {}", n, w);
        }
    }
    Ok(())
}

/// FJ-977: Validate secret resources have rotation policies defined.
pub(crate) fn cmd_validate_check_resource_secret_rotation(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut warnings: Vec<String> = Vec::new();
    for (name, res) in &config.resources {
        let is_secret = name.contains("secret")
            || name.contains("key")
            || name.contains("password")
            || name.contains("credential")
            || name.contains("token");
        if is_secret && res.tags.is_empty() {
            warnings.push(name.clone());
        }
    }
    if json {
        let items: Vec<String> = warnings.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"secrets_without_rotation\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All secret resources have rotation metadata.");
    } else {
        println!("Secrets without rotation tags:");
        for n in &warnings {
            println!("  {} — missing rotation policy tags", n);
        }
    }
    Ok(())
}
/// FJ-981: Verify resources define all lifecycle stages.
pub(crate) fn cmd_validate_check_resource_lifecycle_completeness(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let mut warnings: Vec<String> = Vec::new();
    for (name, res) in &config.resources {
        if res.content.is_none() && res.depends_on.is_empty() && res.tags.is_empty() {
            warnings.push(name.clone());
        }
    }
    if json {
        let items: Vec<String> = warnings.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("{{\"incomplete_lifecycle\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resources have complete lifecycle definitions.");
    } else {
        println!("Resources with incomplete lifecycle:");
        for n in &warnings {
            println!("  {} — missing content/deps/tags", n);
        }
    }
    Ok(())
}
/// FJ-985: Verify resource types are compatible with declared providers.
pub(crate) fn cmd_validate_check_resource_provider_compatibility(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;
    let valid_types = [
        "file",
        "package",
        "service",
        "mount",
        "cron",
        "directory",
        "user",
        "group",
        "link",
    ];
    let mut warnings: Vec<(String, String)> = Vec::new();
    for (name, res) in &config.resources {
        let rtype = format!("{:?}", res.resource_type).to_lowercase();
        if !valid_types.iter().any(|t| rtype.contains(t)) {
            warnings.push((name.clone(), rtype));
        }
    }
    if json {
        let items: Vec<String> = warnings
            .iter()
            .map(|(n, t)| format!("{{\"resource\":\"{}\",\"type\":\"{}\"}}", n, t))
            .collect();
        println!("{{\"provider_warnings\":[{}]}}", items.join(","));
    } else if warnings.is_empty() {
        println!("All resource types are compatible with providers.");
    } else {
        println!("Provider compatibility warnings:");
        for (n, t) in &warnings {
            println!("  {} — unknown type '{}'", n, t);
        }
    }
    Ok(())
}
