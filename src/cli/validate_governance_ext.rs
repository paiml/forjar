//! Phase 101 — Fleet Insight & Dependency Quality: validate commands (FJ-1070, FJ-1073, FJ-1076).

#![allow(dead_code)]

use crate::core::types;
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// FJ-1070: Resource dependency fan-out limit
// ============================================================================

/// Default threshold for fan-out (number of direct dependents).
const DEFAULT_FAN_OUT_THRESHOLD: usize = 10;

/// A warning about a resource whose fan-out exceeds the threshold.
struct FanOutWarning {
    resource: String,
    fan_out: usize,
    threshold: usize,
}

/// Count how many other resources depend on each resource (fan-out = dependent count).
fn find_fan_out_warnings(
    config: &types::ForjarConfig,
    threshold: usize,
) -> Vec<FanOutWarning> {
    let mut dependent_counts: HashMap<String, usize> = HashMap::new();
    for resource in config.resources.values() {
        for dep in &resource.depends_on {
            *dependent_counts.entry(dep.clone()).or_insert(0) += 1;
        }
    }
    let mut warnings = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let count = dependent_counts.get(name.as_str()).copied().unwrap_or(0);
        if count > threshold {
            warnings.push(FanOutWarning {
                resource: name.clone(),
                fan_out: count,
                threshold,
            });
        }
    }
    warnings
}

