//! Phase 107 — Resource Scoring & Quality Checks (FJ-1118, FJ-1121, FJ-1124).

#![allow(dead_code)]

use crate::core::types;
use std::path::Path;

// ============================================================================
// Helpers
// ============================================================================

/// Parse a forjar config from a file path.
fn load_config(file: &Path) -> Result<types::ForjarConfig, String> {
    let raw = std::fs::read_to_string(file).map_err(|e| format!("read: {e}"))?;
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&raw).map_err(|e| format!("parse: {e}"))?;
    Ok(config)
}

// ============================================================================
// FJ-1118: Resource dependency ordering consistency
// ============================================================================

/// A dependency ordering inconsistency.
struct DepInconsistency {
    resource: String,
    missing_dep: String,
}
fn find_dep_inconsistencies(config: &types::ForjarConfig) -> Vec<DepInconsistency> {
    let mut results = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                results.push(DepInconsistency {
                    resource: name.clone(),
                    missing_dep: dep.clone(),
                });
            }
        }
    }
    results
}
/// FJ-1118: Check dependency ordering consistency across resources.
pub(crate) fn cmd_validate_check_resource_dependency_ordering_consistency(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let config = load_config(file)?;
    let total = config.resources.len();
    let issues = find_dep_inconsistencies(&config);
    if json {
        let details: Vec<serde_json::Value> = issues
            .iter()
            .map(|i| {
                serde_json::json!({
                    "resource": i.resource,
                    "missing_dep": i.missing_dep
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "check_resource_dependency_ordering_consistency": {
                    "total": total,
                    "issues": issues.len(),
                    "details": details
                }
            })
        );
    } else {
        println!(
            "Dependency ordering: {} resources checked, {} inconsistencies",
            total,
            issues.len()
        );
        for i in &issues {
            println!(
                "  inconsistency: '{}' depends on '{}' which does not exist",
                i.resource, i.missing_dep
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1121: Resource tag value format validation
// ============================================================================

/// A tag value format warning.
struct TagWarning {
    resource: String,
    tag: String,
    reason: String,
}
fn check_tag_format(tag: &str) -> Option<&'static str> {
    if tag.is_empty() {
        return Some("empty tag value");
    }
    if tag.contains(' ') {
        return Some("contains spaces");
    }
    let has_special = tag
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.' && c != ':');
    if has_special {
        return Some("contains special characters");
    }
    None
}
fn find_tag_warnings(config: &types::ForjarConfig) -> (usize, Vec<TagWarning>) {
    let mut warnings = Vec::new();
    let mut tagged_count: usize = 0;
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        if resource.tags.is_empty() {
            continue;
        }
        tagged_count += 1;
        for tag in &resource.tags {
            if let Some(reason) = check_tag_format(tag) {
                warnings.push(TagWarning {
                    resource: name.clone(),
                    tag: tag.clone(),
                    reason: reason.to_string(),
                });
            }
        }
    }
    (tagged_count, warnings)
}
/// FJ-1121: Validate tag value formats across all resources.
pub(crate) fn cmd_validate_check_resource_tag_value_format(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let config = load_config(file)?;
    let (tagged_count, warnings) = find_tag_warnings(&config);
    if json {
        let details: Vec<serde_json::Value> = warnings
            .iter()
            .map(|w| {
                serde_json::json!({
                    "resource": w.resource,
                    "tag": w.tag,
                    "reason": w.reason
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "check_resource_tag_value_format": {
                    "total": tagged_count,
                    "warnings": warnings.len(),
                    "details": details
                }
            })
        );
    } else {
        println!(
            "Tag value format: {} warnings across {} tagged resources",
            warnings.len(),
            tagged_count
        );
        for w in &warnings {
            println!(
                "  warning: resource '{}' tag '{}' — {}",
                w.resource, w.tag, w.reason
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1124: Resource provider version pinning
// ============================================================================

fn count_version_pinning(config: &types::ForjarConfig) -> (usize, usize, usize) {
    let total = config.resources.len();
    let mut pinned: usize = 0;
    for resource in config.resources.values() {
        if resource.version.is_some() {
            pinned += 1;
        }
    }
    let unpinned = total.saturating_sub(pinned);
    (total, pinned, unpinned)
}
/// FJ-1124: Check how many resources have pinned provider versions.
pub(crate) fn cmd_validate_check_resource_provider_version_pinning(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let config = load_config(file)?;
    let (total, pinned, unpinned) = count_version_pinning(&config);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "check_resource_provider_version_pinning": {
                    "total": total,
                    "pinned": pinned,
                    "unpinned": unpinned
                }
            })
        );
    } else {
        println!(
            "Provider version pinning: {}/{} resources have pinned versions",
            pinned, total
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
    use std::io::Write;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const EMPTY_CFG: &str = "\
version: '1.0'
name: test
resources: {}
";

    // -- FJ-1118: Dependency ordering consistency tests --

    #[test]
    fn dep_ordering_empty_config() {
        let f = write_temp_config(EMPTY_CFG);
        let r = cmd_validate_check_resource_dependency_ordering_consistency(f.path(), false);
        assert!(r.is_ok());
    }
    #[test]
    fn dep_ordering_consistent() {
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
        let r = cmd_validate_check_resource_dependency_ordering_consistency(f.path(), false);
        assert!(r.is_ok());
    }
    #[test]
    fn dep_ordering_inconsistent() {
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
        let r = cmd_validate_check_resource_dependency_ordering_consistency(f.path(), false);
        assert!(r.is_ok());
    }
    #[test]
    fn dep_ordering_json() {
        let yaml = "\
version: '1.0'
name: test
resources:
  app:
    type: file
    depends_on: [missing]
";
        let f = write_temp_config(yaml);
        let r = cmd_validate_check_resource_dependency_ordering_consistency(f.path(), true);
        assert!(r.is_ok());
    }
    #[test]
    fn dep_ordering_file_not_found() {
        let r = cmd_validate_check_resource_dependency_ordering_consistency(
            Path::new("/nonexistent/forjar.yaml"),
            false,
        );
        assert!(r.is_err());
    }

    // -- FJ-1121: Tag value format tests --

    #[test]
    fn tag_format_empty_config() {
        let f = write_temp_config(EMPTY_CFG);
        let r = cmd_validate_check_resource_tag_value_format(f.path(), false);
        assert!(r.is_ok());
    }
    #[test]
    fn tag_format_valid_tags() {
        let yaml = "\
version: '1.0'
name: test
resources:
  web:
    type: file
    tags: [production, tier-1, v2.0]
";
        let f = write_temp_config(yaml);
        let r = cmd_validate_check_resource_tag_value_format(f.path(), false);
        assert!(r.is_ok());
    }
    #[test]
    fn tag_format_invalid_tags() {
        let yaml = "\
version: '1.0'
name: test
resources:
  svc:
    type: service
    tags: ['has space', 'ok-tag', 'bad!char']
";
        let f = write_temp_config(yaml);
        let r = cmd_validate_check_resource_tag_value_format(f.path(), false);
        assert!(r.is_ok());
    }
    #[test]
    fn tag_format_json() {
        let yaml = "\
version: '1.0'
name: test
resources:
  app:
    type: file
    tags: ['good', 'bad tag']
";
        let f = write_temp_config(yaml);
        let r = cmd_validate_check_resource_tag_value_format(f.path(), true);
        assert!(r.is_ok());
    }
    #[test]
    fn tag_format_file_not_found() {
        let r = cmd_validate_check_resource_tag_value_format(
            Path::new("/nonexistent/forjar.yaml"),
            false,
        );
        assert!(r.is_err());
    }

    // -- FJ-1124: Provider version pinning tests --

    #[test]
    fn version_pinning_empty_config() {
        let f = write_temp_config(EMPTY_CFG);
        let r = cmd_validate_check_resource_provider_version_pinning(f.path(), false);
        assert!(r.is_ok());
    }
    #[test]
    fn version_pinning_all_pinned() {
        let yaml = "\
version: '1.0'
name: test
resources:
  nginx:
    type: package
    version: '1.24'
  redis:
    type: package
    version: '7.2'
";
        let f = write_temp_config(yaml);
        let r = cmd_validate_check_resource_provider_version_pinning(f.path(), false);
        assert!(r.is_ok());
    }
    #[test]
    fn version_pinning_mixed() {
        let yaml = "\
version: '1.0'
name: test
resources:
  nginx:
    type: package
    version: '1.24'
  curl:
    type: package
  cfg:
    type: file
";
        let f = write_temp_config(yaml);
        let r = cmd_validate_check_resource_provider_version_pinning(f.path(), false);
        assert!(r.is_ok());
    }
    #[test]
    fn version_pinning_json() {
        let yaml = "\
version: '1.0'
name: test
resources:
  app:
    type: package
    version: '2.0'
  lib:
    type: package
";
        let f = write_temp_config(yaml);
        let r = cmd_validate_check_resource_provider_version_pinning(f.path(), true);
        assert!(r.is_ok());
    }
    #[test]
    fn version_pinning_file_not_found() {
        let r = cmd_validate_check_resource_provider_version_pinning(
            Path::new("/nonexistent/forjar.yaml"),
            false,
        );
        assert!(r.is_err());
    }
}
