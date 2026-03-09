//! Phase 106 — Resource Audit & Coverage Analysis: validate commands (FJ-1110, FJ-1113, FJ-1116).

use crate::core::types;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

// ============================================================================
// FJ-1110: Resource dependency completeness audit
// ============================================================================

/// A missing dependency reference.
struct MissingDep {
    resource: String,
    missing_dep: String,
}

/// Find all declared dependencies that do not exist as resource keys.
fn find_missing_deps(config: &types::ForjarConfig) -> Vec<MissingDep> {
    let mut results = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                results.push(MissingDep {
                    resource: name.clone(),
                    missing_dep: dep.clone(),
                });
            }
        }
    }
    results
}

/// FJ-1110: Verify all declared dependencies exist in config.resources.
/// Reports missing dependency references that would cause apply failures.
pub(crate) fn cmd_validate_check_resource_dependency_completeness_audit(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let missing = find_missing_deps(&cfg);

    if json {
        let items: Vec<serde_json::Value> = missing
            .iter()
            .map(|m| {
                serde_json::json!({
                    "resource": m.resource,
                    "missing_dep": m.missing_dep
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "dependency_completeness": {
                    "warnings": missing.len(),
                    "missing": items
                }
            })
        );
    } else {
        println!("Dependency completeness: {} warnings", missing.len());
        for m in &missing {
            println!(
                "  warning: resource '{}' depends on '{}' which does not exist",
                m.resource, m.missing_dep
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1113: Resource machine coverage gap
// ============================================================================

/// A coverage gap: a machine missing resource types that other machines have.
struct CoverageGap {
    machine: String,
    missing_types: Vec<String>,
}

/// Compare resource types present on each machine vs fleet-wide types.
fn find_machine_coverage_gaps(config: &types::ForjarConfig) -> Vec<CoverageGap> {
    // Collect resource types per machine
    let mut machine_types: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut fleet_types: BTreeSet<String> = BTreeSet::new();

    for resource in config.resources.values() {
        let rtype = resource.resource_type.to_string();
        fleet_types.insert(rtype.clone());
        for machine in resource.machine.iter() {
            machine_types
                .entry(machine.to_owned())
                .or_default()
                .insert(rtype.clone());
        }
    }

    // Find gaps: machines missing types present in the fleet
    let mut gaps = Vec::new();
    for (machine, types_set) in &machine_types {
        let missing: Vec<String> = fleet_types.difference(types_set).cloned().collect();
        if !missing.is_empty() {
            gaps.push(CoverageGap {
                machine: machine.clone(),
                missing_types: missing,
            });
        }
    }
    gaps
}

/// FJ-1113: Check for machine coverage gaps — machines missing resource types
/// that are present on other machines in the fleet.
pub(crate) fn cmd_validate_check_resource_machine_coverage_gap(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let gaps = find_machine_coverage_gaps(&cfg);

    if json {
        let items: Vec<serde_json::Value> = gaps
            .iter()
            .map(|g| {
                serde_json::json!({
                    "machine": g.machine,
                    "missing_types": g.missing_types
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "machine_coverage_gaps": {
                    "warnings": gaps.len(),
                    "gaps": items
                }
            })
        );
    } else {
        println!("Machine coverage gaps: {} warnings", gaps.len());
        for g in &gaps {
            println!(
                "  warning: machine '{}' is missing types: [{}]",
                g.machine,
                g.missing_types.join(", ")
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1116: Resource path depth limit
// ============================================================================

/// Maximum allowed directory depth (number of '/' separators).
const PATH_DEPTH_LIMIT: usize = 8;

/// A path depth violation.
struct PathDepthViolation {
    resource: String,
    path: String,
    depth: usize,
}

/// Find file resources whose path exceeds the depth limit.
fn find_path_depth_violations(
    config: &types::ForjarConfig,
    limit: usize,
) -> Vec<PathDepthViolation> {
    let mut violations = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        if let Some(ref path) = resource.path {
            let depth = path.chars().filter(|&c| c == '/').count();
            if depth > limit {
                violations.push(PathDepthViolation {
                    resource: name.clone(),
                    path: path.clone(),
                    depth,
                });
            }
        }
    }
    violations
}

/// FJ-1116: Check file resource paths for directory depth exceeding the limit.
/// Deep paths may indicate overly nested directory structures.
pub(crate) fn cmd_validate_check_resource_path_depth_limit(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let violations = find_path_depth_violations(&cfg, PATH_DEPTH_LIMIT);

    if json {
        let items: Vec<serde_json::Value> = violations
            .iter()
            .map(|v| {
                serde_json::json!({
                    "resource": v.resource,
                    "path": v.path,
                    "depth": v.depth
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "path_depth": {
                    "limit": PATH_DEPTH_LIMIT,
                    "violations": items
                }
            })
        );
    } else if violations.is_empty() {
        println!("Path depth: 0 resources exceed limit ({PATH_DEPTH_LIMIT} levels)");
    } else {
        println!(
            "Path depth: {} resources exceed limit ({} levels)",
            violations.len(),
            PATH_DEPTH_LIMIT
        );
        for v in &violations {
            println!(
                "  warning: resource '{}' path '{}' has depth {}",
                v.resource, v.path, v.depth
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
    use std::io::Write;

    /// Write a YAML config string to a temporary file and return the path.
    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    // -- FJ-1110: Dependency completeness tests --

    #[test]
    fn test_dependency_completeness_no_warnings() {
        let yaml = "\
version: '1.0'
name: test
resources:
  a:
    type: file
  b:
    type: file
    depends_on: [a]
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_dependency_completeness_audit(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dependency_completeness_with_missing() {
        let yaml = "\
version: '1.0'
name: test
resources:
  web:
    type: service
    depends_on: [db, cache]
  db:
    type: package
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_dependency_completeness_audit(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dependency_completeness_json_output() {
        let yaml = "\
version: '1.0'
name: test
resources:
  app:
    type: file
    depends_on: [missing-res]
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_dependency_completeness_audit(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dependency_completeness_file_not_found() {
        let result = cmd_validate_check_resource_dependency_completeness_audit(
            Path::new("/nonexistent/forjar.yaml"),
            false,
        );
        assert!(result.is_err());
    }

    // -- FJ-1113: Machine coverage gap tests --

    #[test]
    fn test_machine_coverage_no_gaps() {
        let yaml = "\
version: '1.0'
name: test
resources:
  web-pkg:
    type: package
    machine: web
  db-pkg:
    type: package
    machine: db
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_machine_coverage_gap(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_machine_coverage_with_gaps() {
        let yaml = "\
version: '1.0'
name: test
resources:
  web-pkg:
    type: package
    machine: web
  web-svc:
    type: service
    machine: web
  db-pkg:
    type: package
    machine: db
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_machine_coverage_gap(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_machine_coverage_json_output() {
        let yaml = "\
version: '1.0'
name: test
resources:
  web-file:
    type: file
    machine: web
  db-svc:
    type: service
    machine: db
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_machine_coverage_gap(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_machine_coverage_file_not_found() {
        let result = cmd_validate_check_resource_machine_coverage_gap(
            Path::new("/nonexistent/forjar.yaml"),
            false,
        );
        assert!(result.is_err());
    }

    // -- FJ-1116: Path depth limit tests --

    #[test]
    fn test_path_depth_no_violations() {
        let yaml = "\
version: '1.0'
name: test
resources:
  cfg:
    type: file
    path: /etc/nginx/nginx.conf
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_path_depth_limit(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_depth_with_violation() {
        let yaml = "\
version: '1.0'
name: test
resources:
  deep-file:
    type: file
    path: /a/b/c/d/e/f/g/h/i/j/k.conf
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_path_depth_limit(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_depth_json_output() {
        let yaml = "\
version: '1.0'
name: test
resources:
  shallow:
    type: file
    path: /etc/app.conf
  deep:
    type: file
    path: /a/b/c/d/e/f/g/h/i/deep.txt
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_path_depth_limit(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_depth_file_not_found() {
        let result = cmd_validate_check_resource_path_depth_limit(
            Path::new("/nonexistent/forjar.yaml"),
            false,
        );
        assert!(result.is_err());
    }
}
