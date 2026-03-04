//! Phase 104 — Operational Maturity & Dependency Governance: validate commands (FJ-1094, FJ-1097, FJ-1100).

#![allow(dead_code)]

use crate::core::types;
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// FJ-1094: Resource dependency version drift
// ============================================================================

/// A warning about a package resource with no version constraint.
struct VersionDriftWarning {
    resource: String,
    packages: Vec<String>,
}

/// Find package resources that lack a version constraint.
fn find_version_drift_warnings(config: &types::ForjarConfig) -> Vec<VersionDriftWarning> {
    let mut warnings = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        if resource.resource_type != types::ResourceType::Package {
            continue;
        }
        if resource.version.is_none() && !resource.packages.is_empty() {
            warnings.push(VersionDriftWarning {
                resource: name.clone(),
                packages: resource.packages.clone(),
            });
        }
    }
    warnings
}

/// FJ-1094: Check if package resources have version constraints. Warn if no version pinning found.
///
/// Parses the config and inspects each resource of type `package`. Resources that
/// declare packages but have no `version` field are flagged, since unpinned packages
/// are vulnerable to silent version drift across environments.
pub(crate) fn cmd_validate_check_resource_dependency_version_drift(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let warnings = find_version_drift_warnings(&cfg);

    if json {
        let items: Vec<serde_json::Value> = warnings
            .iter()
            .map(|w| {
                serde_json::json!({
                    "resource": w.resource,
                    "packages": w.packages
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({ "version_drift_warnings": items, "count": warnings.len() })
        );
    } else if warnings.is_empty() {
        println!("No version drift warnings found.");
    } else {
        println!("Version drift warnings ({}):", warnings.len());
        for w in &warnings {
            println!(
                "  warning: resource '{}' has no version constraint (packages: {})",
                w.resource,
                w.packages.join(", ")
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1097: Resource naming length limit
// ============================================================================

/// Maximum resource name length before a warning is issued.
const MAX_NAME_LENGTH: usize = 64;

/// A warning about a resource whose name exceeds the length limit.
struct NamingLengthWarning {
    resource: String,
    length: usize,
    limit: usize,
}

/// Find resources whose names exceed the length limit.
fn find_naming_length_warnings(
    config: &types::ForjarConfig,
    limit: usize,
) -> Vec<NamingLengthWarning> {
    let mut warnings = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        if name.len() > limit {
            warnings.push(NamingLengthWarning {
                resource: name.clone(),
                length: name.len(),
                limit,
            });
        }
    }
    warnings
}

/// FJ-1097: Warn if resource names exceed 64 characters.
///
/// Parses the config and checks the length of each resource name. Excessively
/// long names harm readability and may cause issues with log output, state files,
/// or external tooling that truncates identifiers.
pub(crate) fn cmd_validate_check_resource_naming_length_limit(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let warnings = find_naming_length_warnings(&cfg, MAX_NAME_LENGTH);

    if json {
        let items: Vec<serde_json::Value> = warnings
            .iter()
            .map(|w| {
                serde_json::json!({
                    "resource": w.resource,
                    "length": w.length,
                    "limit": w.limit
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({ "naming_length_warnings": items, "count": warnings.len() })
        );
    } else if warnings.is_empty() {
        println!("All resource names are within the length limit.");
    } else {
        println!("Naming length warnings ({}):", warnings.len());
        for w in &warnings {
            println!(
                "  warning: resource '{}' has {} characters (limit: {})",
                w.resource, w.length, w.limit
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1100: Resource type coverage per machine
// ============================================================================

/// Expected resource types that every machine should ideally cover.
const EXPECTED_TYPES: &[&str] = &["file", "package", "service"];

/// Coverage report for a single machine.
struct TypeCoverageEntry {
    machine: String,
    present_types: Vec<String>,
    missing_types: Vec<String>,
}

/// Extract machine names from a resource.
fn machine_names(resource: &types::Resource) -> Vec<String> {
    resource.machine.to_vec()
}

/// Group resources by machine and check which expected types are present.
fn compute_type_coverage(config: &types::ForjarConfig) -> Vec<TypeCoverageEntry> {
    let mut machine_types: HashMap<String, Vec<String>> = HashMap::new();
    for resource in config.resources.values() {
        let rtype = resource.resource_type.to_string();
        for m in machine_names(resource) {
            machine_types.entry(m).or_default().push(rtype.clone());
        }
    }
    let mut machines: Vec<String> = machine_types.keys().cloned().collect();
    machines.sort();
    let mut entries = Vec::new();
    for machine in machines {
        let types_seen = &machine_types[&machine];
        let present: Vec<String> = EXPECTED_TYPES
            .iter()
            .filter(|t| types_seen.iter().any(|s| s == **t))
            .map(|t| (*t).to_string())
            .collect();
        let missing: Vec<String> = EXPECTED_TYPES
            .iter()
            .filter(|t| !types_seen.iter().any(|s| s == **t))
            .map(|t| (*t).to_string())
            .collect();
        entries.push(TypeCoverageEntry {
            machine,
            present_types: present,
            missing_types: missing,
        });
    }
    entries
}

/// FJ-1100: Warn if machines lack coverage of expected resource types (file, package, service).
///
/// Parses the config and groups resources by their target machine. For each
/// machine, checks whether the expected resource types (`file`, `package`,
/// `service`) are represented. Missing types may indicate incomplete
/// configuration or oversight in machine provisioning.
pub(crate) fn cmd_validate_check_resource_type_coverage_per_machine(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let entries = compute_type_coverage(&cfg);

    if json {
        let items: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "machine": e.machine,
                    "present_types": e.present_types,
                    "missing_types": e.missing_types
                })
            })
            .collect();
        println!("{}", serde_json::json!({ "type_coverage": items }));
    } else if entries.is_empty() {
        println!("No machines found for type coverage analysis.");
    } else {
        println!(
            "Resource type coverage per machine ({} machines):",
            entries.len()
        );
        for e in &entries {
            if e.missing_types.is_empty() {
                println!(
                    "  {}: full coverage ({})",
                    e.machine,
                    e.present_types.join(", ")
                );
            } else {
                println!(
                    "  warning: {} missing types: {} (has: {})",
                    e.machine,
                    e.missing_types.join(", "),
                    e.present_types.join(", ")
                );
            }
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

    // -- FJ-1094: Version drift tests --

    #[test]
    fn test_version_drift_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_version_drift_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_version_drift_package_with_version() {
        let mut r = make_resource("package");
        r.packages = vec!["nginx".to_string()];
        r.version = Some("1.24".to_string());
        let config = make_config(vec![("web-pkg", r)]);
        let warnings = find_version_drift_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_version_drift_package_without_version() {
        let mut r = make_resource("package");
        r.packages = vec!["nginx".to_string(), "curl".to_string()];
        let config = make_config(vec![("web-pkg", r)]);
        let warnings = find_version_drift_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].resource, "web-pkg");
        assert_eq!(warnings[0].packages, vec!["nginx", "curl"]);
    }

    #[test]
    fn test_version_drift_non_package_ignored() {
        let r = make_resource("file");
        let config = make_config(vec![("cfg-file", r)]);
        let warnings = find_version_drift_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_version_drift_package_no_packages_list() {
        let r = make_resource("package");
        // No packages listed — nothing to pin, no warning
        let config = make_config(vec![("empty-pkg", r)]);
        let warnings = find_version_drift_warnings(&config);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_version_drift_mixed_resources() {
        let mut p1 = make_resource("package");
        p1.packages = vec!["redis".to_string()];
        p1.version = Some("7.0".to_string());

        let mut p2 = make_resource("package");
        p2.packages = vec!["postgres".to_string()];
        let s = make_resource("service");
        let config = make_config(vec![("pinned", p1), ("unpinned", p2), ("svc", s)]);
        let warnings = find_version_drift_warnings(&config);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].resource, "unpinned");
    }

    // -- FJ-1097: Naming length tests --

    #[test]
    fn test_naming_length_empty_config() {
        let config = make_config(vec![]);
        let warnings = find_naming_length_warnings(&config, 64);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_naming_length_under_limit() {
        let r = make_resource("file");
        let config = make_config(vec![("short-name", r)]);
        let warnings = find_naming_length_warnings(&config, 64);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_naming_length_exact_limit() {
        let r = make_resource("file");
        let name = "a".repeat(64);
        let mut map = indexmap::IndexMap::new();
        map.insert(name.clone(), r);
        let yaml = "version: '1.0'\nname: test\nresources: {}";
        let mut cfg: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        cfg.resources = map;
        let warnings = find_naming_length_warnings(&cfg, 64);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_naming_length_over_limit() {
        let r = make_resource("service");
        let name = "a".repeat(65);
        let mut map = indexmap::IndexMap::new();
        map.insert(name.clone(), r);
        let yaml = "version: '1.0'\nname: test\nresources: {}";
        let mut cfg: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        cfg.resources = map;
        let warnings = find_naming_length_warnings(&cfg, 64);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].length, 65);
        assert_eq!(warnings[0].limit, 64);
    }

    #[test]
    fn test_naming_length_multiple_warnings() {
        let r1 = make_resource("file");
        let r2 = make_resource("package");
        let r3 = make_resource("service");
        let long1 = "b".repeat(70);
        let long2 = "c".repeat(80);
        let mut map = indexmap::IndexMap::new();
        map.insert(long1, r1);
        map.insert("ok".to_string(), r2);
        map.insert(long2, r3);
        let yaml = "version: '1.0'\nname: test\nresources: {}";
        let mut cfg: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        cfg.resources = map;
        let warnings = find_naming_length_warnings(&cfg, 64);
        assert_eq!(warnings.len(), 2);
    }

    // -- FJ-1100: Type coverage tests --

    #[test]
    fn test_type_coverage_empty_config() {
        let config = make_config(vec![]);
        let entries = compute_type_coverage(&config);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_type_coverage_full_coverage() {
        let mut f = make_resource("file");
        f.machine = types::MachineTarget::Single("web".to_string());
        let mut p = make_resource("package");
        p.machine = types::MachineTarget::Single("web".to_string());
        let mut s = make_resource("service");
        s.machine = types::MachineTarget::Single("web".to_string());
        let config = make_config(vec![("cfg", f), ("pkg", p), ("svc", s)]);
        let entries = compute_type_coverage(&config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].machine, "web");
        assert!(entries[0].missing_types.is_empty());
        assert_eq!(entries[0].present_types.len(), 3);
    }

    #[test]
    fn test_type_coverage_missing_service() {
        let mut f = make_resource("file");
        f.machine = types::MachineTarget::Single("db".to_string());
        let mut p = make_resource("package");
        p.machine = types::MachineTarget::Single("db".to_string());
        let config = make_config(vec![("cfg", f), ("pkg", p)]);
        let entries = compute_type_coverage(&config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].machine, "db");
        assert_eq!(entries[0].missing_types, vec!["service"]);
    }

    #[test]
    fn test_type_coverage_multiple_machines() {
        let mut f = make_resource("file");
        f.machine = types::MachineTarget::Single("web".to_string());
        let mut p = make_resource("package");
        p.machine = types::MachineTarget::Single("db".to_string());
        let config = make_config(vec![("cfg", f), ("pkg", p)]);
        let entries = compute_type_coverage(&config);
        assert_eq!(entries.len(), 2);
        // db: has package, missing file + service
        let db = entries.iter().find(|e| e.machine == "db").unwrap();
        assert_eq!(db.present_types, vec!["package"]);
        assert_eq!(db.missing_types, vec!["file", "service"]);
        // web: has file, missing package + service
        let web = entries.iter().find(|e| e.machine == "web").unwrap();
        assert_eq!(web.present_types, vec!["file"]);
        assert_eq!(web.missing_types, vec!["package", "service"]);
    }

    #[test]
    fn test_type_coverage_multi_machine_target() {
        let mut f = make_resource("file");
        f.machine = types::MachineTarget::Multiple(vec!["a".to_string(), "b".to_string()]);
        let config = make_config(vec![("cfg", f)]);
        let entries = compute_type_coverage(&config);
        assert_eq!(entries.len(), 2);
        let a = entries.iter().find(|e| e.machine == "a").unwrap();
        assert_eq!(a.present_types, vec!["file"]);
        let b = entries.iter().find(|e| e.machine == "b").unwrap();
        assert_eq!(b.present_types, vec!["file"]);
    }

    #[test]
    fn test_type_coverage_non_expected_type_ignored() {
        let mut d = make_resource("docker");
        d.machine = types::MachineTarget::Single("ci".to_string());
        let config = make_config(vec![("container", d)]);
        let entries = compute_type_coverage(&config);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].machine, "ci");
        assert!(entries[0].present_types.is_empty());
        assert_eq!(entries[0].missing_types, vec!["file", "package", "service"]);
    }
}
