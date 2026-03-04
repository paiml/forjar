//! Phase 105 — Resource Hygiene & Structural Depth: validate commands (FJ-1102, FJ-1105, FJ-1108).

#![allow(dead_code)]

use crate::core::types;
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// FJ-1102: Resource dependency depth variance
// ============================================================================

/// Depth statistics for dependency chains.
struct DepthStats {
    min: usize,
    max: usize,
    variance: f64,
    resource_count: usize,
}

/// Compute dependency chain depth for each resource (Bellman-Ford propagation).
fn compute_depths(config: &types::ForjarConfig) -> HashMap<String, usize> {
    let mut depths: HashMap<String, usize> = HashMap::new();
    let names: Vec<String> = config.resources.keys().cloned().collect();

    // Initialize all resources at depth 0
    for name in &names {
        depths.insert(name.clone(), 0);
    }

    // Iteratively propagate depths until stable (Bellman-Ford style)
    let mut changed = true;
    while changed {
        changed = false;
        for name in &names {
            let resource = &config.resources[name];
            for dep_name in &resource.depends_on {
                if let Some(&dep_depth) = depths.get(dep_name) {
                    let new_depth = dep_depth + 1;
                    let current = depths[name];
                    if new_depth > current {
                        depths.insert(name.clone(), new_depth);
                        changed = true;
                    }
                }
            }
        }
    }

    depths
}

