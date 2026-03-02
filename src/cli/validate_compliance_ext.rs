//! Phase 98 — Compliance Automation & Drift Intelligence: validate commands.

#![allow(dead_code)]

use crate::core::types;
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// Compliance tag keywords
// ============================================================================

const COMPLIANCE_KEYWORDS: &[&str] = &["pci", "hipaa", "sox", "gdpr", "iso27001"];

/// Check if a tag contains any compliance keyword (case-insensitive).
fn tag_has_compliance_keyword(tag: &str) -> bool {
    let lower = tag.to_lowercase();
    COMPLIANCE_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

// ============================================================================
// FJ-1046: Resource compliance tags
// ============================================================================

/// Collect resources that have no compliance-related tags.
fn find_resources_missing_compliance_tags(config: &types::ForjarConfig) -> Vec<String> {
    let mut missing = Vec::new();
    for (name, resource) in &config.resources {
        let has_compliance = resource.tags.iter().any(|t| tag_has_compliance_keyword(t));
        if !has_compliance {
            missing.push(name.clone());
        }
    }
    missing.sort();
    missing
}

/// Print compliance-tag warnings as JSON.
fn print_compliance_tags_json(missing: &[String]) {
    let items: Vec<String> = missing.iter().map(|n| format!("\"{}\"", n)).collect();
    println!(
        r#"{{"compliance_tag_warnings":[{}],"count":{}}}"#,
        items.join(","),
        missing.len()
    );
}

/// Print compliance-tag warnings as text.
fn print_compliance_tags_text(missing: &[String]) {
    if missing.is_empty() {
        println!("All resources have compliance-related tags.");
    } else {
        println!("Resources missing compliance tags ({}):", missing.len());
        for name in missing {
            println!(
                "  warning: {} has no tags matching compliance keywords (pci, hipaa, sox, gdpr, iso27001)",
                name
            );
        }
    }
}

/// FJ-1046: Check that resources carry compliance-related tags.
///
/// Parses the config and checks each resource for tags containing
/// compliance keywords (pci, hipaa, sox, gdpr, iso27001). Resources
/// without any such tag produce a warning.
pub(crate) fn cmd_validate_check_resource_compliance_tags(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let missing = find_resources_missing_compliance_tags(&config);

    if json {
        print_compliance_tags_json(&missing);
    } else {
        print_compliance_tags_text(&missing);
    }
    Ok(())
}

// ============================================================================
// FJ-1049: Resource rollback coverage
// ============================================================================

/// Check a single resource for rollback mechanism coverage.
/// Returns `Some(reason)` if the resource lacks coverage.
fn check_rollback_coverage(resource: &types::Resource) -> Option<String> {
    let has_pre = resource.pre_apply.is_some();
    let has_post = resource.post_apply.is_some();
    if has_pre || has_post {
        return None;
    }
    Some("no pre_apply or post_apply hook defined".to_string())
}

/// Collect service/package resources that lack rollback mechanisms.
fn find_resources_missing_rollback(config: &types::ForjarConfig) -> Vec<(String, String)> {
    let mut warnings = Vec::new();
    for (name, resource) in &config.resources {
        let needs_check = matches!(
            resource.resource_type,
            types::ResourceType::Service | types::ResourceType::Package
        );
        if !needs_check {
            continue;
        }
        if let Some(reason) = check_rollback_coverage(resource) {
            warnings.push((name.clone(), reason));
        }
    }
    warnings.sort_by(|a, b| a.0.cmp(&b.0));
    warnings
}

/// Print rollback-coverage warnings as JSON.
fn print_rollback_json(warnings: &[(String, String)]) {
    let items: Vec<String> = warnings
        .iter()
        .map(|(name, reason)| {
            format!(
                r#"{{"resource":"{}","type_requires_rollback":true,"reason":"{}"}}"#,
                name, reason
            )
        })
        .collect();
    println!(
        r#"{{"rollback_coverage_warnings":[{}],"count":{}}}"#,
        items.join(","),
        warnings.len()
    );
}

/// Print rollback-coverage warnings as text.
fn print_rollback_text(warnings: &[(String, String)]) {
    if warnings.is_empty() {
        println!("All service and package resources have rollback coverage.");
    } else {
        println!("Resources lacking rollback coverage ({}):", warnings.len());
        for (name, reason) in warnings {
            println!("  warning: {} — {}", name, reason);
        }
    }
}

/// FJ-1049: Check that service and package resources have rollback mechanisms.
///
/// Parses the config and checks each resource of type `service` or `package`
/// for the presence of `pre_apply` or `post_apply` lifecycle hooks. Resources
/// without any such hook produce a warning about missing rollback coverage.
pub(crate) fn cmd_validate_check_resource_rollback_coverage(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_resources_missing_rollback(&config);

    if json {
        print_rollback_json(&warnings);
    } else {
        print_rollback_text(&warnings);
    }
    Ok(())
}

// ============================================================================
// FJ-1052: Resource dependency balance
// ============================================================================

const MAX_FAN: usize = 5;

/// Fan-in/fan-out entry for a single resource.
struct FanMetrics {
    name: String,
    fan_in: usize,
    fan_out: usize,
    detail: String,
}

/// Compute fan-in and fan-out for all resources.
fn compute_fan_metrics(config: &types::ForjarConfig) -> HashMap<String, (usize, usize)> {
    let mut fan_in: HashMap<String, usize> = HashMap::new();
    let mut fan_out: HashMap<String, usize> = HashMap::new();

    // Initialize all resources to zero.
    for name in config.resources.keys() {
        fan_in.entry(name.clone()).or_insert(0);
        fan_out.entry(name.clone()).or_insert(0);
    }

    // Count: for each dependency edge A -> B (A depends_on B),
    // B's fan-in increases, A's fan-out increases.
    for (name, resource) in &config.resources {
        let out = resource.depends_on.len();
        *fan_out.entry(name.clone()).or_insert(0) += out;
        for dep in &resource.depends_on {
            *fan_in.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    let mut result = HashMap::new();
    for name in config.resources.keys() {
        let fi = fan_in.get(name).copied().unwrap_or(0);
        let fo = fan_out.get(name).copied().unwrap_or(0);
        result.insert(name.clone(), (fi, fo));
    }
    result
}

/// Find resources with imbalanced fan-in or fan-out.
fn find_imbalanced_resources(config: &types::ForjarConfig) -> Vec<FanMetrics> {
    let metrics = compute_fan_metrics(config);
    let mut warnings = Vec::new();

    for (name, &(fi, fo)) in &metrics {
        let mut reasons = Vec::new();
        if fi > MAX_FAN {
            reasons.push(format!("fan-in {} exceeds {}", fi, MAX_FAN));
        }
        if fo > MAX_FAN {
            reasons.push(format!("fan-out {} exceeds {}", fo, MAX_FAN));
        }
        if !reasons.is_empty() {
            warnings.push(FanMetrics {
                name: name.clone(),
                fan_in: fi,
                fan_out: fo,
                detail: reasons.join("; "),
            });
        }
    }

    warnings.sort_by(|a, b| a.name.cmp(&b.name));
    warnings
}

/// Print dependency-balance warnings as JSON.
fn print_balance_json(warnings: &[FanMetrics]) {
    let items: Vec<String> = warnings
        .iter()
        .map(|m| {
            format!(
                r#"{{"resource":"{}","fan_in":{},"fan_out":{},"detail":"{}"}}"#,
                m.name, m.fan_in, m.fan_out, m.detail
            )
        })
        .collect();
    println!(
        r#"{{"dependency_balance_warnings":[{}],"count":{}}}"#,
        items.join(","),
        warnings.len()
    );
}

/// Print dependency-balance warnings as text.
fn print_balance_text(warnings: &[FanMetrics]) {
    if warnings.is_empty() {
        println!("All resources have balanced dependency fan-in/fan-out.");
    } else {
        println!("Imbalanced dependency resources ({}):", warnings.len());
        for m in warnings {
            println!(
                "  warning: {} (fan-in={}, fan-out={}) — {}",
                m.name, m.fan_in, m.fan_out, m.detail
            );
        }
    }
}

/// FJ-1052: Check resource dependency balance (fan-in / fan-out).
///
/// Parses the config and computes fan-in (how many resources depend on each
/// resource) and fan-out (how many dependencies each resource declares).
/// Warns if any resource exceeds a fan-in or fan-out threshold of 5.
pub(crate) fn cmd_validate_check_resource_dependency_balance(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&content).map_err(|e| e.to_string())?;

    let warnings = find_imbalanced_resources(&config);

    if json {
        print_balance_json(&warnings);
    } else {
        print_balance_text(&warnings);
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_has_compliance_keyword() {
        assert!(tag_has_compliance_keyword("pci-dss"));
        assert!(tag_has_compliance_keyword("HIPAA"));
        assert!(tag_has_compliance_keyword("SOX-audit"));
        assert!(tag_has_compliance_keyword("gdpr-eu"));
        assert!(tag_has_compliance_keyword("iso27001"));
        assert!(!tag_has_compliance_keyword("production"));
        assert!(!tag_has_compliance_keyword("web"));
    }

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

    #[test]
    fn test_compliance_tags_all_present() {
        let mut r = make_resource("file");
        r.tags = vec!["hipaa-compliant".to_string()];
        let config = make_config(vec![("myfile", r)]);
        let missing = find_resources_missing_compliance_tags(&config);
        assert!(missing.is_empty());
    }

    #[test]
    fn test_compliance_tags_missing() {
        let r = make_resource("file");
        let config = make_config(vec![("myfile", r)]);
        let missing = find_resources_missing_compliance_tags(&config);
        assert_eq!(missing, vec!["myfile"]);
    }

    #[test]
    fn test_rollback_coverage_with_hooks() {
        let mut r = make_resource("service");
        r.pre_apply = Some("systemctl snapshot".to_string());
        let config = make_config(vec![("svc", r)]);
        let warnings = find_resources_missing_rollback(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_rollback_coverage_missing() {
        let r = make_resource("package");
        let config = make_config(vec![("pkg", r)]);
        let warnings = find_resources_missing_rollback(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].0, "pkg");
    }

    #[test]
    fn test_rollback_skips_non_service_package() {
        let r = make_resource("file");
        let config = make_config(vec![("f", r)]);
        let warnings = find_resources_missing_rollback(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_dependency_balance_ok() {
        let mut a = make_resource("file");
        a.depends_on = vec!["b".to_string()];
        let b = make_resource("file");
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_imbalanced_resources(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_dependency_balance_high_fan_in() {
        let target = make_resource("file");
        let mut resources: Vec<(&str, types::Resource)> = Vec::new();
        resources.push(("target", target));
        // Create 6 resources all depending on "target" to exceed fan-in=5.
        let names = ["a", "b", "c", "d", "e", "f"];
        for name in &names {
            let mut r = make_resource("file");
            r.depends_on = vec!["target".to_string()];
            resources.push((name, r));
        }
        let config = make_config(resources);
        let warnings = find_imbalanced_resources(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].name, "target");
        assert_eq!(warnings[0].fan_in, 6);
    }

    #[test]
    fn test_dependency_balance_high_fan_out() {
        let mut hub = make_resource("file");
        hub.depends_on = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
            "e".to_string(),
            "f".to_string(),
        ];
        let mut resources: Vec<(&str, types::Resource)> = vec![("hub", hub)];
        for name in &["a", "b", "c", "d", "e", "f"] {
            resources.push((name, make_resource("file")));
        }
        let config = make_config(resources);
        let warnings = find_imbalanced_resources(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].name, "hub");
        assert_eq!(warnings[0].fan_out, 6);
    }
}
