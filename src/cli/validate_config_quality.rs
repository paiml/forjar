//! Phase 103 — Fleet Analytics & Configuration Quality: validate commands (FJ-1086, FJ-1089, FJ-1092).

use crate::core::types;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

// ============================================================================
// FJ-1086: Resource dependency isolation
// ============================================================================

/// A warning about two resources with different tags sharing a dependency edge.
struct DependencyIsolationWarning {
    resource: String,
    dependency: String,
    resource_tags: Vec<String>,
    dependency_tags: Vec<String>,
}

/// Find dependency edges where the two resources have different tag sets.
fn find_dependency_isolation_warnings(
    config: &types::ForjarConfig,
) -> Vec<DependencyIsolationWarning> {
    let mut warnings = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        let res_tags: BTreeSet<&String> = resource.tags.iter().collect();
        for dep_name in &resource.depends_on {
            if let Some(dep_resource) = config.resources.get(dep_name) {
                let dep_tags: BTreeSet<&String> = dep_resource.tags.iter().collect();
                if !res_tags.is_empty() && !dep_tags.is_empty() && res_tags != dep_tags {
                    warnings.push(DependencyIsolationWarning {
                        resource: name.clone(),
                        dependency: dep_name.clone(),
                        resource_tags: resource.tags.clone(),
                        dependency_tags: dep_resource.tags.clone(),
                    });
                }
            }
        }
    }
    warnings
}