/// Compute depth statistics across all resources.
fn compute_depth_stats(config: &types::ForjarConfig) -> Option<DepthStats> {
    if config.resources.is_empty() {
        return None;
    }
    let depths = compute_depths(config);
    let values: Vec<usize> = depths.values().copied().collect();
    let min = values.iter().copied().min().unwrap_or(0);
    let max = values.iter().copied().max().unwrap_or(0);
    let n = values.len() as f64;
    let mean = values.iter().copied().sum::<usize>() as f64 / n;
    let variance = values
        .iter()
        .map(|&v| {
            let diff = v as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / n;

    Some(DepthStats {
        min,
        max,
        variance,
        resource_count: values.len(),
    })
}

/// Depth variance threshold: warn if `max_depth - min_depth > DEPTH_VARIANCE_THRESHOLD`.
const DEPTH_VARIANCE_THRESHOLD: usize = 3;

/// FJ-1102: Check dependency chain depth variance across resources. Warns when
/// depth spread exceeds the threshold, indicating uneven dependency topology.
pub(crate) fn cmd_validate_check_resource_dependency_depth_variance(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let stats = compute_depth_stats(&cfg);

    if json {
        let (min, max, variance, warnings) = match &stats {
            Some(s) => {
                let w = if s.max - s.min > DEPTH_VARIANCE_THRESHOLD {
                    1
                } else {
                    0
                };
                (s.min, s.max, s.variance, w)
            }
            None => (0, 0, 0.0, 0),
        };
        println!(
            "{}",
            serde_json::json!({
                "dependency_depth_variance": {
                    "min": min,
                    "max": max,
                    "variance": variance,
                    "warnings": warnings
                }
            })
        );
    } else {
        match stats {
            Some(s) if s.max - s.min > DEPTH_VARIANCE_THRESHOLD => {
                println!(
                    "Dependency depth variance: {} (min={}, max={}, resources={})",
                    s.variance, s.min, s.max, s.resource_count
                );
            }
            Some(_) | None => {
                println!("Dependency depth variance: OK (uniform depths)");
            }
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1105: Resource tag key naming conventions
// ============================================================================

/// A violation of tag key naming conventions.
struct TagKeyViolation {
    resource: String,
    key: String,
    reason: String,
}

/// Check whether a tag key follows naming conventions (lowercase, no spaces).
fn validate_tag_key(key: &str) -> Option<String> {
    if key.is_empty() {
        return Some("empty key".to_string());
    }
    if key != key.to_lowercase() {
        return Some("key contains uppercase characters".to_string());
    }
    if key.contains(' ') {
        return Some("key contains spaces".to_string());
    }
    let valid = key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':');
    if !valid {
        return Some("key contains invalid characters".to_string());
    }
    None
}

/// Find all tag key naming violations across resources.
fn find_tag_key_violations(config: &types::ForjarConfig) -> Vec<TagKeyViolation> {
    let mut violations = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        for tag in &resource.tags {
            // Tags may be "key:value" format — extract the key part
            let key = tag.split(':').next().unwrap_or(tag);
            if let Some(reason) = validate_tag_key(key) {
                violations.push(TagKeyViolation {
                    resource: name.clone(),
                    key: key.to_string(),
                    reason,
                });
            }
        }
    }
    violations
}

/// FJ-1105: Check that all tag keys follow naming conventions. Tags may be
/// `key:value` pairs; only the key portion is validated.
pub(crate) fn cmd_validate_check_resource_tag_key_naming(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let violations = find_tag_key_violations(&cfg);

    if json {
        let items: Vec<serde_json::Value> = violations
            .iter()
            .map(|v| {
                serde_json::json!({
                    "resource": v.resource,
                    "key": v.key,
                    "reason": v.reason
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "tag_key_naming": {
                    "warnings": violations.len(),
                    "violations": items
                }
            })
        );
    } else if violations.is_empty() {
        println!("Tag key naming: 0 warnings");
    } else {
        println!("Tag key naming: {} warnings", violations.len());
        for v in &violations {
            println!(
                "  warning: resource '{}' tag key '{}': {}",
                v.resource, v.key, v.reason
            );
        }
    }
    Ok(())
}

// ============================================================================
// FJ-1108: Resource content length limit
// ============================================================================

/// Maximum inline content length before a warning is issued.
const CONTENT_LENGTH_LIMIT: usize = 4096;

/// A violation for a resource whose content exceeds the length limit.
struct ContentLengthViolation {
    resource: String,
    length: usize,
}

/// Find resources whose inline content exceeds the length limit.
fn find_content_length_violations(
    config: &types::ForjarConfig,
    limit: usize,
) -> Vec<ContentLengthViolation> {
    let mut violations = Vec::new();
    let mut names: Vec<&String> = config.resources.keys().collect();
    names.sort();
    for name in names {
        let resource = &config.resources[name];
        if let Some(ref content) = resource.content {
            if content.len() > limit {
                violations.push(ContentLengthViolation {
                    resource: name.clone(),
                    length: content.len(),
                });
            }
        }
    }
    violations
}

/// FJ-1108: Warn if inline resource content exceeds the length limit. Content
/// exceeding 4096 characters should be moved to an external file.
pub(crate) fn cmd_validate_check_resource_content_length_limit(
    file: &Path,
    json: bool,
) -> Result<(), String> {
    let txt = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let cfg: crate::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&txt).map_err(|e| e.to_string())?;

    let violations = find_content_length_violations(&cfg, CONTENT_LENGTH_LIMIT);

    if json {
        let items: Vec<serde_json::Value> = violations
            .iter()
            .map(|v| {
                serde_json::json!({
                    "resource": v.resource,
                    "length": v.length
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "content_length": {
                    "limit": CONTENT_LENGTH_LIMIT,
                    "violations": items
                }
            })
        );
    } else if violations.is_empty() {
        println!(
            "Content length: 0 resources exceed limit ({CONTENT_LENGTH_LIMIT} chars)"
        );
    } else {
        println!(
            "Content length: {} resources exceed limit ({} chars)",
            violations.len(),
            CONTENT_LENGTH_LIMIT
        );
        for v in &violations {
            println!(
                "  warning: resource '{}' has {} characters",
                v.resource, v.length
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

    // -- FJ-1102: Dependency depth variance tests --

    #[test]
    fn test_depth_variance_empty_config() {
        let f = write_temp_config("version: '1.0'\nname: test\nresources: {}\n");
        let result = cmd_validate_check_resource_dependency_depth_variance(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_depth_variance_with_deep_chain() {
        let yaml = "\
version: '1.0'
name: test
resources:
  a:
    type: file
  b:
    type: file
    depends_on: [a]
  c:
    type: file
    depends_on: [b]
  d:
    type: file
    depends_on: [c]
  e:
    type: file
    depends_on: [d]
  root:
    type: file
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_dependency_depth_variance(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_depth_variance_json_output() {
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
        let result = cmd_validate_check_resource_dependency_depth_variance(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_depth_variance_file_not_found() {
        let result = cmd_validate_check_resource_dependency_depth_variance(
            Path::new("/nonexistent/forjar.yaml"),
            false,
        );
        assert!(result.is_err());
    }

    // -- FJ-1105: Tag key naming tests --

    #[test]
    fn test_tag_key_naming_empty_config() {
        let f = write_temp_config("version: '1.0'\nname: test\nresources: {}\n");
        let result = cmd_validate_check_resource_tag_key_naming(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tag_key_naming_with_violations() {
        let yaml = "\
version: '1.0'
name: test
resources:
  web:
    type: file
    tags:
      - 'env:prod'
      - 'Bad Key'
      - 'UPPER'
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_tag_key_naming(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tag_key_naming_json_output() {
        let yaml = "\
version: '1.0'
name: test
resources:
  web:
    type: file
    tags:
      - 'valid-key:value'
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_tag_key_naming(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tag_key_naming_file_not_found() {
        let result = cmd_validate_check_resource_tag_key_naming(
            Path::new("/nonexistent/forjar.yaml"),
            false,
        );
        assert!(result.is_err());
    }

    // -- FJ-1108: Content length limit tests --

    #[test]
    fn test_content_length_empty_config() {
        let f = write_temp_config("version: '1.0'\nname: test\nresources: {}\n");
        let result = cmd_validate_check_resource_content_length_limit(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_content_length_with_violation() {
        let big_content = "x".repeat(5000);
        let yaml = format!(
            "version: '1.0'\nname: test\nresources:\n  big-file:\n    type: file\n    content: '{big_content}'\n"
        );
        let f = write_temp_config(&yaml);
        let result = cmd_validate_check_resource_content_length_limit(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_content_length_json_output() {
        let yaml = "\
version: '1.0'
name: test
resources:
  small:
    type: file
    content: 'hello world'
";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_resource_content_length_limit(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_content_length_file_not_found() {
        let result = cmd_validate_check_resource_content_length_limit(
            Path::new("/nonexistent/forjar.yaml"),
            false,
        );
        assert!(result.is_err());
    }
}
