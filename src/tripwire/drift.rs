//! FJ-016: Drift detection — compare live state to lock hashes.

use crate::core::types::{ResourceStatus, ResourceType, StateLock};
use crate::tripwire::hasher;
use std::path::Path;

/// A single drift finding.
#[derive(Debug, Clone)]
pub struct DriftFinding {
    pub resource_id: String,
    pub resource_type: ResourceType,
    pub expected_hash: String,
    pub actual_hash: String,
    pub detail: String,
}

/// Check a single file resource for drift.
pub fn check_file_drift(
    resource_id: &str,
    path: &str,
    expected_hash: &str,
) -> Option<DriftFinding> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Some(DriftFinding {
            resource_id: resource_id.to_string(),
            resource_type: ResourceType::File,
            expected_hash: expected_hash.to_string(),
            actual_hash: "MISSING".to_string(),
            detail: format!("{} does not exist", path),
        });
    }

    let actual = if file_path.is_dir() {
        hasher::hash_directory(file_path).unwrap_or_else(|e| format!("ERROR:{}", e))
    } else {
        hasher::hash_file(file_path).unwrap_or_else(|e| format!("ERROR:{}", e))
    };

    if actual != expected_hash {
        Some(DriftFinding {
            resource_id: resource_id.to_string(),
            resource_type: ResourceType::File,
            expected_hash: expected_hash.to_string(),
            actual_hash: actual,
            detail: format!("{} content changed", path),
        })
    } else {
        None
    }
}

/// Check all file-type resources in a lock for drift.
pub fn detect_drift(lock: &StateLock) -> Vec<DriftFinding> {
    let mut findings = Vec::new();

    for (id, rl) in &lock.resources {
        if rl.status != ResourceStatus::Converged {
            continue;
        }

        // Only file-type drift is detectable locally (package/service require remote query — Phase 2)
        if rl.resource_type == ResourceType::File {
            if let Some(path_val) = rl.details.get("path") {
                let path = match path_val {
                    serde_yaml_ng::Value::String(s) => s.as_str(),
                    _non_string => continue,
                };
                if let Some(content_hash) = rl.details.get("content_hash") {
                    let expected = match content_hash {
                        serde_yaml_ng::Value::String(s) => s.as_str(),
                        _non_string => continue,
                    };
                    if let Some(finding) = check_file_drift(id, path, expected) {
                        findings.push(finding);
                    }
                }
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj016_no_drift() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello").unwrap();
        let hash = hasher::hash_file(&file).unwrap();

        let result = check_file_drift("test-file", file.to_str().unwrap(), &hash);
        assert!(result.is_none());
    }

    #[test]
    fn test_fj016_content_drift() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "hello").unwrap();
        let hash = hasher::hash_file(&file).unwrap();

        // Modify file outside forjar
        std::fs::write(&file, "modified").unwrap();

        let result = check_file_drift("test-file", file.to_str().unwrap(), &hash);
        assert!(result.is_some());
        let finding = result.unwrap();
        assert_eq!(finding.resource_id, "test-file");
        assert_ne!(finding.actual_hash, finding.expected_hash);
    }

    #[test]
    fn test_fj016_missing_file() {
        let result = check_file_drift("gone", "/nonexistent/file.txt", "blake3:abc");
        assert!(result.is_some());
        assert_eq!(result.unwrap().actual_hash, "MISSING");
    }

    #[test]
    fn test_fj016_detect_drift_empty_lock() {
        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test-box".to_string(),
            generated_at: "2026-02-16T14:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources: indexmap::IndexMap::new(),
        };
        let findings = detect_drift(&lock);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_fj016_detect_drift_converged_file_with_drift() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.txt");
        std::fs::write(&file, "original").unwrap();
        let hash = hasher::hash_string("original-content"); // mismatched hash

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(hash),
        );
        resources.insert(
            "config".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: "blake3:xyz".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test-box".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].resource_id, "config");
        assert!(findings[0].detail.contains("content changed"));
    }

    #[test]
    fn test_fj016_detect_drift_no_drift_when_matching() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("stable.txt");
        std::fs::write(&file, "stable content").unwrap();
        let content_hash = hasher::hash_file(&file).unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(content_hash),
        );
        resources.insert(
            "stable".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:x".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test-box".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(
            findings.is_empty(),
            "no drift expected when file hash matches"
        );
    }

    #[test]
    fn test_fj016_detect_drift_skips_non_converged() {
        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String("/nonexistent".to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:abc".to_string()),
        );
        resources.insert(
            "failed-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Failed,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:abc".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test-box".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(
            findings.is_empty(),
            "non-converged resources should be skipped"
        );
    }

    #[test]
    fn test_fj016_detect_drift_skips_non_file_types() {
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "my-pkg".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::Package,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:abc".to_string(),
                details: std::collections::HashMap::new(),
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test-box".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(findings.is_empty(), "package resources should be skipped");
    }

    #[test]
    fn test_fj016_detect_drift_missing_path_detail() {
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "no-path".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:abc".to_string(),
                details: std::collections::HashMap::new(), // no "path" key
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test-box".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(findings.is_empty(), "missing path detail should be skipped");
    }

    #[test]
    fn test_fj016_detect_drift_non_string_path_skipped() {
        // Exercises the `_ => continue` branch when path is not a String
        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42)),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:abc".to_string()),
        );
        resources.insert(
            "bad-path".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:abc".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test-box".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(
            findings.is_empty(),
            "non-string path value should be skipped"
        );
    }

    #[test]
    fn test_fj016_detect_drift_non_string_content_hash_skipped() {
        // Exercises the `_ => continue` branch when content_hash is not a String
        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String("/tmp/test.txt".to_string()),
        );
        details.insert("content_hash".to_string(), serde_yaml_ng::Value::Bool(true));
        resources.insert(
            "bad-hash".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:abc".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test-box".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(
            findings.is_empty(),
            "non-string content_hash should be skipped"
        );
    }

    #[test]
    fn test_fj016_check_file_drift_directory() {
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("mydir");
        std::fs::create_dir(&subdir).unwrap();
        let hash = hasher::hash_directory(&subdir).unwrap();

        // No drift when hash matches
        let result = check_file_drift("dir-resource", subdir.to_str().unwrap(), &hash);
        assert!(result.is_none());

        // Create a file inside — hash changes
        std::fs::write(subdir.join("new.txt"), "surprise").unwrap();
        let result = check_file_drift("dir-resource", subdir.to_str().unwrap(), &hash);
        assert!(result.is_some());
    }
}