/// FJ-1086: Warn if resources with different tags share dependencies.
///
/// Parses the config and examines each dependency edge. If both the resource and
/// its dependency have tags, but the tag sets differ, a warning is emitted. This
/// indicates resources in different lifecycle stages sharing dependencies.
pub(crate) fn cmd_validate_check_resource_dependency_isolation(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let warnings = find_dependency_isolation_warnings(&cfg);

    if json {
        let items: Vec<serde_json::Value> = warnings
            .iter()
            .map(|w| {
                serde_json::json!({
                    "resource": w.resource,
                    "dependency": w.dependency,
                    "resource_tags": w.resource_tags,
                    "dependency_tags": w.dependency_tags
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({ "isolation_warnings": items, "count": warnings.len() })
        );
    } else if warnings.is_empty() {
        println!("No dependency isolation warnings found.");
    } else {
        println!("Dependency isolation warnings ({}):", warnings.len());
        for w in &warnings {
            println!(
                "  warning: resource '{}' (tags: {}) depends on '{}' (tags: {})",
                w.resource,
                w.resource_tags.join(", "),
                w.dependency,
                w.dependency_tags.join(", ")
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1089: Resource tag value consistency
// ============================================================================

/// A warning about inconsistent tag values across resources of the same type.
struct TagConsistencyWarning {
    resource_type: String,
    resource: String,
    tags: Vec<String>,
    expected_tags: Vec<String>,
}

/// Group resources by type and find tag inconsistencies within each group.
fn find_tag_consistency_warnings(config: &types::ForjarConfig) -> Vec<TagConsistencyWarning> {
    // Group resources by type
    let mut by_type: HashMap<String, Vec<(&String, &types::Resource)>> = HashMap::new();
    for (name, resource) in &config.resources {
        let rt = resource.resource_type.to_string();
        by_type.entry(rt).or_default().push((name, resource));
    }

    let mut warnings = Vec::new();
    let mut type_keys: Vec<&String> = by_type.keys().collect();
    type_keys.sort();

    for rt in type_keys {
        let group = &by_type[rt];
        if group.len() < 2 {
            continue;
        }
        // Use the first resource's tag set as the reference
        let reference_tags: BTreeSet<&String> = group[0].1.tags.iter().collect();
        let reference_sorted: Vec<String> = reference_tags.iter().map(|t| (*t).clone()).collect();

        for &(name, resource) in &group[1..] {
            let current_tags: BTreeSet<&String> = resource.tags.iter().collect();
            if !current_tags.is_empty()
                && !reference_tags.is_empty()
                && current_tags != reference_tags
            {
                warnings.push(TagConsistencyWarning {
                    resource_type: rt.clone(),
                    resource: name.clone(),
                    tags: resource.tags.clone(),
                    expected_tags: reference_sorted.clone(),
                });
            }
        }
    }
    warnings
}

/// FJ-1089: Warn if tag values are inconsistent across resources of the same type.
///
/// Parses the config, groups resources by type, and compares tag sets within each
/// group. If a resource's tags differ from the first resource of the same type,
/// a warning is emitted indicating inconsistent lifecycle tagging.
pub(crate) fn cmd_validate_check_resource_tag_value_consistency(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let warnings = find_tag_consistency_warnings(&cfg);

    if json {
        let items: Vec<serde_json::Value> = warnings
            .iter()
            .map(|w| {
                serde_json::json!({
                    "resource_type": w.resource_type,
                    "resource": w.resource,
                    "tags": w.tags,
                    "expected_tags": w.expected_tags
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({ "tag_consistency_warnings": items, "count": warnings.len() })
        );
    } else if warnings.is_empty() {
        println!("All resources have consistent tag values within their type.");
    } else {
        println!("Tag consistency warnings ({}):", warnings.len());
        for w in &warnings {
            println!(
                "  warning: {} resource '{}' has tags [{}], expected [{}]",
                w.resource_type,
                w.resource,
                w.tags.join(", "),
                w.expected_tags.join(", ")
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1092: Resource machine distribution balance
// ============================================================================

/// Collect per-machine resource counts and determine balance.
fn compute_distribution_balance(config: &types::ForjarConfig) -> (HashMap<String, usize>, bool) {
    let mut per_machine: HashMap<String, usize> = HashMap::new();
    for resource in config.resources.values() {
        let machines = resource.machine.to_vec();
        for m in machines {
            *per_machine.entry(m).or_insert(0) += 1;
        }
    }

    let balanced = if per_machine.len() < 2 {
        true
    } else {
        let counts: Vec<usize> = per_machine.values().copied().collect();
        let min_count = counts.iter().copied().min().unwrap_or(1).max(1);
        let max_count = counts.iter().copied().max().unwrap_or(1);
        // Balanced if max/min ratio <= 3
        max_count <= min_count * 3
    };

    (per_machine, balanced)
}

/// FJ-1092: Warn if resources are unevenly distributed across machines.
///
/// Parses the config and counts resources per machine. Computes the max/min ratio;
/// if the ratio exceeds 3, the distribution is flagged as unbalanced. Reports
/// per-machine counts and overall balance status.
pub(crate) fn cmd_validate_check_resource_machine_distribution_balance(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let (per_machine, balanced) = compute_distribution_balance(&cfg);

    if json {
        let machine_map: serde_json::Value = per_machine
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::json!(v)))
            .collect::<serde_json::Map<String, serde_json::Value>>()
            .into();
        println!(
            "{}",
            serde_json::json!({
                "distribution_balance": {
                    "per_machine": machine_map,
                    "balanced": balanced
                }
            })
        );
    } else if per_machine.is_empty() {
        println!("No machine assignments found.");
    } else {
        println!("Machine distribution balance (balanced: {balanced}):");
        let mut sorted_machines: Vec<(&String, &usize)> = per_machine.iter().collect();
        sorted_machines.sort_by_key(|(k, _)| (*k).clone());
        for (machine, count) in &sorted_machines {
            println!("  {machine}: {count} resources");
        }
        if !balanced {
            println!("  warning: resource distribution is unbalanced (max/min ratio > 3)");
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
        let yaml = format!("type: {rtype}");
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

    // -- FJ-1086: Dependency isolation tests --

    #[test]
    fn test_dependency_isolation_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_dependency_isolation_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_dependency_isolation_same_tags_no_warning() {
        let (mut a, mut b) = (make_resource("package"), make_resource("service"));
        a.tags = vec!["env:prod".to_string()];
        b.tags = vec!["env:prod".to_string()];
        b.depends_on = vec!["a".to_string()];
        let warnings = find_dependency_isolation_warnings(&make_config(vec![("a", a), ("b", b)]));
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_dependency_isolation_different_tags_warns() {
        let (mut a, mut b) = (make_resource("package"), make_resource("service"));
        a.tags = vec!["env:staging".to_string()];
        b.tags = vec!["env:prod".to_string()];
        b.depends_on = vec!["a".to_string()];
        let warnings = find_dependency_isolation_warnings(&make_config(vec![("a", a), ("b", b)]));
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].resource, "b");
        assert_eq!(warnings[0].dependency, "a");
    }

    #[test]
    fn test_dependency_isolation_empty_tags_no_warning() {
        let a = make_resource("package");
        let mut b = make_resource("service");
        b.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_dependency_isolation_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_dependency_isolation_one_empty_tags_no_warning() {
        let a = make_resource("package");
        let mut b = make_resource("service");
        b.tags = vec!["env:prod".to_string()];
        b.depends_on = vec!["a".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_dependency_isolation_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_dependency_isolation_missing_dependency_no_panic() {
        let mut a = make_resource("package");
        a.tags = vec!["env:prod".to_string()];
        a.depends_on = vec!["nonexistent".to_string()];
        let config = make_config(vec![("a", a)]);
        let warnings = find_dependency_isolation_warnings(&config);
        assert!(warnings.is_empty());
    }

    // -- FJ-1089: Tag value consistency tests --

    #[test]
    fn test_tag_consistency_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_tag_consistency_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_tag_consistency_single_resource_per_type() {
        let mut a = make_resource("file");
        a.tags = vec!["env:prod".to_string()];
        let config = make_config(vec![("a", a)]);
        let warnings = find_tag_consistency_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_tag_consistency_same_tags_no_warning() {
        let mut a = make_resource("service");
        a.tags = vec!["env:prod".to_string(), "tier:web".to_string()];
        let mut b = make_resource("service");
        b.tags = vec!["env:prod".to_string(), "tier:web".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_tag_consistency_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_tag_consistency_different_tags_warns() {
        let mut a = make_resource("service");
        a.tags = vec!["env:prod".to_string()];
        let mut b = make_resource("service");
        b.tags = vec!["env:staging".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_tag_consistency_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].resource_type, "service");
        assert_eq!(warnings[0].resource, "b");
    }

    #[test]
    fn test_tag_consistency_empty_tags_no_warning() {
        let a = make_resource("package");
        let b = make_resource("package");
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_tag_consistency_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_tag_consistency_different_types_no_cross_warning() {
        let mut a = make_resource("file");
        a.tags = vec!["env:prod".to_string()];
        let mut b = make_resource("service");
        b.tags = vec!["env:staging".to_string()];
        let config = make_config(vec![("a", a), ("b", b)]);
        let warnings = find_tag_consistency_warnings(&config);
        assert!(warnings.is_empty());
    }

    // -- FJ-1092: Machine distribution balance tests --

    #[test]
    fn test_distribution_balance_empty_config() {
        let config = make_config(vec![]);
        let (per_machine, balanced) = compute_distribution_balance(&config);
        assert!(per_machine.is_empty());
        assert!(balanced);
    }

    #[test]
    fn test_distribution_balance_single_machine() {
        let a = make_resource("package");
        let b = make_resource("service");
        let config = make_config(vec![("a", a), ("b", b)]);
        let (per_machine, balanced) = compute_distribution_balance(&config);
        assert_eq!(per_machine.len(), 1);
        assert_eq!(per_machine["localhost"], 2);
        assert!(balanced);
    }

    #[test]
    fn test_distribution_balance_even() {
        let (mut a, mut b) = (make_resource("package"), make_resource("service"));
        a.machine = types::MachineTarget::Single("web1".to_string());
        b.machine = types::MachineTarget::Single("web1".to_string());
        let (mut c, mut d) = (make_resource("file"), make_resource("package"));
        c.machine = types::MachineTarget::Single("web2".to_string());
        d.machine = types::MachineTarget::Single("web2".to_string());
        let config = make_config(vec![("a", a), ("b", b), ("c", c), ("d", d)]);
        let (pm, balanced) = compute_distribution_balance(&config);
        assert_eq!(pm["web1"], 2);
        assert_eq!(pm["web2"], 2);
        assert!(balanced);
    }

    #[test]
    fn test_distribution_balance_unbalanced() {
        let mut a = make_resource("package");
        a.machine = types::MachineTarget::Single("web1".to_string());
        let mut resources: Vec<(&str, types::Resource)> = vec![("a", a)];
        let names: Vec<String> = (0..4).map(|i| format!("svc-{i}")).collect();
        for name in &names {
            let mut r = make_resource("service");
            r.machine = types::MachineTarget::Single("web2".to_string());
            resources.push((name.as_str(), r));
        }
        let (_, balanced) = compute_distribution_balance(&make_config(resources));
        assert!(!balanced);
    }

    #[test]
    fn test_distribution_balance_ratio_at_boundary() {
        let (mut a, mut b) = (make_resource("package"), make_resource("service"));
        a.machine = types::MachineTarget::Single("m1".to_string());
        b.machine = types::MachineTarget::Single("m2".to_string());
        let (mut c, mut d) = (make_resource("file"), make_resource("package"));
        c.machine = types::MachineTarget::Single("m2".to_string());
        d.machine = types::MachineTarget::Single("m2".to_string());
        let config = make_config(vec![("a", a), ("b", b), ("c", c), ("d", d)]);
        let (pm, balanced) = compute_distribution_balance(&config);
        assert_eq!(pm["m1"], 1);
        assert_eq!(pm["m2"], 3);
        assert!(balanced);
    }

    #[test]
    fn test_distribution_balance_multiple_machine_target() {
        let mut a = make_resource("package");
        a.machine = types::MachineTarget::Multiple(vec!["web1".into(), "web2".into()]);
        let (pm, balanced) = compute_distribution_balance(&make_config(vec![("a", a)]));
        assert_eq!(pm["web1"], 1);
        assert_eq!(pm["web2"], 1);
        assert!(balanced);
    }
}