/// FJ-1070: Warn if any resource has more than N direct dependents (default 10).
///
/// Parses the config and counts how many other resources list each resource in
/// their `depends_on`. Resources exceeding the threshold are flagged, as high
/// fan-out indicates a coupling bottleneck.
pub(crate) fn cmd_validate_check_resource_dependency_fan_out_limit(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let warnings = find_fan_out_warnings(&cfg, DEFAULT_FAN_OUT_THRESHOLD);

    if json {
        let items: Vec<serde_json::Value> = warnings
            .iter()
            .map(|w| {
                serde_json::json!({
                    "resource": w.resource,
                    "fan_out": w.fan_out,
                    "threshold": w.threshold
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({ "fan_out_warnings": items, "count": warnings.len() })
        );
    } else if warnings.is_empty() {
        println!("No dependency fan-out warnings found.");
    } else {
        println!("Dependency fan-out warnings ({}):", warnings.len());
        for w in &warnings {
            println!(
                "  warning: resource '{}' has {} dependents (threshold: {})",
                w.resource, w.fan_out, w.threshold
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1073: Resource tag required keys
// ============================================================================

/// Required tag namespace prefixes that every resource should have.
const REQUIRED_TAG_NAMESPACES: &[&str] = &["env", "team", "tier"];

/// A warning about a resource missing required tag namespaces.
struct TagRequiredKeysWarning {
    resource: String,
    missing_namespaces: Vec<String>,
}

/// Check if a resource's tags contain the expected namespace prefix.
fn has_tag_namespace(tags: &[String], namespace: &str) -> bool {
    let prefix = format!("{}:", namespace);
    tags.iter().any(|t| t.starts_with(&prefix))
}

/// Find resources missing required tag namespaces.
fn find_tag_required_keys_warnings(config: &types::ForjarConfig) -> Vec<TagRequiredKeysWarning> {
    let mut warnings = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        let missing: Vec<String> = REQUIRED_TAG_NAMESPACES
            .iter()
            .filter(|ns| !has_tag_namespace(&resource.tags, ns))
            .map(|ns| (*ns).to_string())
            .collect();
        if !missing.is_empty() {
            warnings.push(TagRequiredKeysWarning {
                resource: name.clone(),
                missing_namespaces: missing,
            });
        }
    }
    warnings
}

/// FJ-1073: Warn if resources lack required tag keys (env, team, tier).
///
/// Parses the config and checks each resource's tags for the presence of
/// expected namespace prefixes (`env:`, `team:`, `tier:`). Resources missing
/// any of these namespaces are flagged for governance compliance.
pub(crate) fn cmd_validate_check_resource_tag_required_keys(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let warnings = find_tag_required_keys_warnings(&cfg);

    if json {
        let items: Vec<serde_json::Value> = warnings
            .iter()
            .map(|w| {
                serde_json::json!({
                    "resource": w.resource,
                    "missing_namespaces": w.missing_namespaces
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({ "tag_warnings": items, "count": warnings.len() })
        );
    } else if warnings.is_empty() {
        println!("All resources have required tag namespaces.");
    } else {
        println!("Tag required-key warnings ({}):", warnings.len());
        for w in &warnings {
            println!(
                "  warning: resource '{}' missing tag namespaces: {}",
                w.resource,
                w.missing_namespaces.join(", ")
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1076: Resource content drift risk
// ============================================================================

/// Drift risk entry for a resource.
struct DriftRiskEntry {
    resource: String,
    resource_type: String,
    base_risk: usize,
    dependency_count: usize,
    dependent_count: usize,
    total_risk: usize,
}

/// Assign a base drift-risk score by resource type.
fn base_risk_for_type(rt: &types::ResourceType) -> usize {
    match rt {
        types::ResourceType::File => 3,
        types::ResourceType::Service => 4,
        types::ResourceType::Package => 2,
        _ => 1,
    }
}

/// Build a map of resource name to dependent count.
fn build_dependent_counts(config: &types::ForjarConfig) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for resource in config.resources.values() {
        for dep in &resource.depends_on {
            *counts.entry(dep.clone()).or_insert(0) += 1;
        }
    }
    counts
}

/// Score drift risk for each resource.
fn compute_drift_risk(config: &types::ForjarConfig) -> Vec<DriftRiskEntry> {
    let dependent_counts = build_dependent_counts(config);
    let mut entries = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        let base = base_risk_for_type(&resource.resource_type);
        let dep_count = resource.depends_on.len();
        let depnt_count = dependent_counts.get(name.as_str()).copied().unwrap_or(0);
        let total = base + dep_count + depnt_count;
        entries.push(DriftRiskEntry {
            resource: name.clone(),
            resource_type: resource.resource_type.to_string(),
            base_risk: base,
            dependency_count: dep_count,
            dependent_count: depnt_count,
            total_risk: total,
        });
    }
    entries
}

/// FJ-1076: Score drift risk based on resource type, content volatility, and dependency count.
///
/// Parses the config and assigns a base risk score by resource type
/// (file=3, service=4, package=2, other=1). Adds 1 per dependency
/// (`depends_on` count) and 1 per dependent (how many others depend on it).
/// Reports the total risk score for each resource.
pub(crate) fn cmd_validate_check_resource_content_drift_risk(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let entries = compute_drift_risk(&cfg);

    if json {
        let items: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "resource": e.resource,
                    "resource_type": e.resource_type,
                    "base_risk": e.base_risk,
                    "dependency_count": e.dependency_count,
                    "dependent_count": e.dependent_count,
                    "total_risk": e.total_risk
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "drift_risk": items }));
    } else if entries.is_empty() {
        println!("No resources to assess for drift risk.");
    } else {
        println!("Drift risk assessment ({} resources):", entries.len());
        for e in &entries {
            println!(
                "  {} ({}): risk={} (base={}, deps={}, dependents={})",
                e.resource, e.resource_type, e.total_risk, e.base_risk,
                e.dependency_count, e.dependent_count
            );
        }
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

    // -- FJ-1070: Fan-out tests --

    #[test]
    fn test_fan_out_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_fan_out_warnings(&config, 10);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_fan_out_under_threshold() {
        let a = make_resource("package");
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_fan_out_warnings(&config, 10);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_fan_out_over_threshold() {
        let base = make_resource("package");
        let mut resources: Vec<(&str, types::Resource)> = vec![("base", base)];
        let names: Vec<String> = (0..12).map(|i| format!("svc-{}", i)).collect();
        for name in &names {
            let mut r = make_resource("service");
            r.depends_on = vec!["base".to_string()];
            resources.push((name.as_str(), r));
        }
        let config = make_config(resources);
        let warnings = find_fan_out_warnings(&config, 10);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].resource, "base");
        assert_eq!(warnings[0].fan_out, 12);
    }

    #[test]
    fn test_fan_out_exact_threshold_no_warning() {
        let base = make_resource("package");
        let mut resources: Vec<(&str, types::Resource)> = vec![("base", base)];
        let names: Vec<String> = (0..10).map(|i| format!("svc-{}", i)).collect();
        for name in &names {
            let mut r = make_resource("service");
            r.depends_on = vec!["base".to_string()];
            resources.push((name.as_str(), r));
        }
        let config = make_config(resources);
        let warnings = find_fan_out_warnings(&config, 10);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_fan_out_no_dependencies() {
        let a = make_resource("file");
        let b = make_resource("service");
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_fan_out_warnings(&config, 10);
        assert!(warnings.is_empty());
    }

    // -- FJ-1073: Tag required keys tests --

    #[test]
    fn test_tag_required_keys_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_tag_required_keys_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_tag_required_keys_all_present() {
        let mut r = make_resource("file");
        r.tags = vec![
            "env:prod".to_string(),
            "team:infra".to_string(),
            "tier:web".to_string(),
        ];
        let config = make_config(vec![("app", r)]);
        let warnings = find_tag_required_keys_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_tag_required_keys_missing_some() {
        let mut r = make_resource("service");
        r.tags = vec!["env:staging".to_string()];
        let config = make_config(vec![("svc", r)]);
        let warnings = find_tag_required_keys_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].resource, "svc");
        assert_eq!(warnings[0].missing_namespaces, vec!["team", "tier"]);
    }

    #[test]
    fn test_tag_required_keys_missing_all() {
        let r = make_resource("package");
        let config = make_config(vec![("pkg", r)]);
        let warnings = find_tag_required_keys_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(
            warnings[0].missing_namespaces,
            vec!["env", "team", "tier"]
        );
    }

    #[test]
    fn test_tag_required_keys_partial_match_not_counted() {
        let mut r = make_resource("file");
        // "environment" does not start with "env:"
        r.tags = vec!["environment".to_string()];
        let config = make_config(vec![("cfg", r)]);
        let warnings = find_tag_required_keys_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].missing_namespaces.contains(&"env".to_string()));
    }

    // -- FJ-1076: Drift risk tests --

    #[test]
    fn test_drift_risk_empty_config() {
        let config = make_config(vec![]);
        let entries = compute_drift_risk(&config);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_drift_risk_base_scores() {
        let f = make_resource("file");
        let s = make_resource("service");
        let p = make_resource("package");
        let m = make_resource("mount");
        let config = make_config(vec![("f", f), ("m", m), ("p", p), ("s", s)]);
        let entries = compute_drift_risk(&config);
        // Sorted: f, m, p, s
        assert_eq!(entries[0].resource, "f");
        assert_eq!(entries[0].base_risk, 3);
        assert_eq!(entries[0].total_risk, 3);
        assert_eq!(entries[1].resource, "m");
        assert_eq!(entries[1].base_risk, 1);
        assert_eq!(entries[2].resource, "p");
        assert_eq!(entries[2].base_risk, 2);
        assert_eq!(entries[3].resource, "s");
        assert_eq!(entries[3].base_risk, 4);
    }

    #[test]
    fn test_drift_risk_with_dependencies() {
        let a = make_resource("package");
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let entries = compute_drift_risk(&config);
        // "a": base=2, deps=0, dependents=1, total=3
        assert_eq!(entries[0].resource, "a");
        assert_eq!(entries[0].total_risk, 3);
        assert_eq!(entries[0].dependent_count, 1);
        // "b": base=4, deps=1, dependents=0, total=5
        assert_eq!(entries[1].resource, "b");
        assert_eq!(entries[1].total_risk, 5);
        assert_eq!(entries[1].dependency_count, 1);
    }

    #[test]
    fn test_drift_risk_high_fan_out_increases_risk() {
        let base = make_resource("file");
        let mut resources: Vec<(&str, types::Resource)> = vec![("base", base)];
        let names: Vec<String> = (0..5).map(|i| format!("dep-{}", i)).collect();
        for name in &names {
            let mut r = make_resource("service");
            r.depends_on = vec!["base".to_string()];
            resources.push((name.as_str(), r));
        }
        let config = make_config(resources);
        let entries = compute_drift_risk(&config);
        let base_entry = entries.iter().find(|e| e.resource == "base").unwrap();
        // base: base_risk=3 + 0 deps + 5 dependents = 8
        assert_eq!(base_entry.total_risk, 8);
        assert_eq!(base_entry.dependent_count, 5);
    }
}
