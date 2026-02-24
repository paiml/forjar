//! FJ-016: Drift detection — compare live state to lock hashes.

use crate::core::types::{Machine, Resource, ResourceStatus, ResourceType, StateLock};
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

/// Check a file resource for drift via transport (for container/remote machines).
/// Runs `cat <path>` on the target and hashes the output.
pub fn check_file_drift_via_transport(
    resource_id: &str,
    path: &str,
    expected_hash: &str,
    machine: &Machine,
) -> Option<DriftFinding> {
    let script = format!(
        "set -euo pipefail\nif [ -d '{}' ]; then echo '__DIR__'; else cat '{}'; fi",
        path, path
    );
    match crate::transport::exec_script(machine, &script) {
        Ok(out) if out.success() => {
            let actual = if out.stdout.trim() == "__DIR__" {
                // For directories, hash the listing instead
                let ls_script = format!("ls -la '{}'", path);
                match crate::transport::exec_script(machine, &ls_script) {
                    Ok(ls_out) if ls_out.success() => hasher::hash_string(&ls_out.stdout),
                    _ => return None,
                }
            } else {
                hasher::hash_string(&out.stdout)
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
        Ok(out) => Some(DriftFinding {
            resource_id: resource_id.to_string(),
            resource_type: ResourceType::File,
            expected_hash: expected_hash.to_string(),
            actual_hash: "MISSING".to_string(),
            detail: format!("{} not accessible: {}", path, out.stderr.trim()),
        }),
        Err(e) => Some(DriftFinding {
            resource_id: resource_id.to_string(),
            resource_type: ResourceType::File,
            expected_hash: expected_hash.to_string(),
            actual_hash: "ERROR".to_string(),
            detail: format!("transport error: {}", e),
        }),
    }
}

/// Check all file-type resources in a lock for drift.
/// Uses local filesystem hashing (for local machines without transport context).
pub fn detect_drift(lock: &StateLock) -> Vec<DriftFinding> {
    detect_drift_impl(lock, None)
}

/// Check all file-type resources in a lock for drift, using transport for remote/container machines.
pub fn detect_drift_with_machine(lock: &StateLock, machine: &Machine) -> Vec<DriftFinding> {
    detect_drift_impl(lock, Some(machine))
}

/// Full drift detection: files via hash comparison, non-file resources via state_query_script.
/// Requires the config resources to reconstruct state query scripts.
pub fn detect_drift_full(
    lock: &StateLock,
    machine: &Machine,
    resources: &indexmap::IndexMap<String, Resource>,
) -> Vec<DriftFinding> {
    let mut findings = detect_drift_impl(lock, Some(machine));

    // Check non-file resources by re-running state_query_script and comparing live_hash
    for (id, rl) in &lock.resources {
        if rl.status != ResourceStatus::Converged {
            continue;
        }
        if rl.resource_type == ResourceType::File {
            continue; // already handled by detect_drift_impl
        }

        let stored_live_hash = match rl.details.get("live_hash") {
            Some(serde_yaml_ng::Value::String(s)) => s.as_str(),
            _ => continue, // no live_hash to compare against
        };

        // Look up the resource config to build a state query script
        let resource = match resources.get(id) {
            Some(r) => r,
            None => continue,
        };

        let query = match crate::core::codegen::state_query_script(resource) {
            Ok(q) => q,
            Err(_) => continue,
        };

        let actual_hash = match crate::transport::exec_script(machine, &query) {
            Ok(out) if out.success() => hasher::hash_string(&out.stdout),
            Ok(out) => {
                findings.push(DriftFinding {
                    resource_id: id.clone(),
                    resource_type: rl.resource_type.clone(),
                    expected_hash: stored_live_hash.to_string(),
                    actual_hash: "ERROR".to_string(),
                    detail: format!(
                        "state query failed: {}",
                        out.stderr.trim()
                    ),
                });
                continue;
            }
            Err(e) => {
                findings.push(DriftFinding {
                    resource_id: id.clone(),
                    resource_type: rl.resource_type.clone(),
                    expected_hash: stored_live_hash.to_string(),
                    actual_hash: "ERROR".to_string(),
                    detail: format!("transport error: {}", e),
                });
                continue;
            }
        };

        if actual_hash != stored_live_hash {
            findings.push(DriftFinding {
                resource_id: id.clone(),
                resource_type: rl.resource_type.clone(),
                expected_hash: stored_live_hash.to_string(),
                actual_hash,
                detail: format!("{} state changed", rl.resource_type),
            });
        }
    }

    findings
}

fn detect_drift_impl(lock: &StateLock, machine: Option<&Machine>) -> Vec<DriftFinding> {
    let mut findings = Vec::new();

    for (id, rl) in &lock.resources {
        if rl.status != ResourceStatus::Converged {
            continue;
        }

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

                    // Use transport for container/remote machines, local hash for local
                    let finding = match machine {
                        Some(m) if m.is_container_transport() => {
                            check_file_drift_via_transport(id, path, expected, m)
                        }
                        _ => check_file_drift(id, path, expected),
                    };

                    if let Some(f) = finding {
                        findings.push(f);
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

    #[test]
    fn test_fj016_full_drift_skips_non_file_without_live_hash() {
        // Non-file resource without live_hash should be skipped by detect_drift_full
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "my-pkg".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Package,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:abc".to_string(),
                details: std::collections::HashMap::new(), // no live_hash
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

        // Local machine (127.0.0.1) — no real transport needed since there's no live_hash
        let machine = Machine {
            hostname: "test-box".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
        };

        let config_resources = indexmap::IndexMap::new();
        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(
            findings.is_empty(),
            "non-file resources without live_hash should be skipped"
        );
    }

    #[test]
    fn test_fj016_full_drift_skips_non_converged() {
        // Non-converged resources should be skipped regardless of type
        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:xxx".to_string()),
        );
        resources.insert(
            "failed-pkg".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Package,
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
        let machine = Machine {
            hostname: "test-box".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
        };
        let config_resources = indexmap::IndexMap::new();
        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(findings.is_empty(), "failed resources should be skipped");
    }

    #[test]
    fn test_fj016_full_drift_skips_missing_resource_config() {
        // Resource with live_hash but not in config should be skipped
        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:xxx".to_string()),
        );
        resources.insert(
            "orphan-pkg".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Package,
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
        let machine = Machine {
            hostname: "test-box".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
        };
        // Empty config — resource not found
        let config_resources = indexmap::IndexMap::new();
        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(
            findings.is_empty(),
            "resources not in config should be skipped"
        );
    }
}
