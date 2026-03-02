//! Phase 99 — Security Validation: secret scope, deprecation usage, when-condition coverage.

#![allow(dead_code)]

use crate::core::types;
use std::path::Path;

// ============================================================================
// FJ-1054: Resource secret scope
// ============================================================================

/// Identify resources that reference secrets and are deployed to multiple machines.
fn find_secret_scope_warnings(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut warnings = Vec::new();
    for (name, resource) in &config.resources {
        let has_secret = has_secret_reference(resource);
        if !has_secret {
            continue;
        }
        let machines = resource.machine.to_vec();
        if machines.len() > 1 {
            warnings.push((
                name.clone(),
                format!(
                    "references secrets and targets {} machines: [{}]",
                    machines.len(),
                    machines.join(", ")
                ),
            ));
        }
    }
    warnings.sort_by(|a, b| a.0.cmp(&b.0));
    warnings
}

/// Check if a resource references secrets via content templates or tags.
fn has_secret_reference(resource: &types::Resource) -> bool {
    let has_secret_tag = resource
        .tags
        .iter()
        .any(|t| t.to_lowercase().contains("secret"));
    if has_secret_tag {
        return true;
    }
    if let Some(ref content) = resource.content {
        if content.contains("{{secret.") || content.contains("${secret.") {
            return true;
        }
    }
    false
}

/// Print warnings as JSON with the given key.
fn print_warnings_json(key: &str, warnings: &[(String, String)]) {
    let items: Vec<String> = warnings
        .iter()
        .map(|(n, d)| format!(r#"{{"resource":"{}","detail":"{}"}}"#, n, d))
        .collect();
    println!(
        r#"{{"{}":[{}],"count":{}}}"#,
        key,
        items.join(","),
        warnings.len()
    );
}

/// Print warnings as text with the given label.
fn print_warnings_text(label: &str, ok_msg: &str, warnings: &[(String, String)]) {
    if warnings.is_empty() {
        println!("{}", ok_msg);
    } else {
        println!("{} ({}):", label, warnings.len());
        for (name, detail) in warnings {
            println!("  warning: {} — {}", name, detail);
        }
    }
}

/// FJ-1054: Check that resources referencing secrets are not deployed across multiple machines.
///
/// Parses the config and identifies resources that reference secrets (via
/// `{{secret.` / `${secret.` templates or "secret" in tags). Warns if any
/// such resource targets more than one machine, which may indicate unintended
/// secret distribution.
pub(crate) fn cmd_validate_check_resource_secret_scope(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_secret_scope_warnings(&config);

    if json {
        print_warnings_json("secret_scope_warnings", &warnings);
    } else {
        print_warnings_text(
            "Secret scope warnings",
            "No secret scope issues found.",
            &warnings,
        );
    }
    Ok(())
}

// ============================================================================
// FJ-1057: Resource deprecation usage
// ============================================================================

/// Collect deprecated resource names (those with "deprecated" in their tags).
fn collect_deprecated_names(config: &types::ForjarConfig) -> Vec<String> {
    config
        .resources
        .iter()
        .filter(|(_, r)| {
            r.tags
                .iter()
                .any(|t| t.to_lowercase().contains("deprecated"))
        })
        .map(|(name, _)| name.clone())
        .collect()
}

/// Find resources whose depends_on references a deprecated resource.
fn find_deprecation_usage_warnings(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let deprecated = collect_deprecated_names(config);
    let mut warnings = Vec::new();

    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            if deprecated.contains(dep) {
                warnings.push((
                    name.clone(),
                    format!("depends on deprecated resource '{}'", dep),
                ));
            }
        }
    }
    warnings.sort_by(|a, b| a.0.cmp(&b.0));
    warnings
}

/// FJ-1057: Check that no resource depends on a deprecated resource.
///
/// Parses the config and identifies resources tagged with "deprecated".
/// Warns if any other resource lists a deprecated resource in its
/// `depends_on` field, indicating reliance on a resource scheduled for removal.
pub(crate) fn cmd_validate_check_resource_deprecation_usage(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_deprecation_usage_warnings(&config);

    if json {
        print_warnings_json("deprecation_usage_warnings", &warnings);
    } else {
        print_warnings_text(
            "Deprecation usage warnings",
            "No deprecation usage issues found.",
            &warnings,
        );
    }
    Ok(())
}

// ============================================================================
// FJ-1060: Resource when-condition coverage
// ============================================================================

/// Find resources whose dependents lack matching when conditions.
///
/// If resource A has a `when` condition but a resource B that depends on A
/// does NOT have a `when` condition, B will always be applied even when A
/// is conditionally skipped. This is likely unintentional.
fn find_when_condition_coverage_warnings(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut warnings = Vec::new();

    // Build a map: resource -> list of its dependents (resources that depend on it).
    let mut dependents_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            dependents_map
                .entry(dep.clone())
                .or_default()
                .push(name.clone());
        }
    }

    // For each resource with a `when` condition, check its dependents.
    for (name, resource) in &config.resources {
        if resource.when.is_none() {
            continue;
        }
        let dependents = match dependents_map.get(name) {
            Some(deps) => deps,
            None => continue,
        };
        for dep_name in dependents {
            let dep_resource = match config.resources.get(dep_name) {
                Some(r) => r,
                None => continue,
            };
            if dep_resource.when.is_none() {
                warnings.push((
                    dep_name.clone(),
                    format!(
                        "depends on '{}' which has a when condition, but has no when condition itself",
                        name
                    ),
                ));
            }
        }
    }
    warnings.sort_by(|a, b| a.0.cmp(&b.0));
    warnings
}

/// FJ-1060: Check that dependents of conditional resources also have when conditions.
///
/// Parses the config and finds resources with `when` conditions set. For each
/// such resource, checks whether its dependents (resources that list it in
/// `depends_on`) also have a `when` condition. Warns if a dependent lacks a
/// matching condition, since it would always run even when its dependency is
/// conditionally skipped.
pub(crate) fn cmd_validate_check_resource_when_condition_coverage(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_when_condition_coverage_warnings(&config);

    if json {
        print_warnings_json("when_condition_coverage_warnings", &warnings);
    } else {
        print_warnings_text(
            "When-condition coverage warnings",
            "All dependents of conditional resources have matching when conditions.",
            &warnings,
        );
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Deserialize a minimal resource from YAML, setting only the `type` field.
    fn make_resource(rtype: &str) -> types::Resource {
        let yaml = format!("type: {}", rtype);
        serde_yaml_ng::from_str(&yaml).unwrap()
    }

    /// Build a ForjarConfig from a list of `(name, resource)` pairs.
    fn make_config(resources: Vec<(&str, types::Resource)>) -> types::ForjarConfig {
        let mut map = indexmap::IndexMap::new();
        for (name, res) in resources {
            map.insert(name.to_string(), res);
        }
        let yaml = "version: '1.0'\nname: test\nresources: {}";
        let mut cfg: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        cfg.resources = map;
        cfg
    }

    // -- FJ-1054: Secret scope tests --

    #[test]
    fn test_secret_scope_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_secret_scope_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_secret_scope_no_warnings_single_machine() {
        let mut r = make_resource("file");
        r.tags = vec!["secret".to_string()];
        // Default machine is single "localhost" — no warning.
        let config = make_config(vec![("secret-cfg", r)]);
        let warnings = find_secret_scope_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_secret_scope_warns_multi_machine() {
        let mut r = make_resource("file");
        r.content = Some("password={{secret.db_pass}}".to_string());
        r.machine = types::MachineTarget::Multiple(vec!["web1".to_string(), "web2".to_string()]);
        let config = make_config(vec![("db-cfg", r)]);
        let warnings = find_secret_scope_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].0, "db-cfg");
        assert!(warnings[0].1.contains("2 machines"));
    }

    #[test]
    fn test_secret_scope_tag_detection() {
        let mut r = make_resource("file");
        r.tags = vec!["secret-keys".to_string()];
        r.machine = types::MachineTarget::Multiple(vec!["a".to_string(), "b".to_string()]);
        let config = make_config(vec![("keys", r)]);
        let warnings = find_secret_scope_warnings(&config);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_secret_scope_no_secret_multi_machine_ok() {
        let mut r = make_resource("file");
        r.machine = types::MachineTarget::Multiple(vec!["a".to_string(), "b".to_string()]);
        let config = make_config(vec![("public-cfg", r)]);
        let warnings = find_secret_scope_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_secret_scope_json_output() {
        let config = make_config(vec![]);
        let warnings = find_secret_scope_warnings(&config);
        print_warnings_json("secret_scope_warnings", &warnings);
    }

    // -- FJ-1057: Deprecation usage tests --

    #[test]
    fn test_deprecation_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_deprecation_usage_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_deprecation_no_warnings() {
        let r_old = make_resource("file");
        let r_new = make_resource("file");
        let config = make_config(vec![("old-svc", r_old), ("new-svc", r_new)]);
        let warnings = find_deprecation_usage_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_deprecation_warns_depends_on_deprecated() {
        let mut deprecated = make_resource("package");
        deprecated.tags = vec!["deprecated".to_string()];
        let mut consumer = make_resource("service");
        consumer.depends_on = vec!["old-pkg".to_string()];
        let config = make_config(vec![("old-pkg", deprecated), ("my-svc", consumer)]);
        let warnings = find_deprecation_usage_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].0, "my-svc");
        assert!(warnings[0].1.contains("old-pkg"));
    }

    #[test]
    fn test_deprecation_tag_case_insensitive() {
        let mut deprecated = make_resource("file");
        deprecated.tags = vec!["DEPRECATED-v1".to_string()];
        let mut consumer = make_resource("file");
        consumer.depends_on = vec!["legacy".to_string()];
        let config = make_config(vec![("legacy", deprecated), ("current", consumer)]);
        let warnings = find_deprecation_usage_warnings(&config);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_deprecation_json_output() {
        let config = make_config(vec![]);
        let warnings = find_deprecation_usage_warnings(&config);
        print_warnings_json("deprecation_usage_warnings", &warnings);
    }

    // -- FJ-1060: When-condition coverage tests --

    #[test]
    fn test_when_coverage_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_when_condition_coverage_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_when_coverage_no_warnings_both_conditional() {
        let mut base = make_resource("package");
        base.when = Some("{{params.env}} == \"prod\"".to_string());
        let mut dependent = make_resource("service");
        dependent.depends_on = vec!["base-pkg".to_string()];
        dependent.when = Some("{{params.env}} == \"prod\"".to_string());
        let config = make_config(vec![("base-pkg", base), ("my-svc", dependent)]);
        let warnings = find_when_condition_coverage_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_when_coverage_warns_dependent_missing_when() {
        let mut base = make_resource("package");
        base.when = Some("{{params.env}} == \"prod\"".to_string());
        let mut dependent = make_resource("service");
        dependent.depends_on = vec!["base-pkg".to_string()];
        // dependent has no when — should warn.
        let config = make_config(vec![("base-pkg", base), ("my-svc", dependent)]);
        let warnings = find_when_condition_coverage_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].0, "my-svc");
        assert!(warnings[0].1.contains("base-pkg"));
    }

    #[test]
    fn test_when_coverage_no_dependents_no_warning() {
        let mut conditional = make_resource("file");
        conditional.when = Some("{{machine.arch}} == \"arm64\"".to_string());
        // No other resource depends on it — no warning.
        let config = make_config(vec![("arm-cfg", conditional)]);
        let warnings = find_when_condition_coverage_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_when_coverage_json_output() {
        let config = make_config(vec![]);
        let warnings = find_when_condition_coverage_warnings(&config);
        print_warnings_json("when_condition_coverage_warnings", &warnings);
    }
}
