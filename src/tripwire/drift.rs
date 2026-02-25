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
                    detail: format!("state query failed: {}", out.stderr.trim()),
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
    use crate::core::types::{MachineTarget, Resource};

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
            cost: 0,
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
            cost: 0,
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
            cost: 0,
        };
        // Empty config — resource not found
        let config_resources = indexmap::IndexMap::new();
        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(
            findings.is_empty(),
            "resources not in config should be skipped"
        );
    }

    #[test]
    fn test_fj016_drift_finding_fields() {
        let finding = DriftFinding {
            resource_id: "nginx-config".to_string(),
            resource_type: ResourceType::File,
            expected_hash: "blake3:aaa".to_string(),
            actual_hash: "blake3:bbb".to_string(),
            detail: "content changed".to_string(),
        };
        assert_eq!(finding.resource_id, "nginx-config");
        assert_eq!(finding.resource_type, ResourceType::File);
        assert_ne!(finding.expected_hash, finding.actual_hash);
    }

    #[test]
    fn test_fj016_missing_file_detail_message() {
        let result = check_file_drift("my-conf", "/does/not/exist/at/all.conf", "blake3:abc");
        let finding = result.unwrap();
        assert_eq!(finding.actual_hash, "MISSING");
        assert!(
            finding.detail.contains("does not exist"),
            "detail should say file does not exist: {}",
            finding.detail
        );
    }

    #[test]
    fn test_fj016_content_drift_detail_message() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("x.txt");
        std::fs::write(&file, "original").unwrap();
        let result = check_file_drift("x", file.to_str().unwrap(), "blake3:wrong");
        let finding = result.unwrap();
        assert!(
            finding.detail.contains("content changed"),
            "detail should mention content changed: {}",
            finding.detail
        );
    }

    #[test]
    fn test_fj016_detect_drift_with_machine_local() {
        // detect_drift_with_machine on local machine uses local hash path
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("local.txt");
        std::fs::write(&file, "local content").unwrap();
        let hash = hasher::hash_file(&file).unwrap();

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
            "local-file".to_string(),
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
            machine: "local".to_string(),
            hostname: "local".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        let findings = detect_drift_with_machine(&lock, &machine);
        assert!(findings.is_empty(), "no drift expected for matching file");
    }

    #[test]
    fn test_fj016_detect_drift_with_machine_local_drift() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("changed.txt");
        std::fs::write(&file, "before").unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:stale".to_string()),
        );
        resources.insert(
            "changed-file".to_string(),
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
            machine: "local".to_string(),
            hostname: "local".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        let findings = detect_drift_with_machine(&lock, &machine);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].resource_id, "changed-file");
    }

    #[test]
    fn test_fj016_detect_drift_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = dir.path().join("a.txt");
        let file2 = dir.path().join("b.txt");
        std::fs::write(&file1, "aaa").unwrap();
        std::fs::write(&file2, "bbb").unwrap();
        let hash1 = hasher::hash_file(&file1).unwrap();

        let mut resources = indexmap::IndexMap::new();
        for (id, path, hash) in [
            ("file-a", file1.to_str().unwrap(), hash1.as_str()),
            ("file-b", file2.to_str().unwrap(), "blake3:wrong"),
        ] {
            let mut details = std::collections::HashMap::new();
            details.insert(
                "path".to_string(),
                serde_yaml_ng::Value::String(path.to_string()),
            );
            details.insert(
                "content_hash".to_string(),
                serde_yaml_ng::Value::String(hash.to_string()),
            );
            resources.insert(
                id.to_string(),
                crate::core::types::ResourceLock {
                    resource_type: ResourceType::File,
                    status: ResourceStatus::Converged,
                    applied_at: None,
                    duration_seconds: None,
                    hash: "blake3:x".to_string(),
                    details,
                },
            );
        }

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        // file-a matches, file-b drifted
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].resource_id, "file-b");
    }

    #[test]
    fn test_fj016_directory_drift_new_file_inside() {
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("watched");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("original.txt"), "content").unwrap();
        let hash = hasher::hash_directory(&subdir).unwrap();

        // No drift initially
        assert!(check_file_drift("dir", subdir.to_str().unwrap(), &hash).is_none());

        // Add a new file — drift detected
        std::fs::write(subdir.join("intruder.txt"), "surprise").unwrap();
        let finding = check_file_drift("dir", subdir.to_str().unwrap(), &hash).unwrap();
        assert_eq!(finding.resource_id, "dir");
        assert!(finding.detail.contains("content changed"));
    }

    #[test]
    fn test_fj016_missing_content_hash_skipped() {
        // File resource with path but no content_hash should be skipped
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("no-hash.txt");
        std::fs::write(&file, "data").unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
        );
        // no content_hash key
        resources.insert(
            "no-hash".to_string(),
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
            hostname: "test".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(
            findings.is_empty(),
            "missing content_hash should skip drift check"
        );
    }

    #[test]
    fn test_fj016_full_drift_non_string_live_hash_skipped() {
        // Non-file resource with non-string live_hash should be skipped
        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(999)),
        );
        resources.insert(
            "bad-live".to_string(),
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
            hostname: "test".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        let machine = Machine {
            hostname: "test".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        let config_resources = indexmap::IndexMap::new();
        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(
            findings.is_empty(),
            "non-string live_hash should be skipped"
        );
    }

    // ── Additional edge case tests ─────────────────────────────────

    #[test]
    fn test_fj016_drift_finding_debug_and_clone() {
        let f = DriftFinding {
            resource_id: "test".to_string(),
            resource_type: ResourceType::File,
            expected_hash: "blake3:aaa".to_string(),
            actual_hash: "blake3:bbb".to_string(),
            detail: "changed".to_string(),
        };
        // Debug
        let dbg = format!("{:?}", f);
        assert!(dbg.contains("test"));
        // Clone
        let c = f.clone();
        assert_eq!(c.resource_id, "test");
        assert_eq!(c.expected_hash, "blake3:aaa");
    }

    #[test]
    fn test_fj016_check_file_drift_transport_local() {
        // check_file_drift_via_transport on a local file should work
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("transport-test.txt");
        std::fs::write(&file, "via transport").unwrap();
        let expected = hasher::hash_string("via transport");

        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        let finding =
            check_file_drift_via_transport("f", file.to_str().unwrap(), &expected, &machine);
        assert!(finding.is_none(), "matching content should show no drift");
    }

    #[test]
    fn test_fj016_check_file_drift_transport_drift() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("transport-drift.txt");
        std::fs::write(&file, "original").unwrap();

        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        // Expected hash of different content
        let finding = check_file_drift_via_transport(
            "f",
            file.to_str().unwrap(),
            "blake3:wrong-hash",
            &machine,
        );
        assert!(finding.is_some(), "mismatched hash should detect drift");
        let f = finding.unwrap();
        assert!(f.detail.contains("content changed"));
    }

    #[test]
    fn test_fj016_check_file_drift_transport_missing_file() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        let finding = check_file_drift_via_transport(
            "missing",
            "/nonexistent/file/forjar-test.txt",
            "blake3:abc",
            &machine,
        );
        assert!(finding.is_some());
        let f = finding.unwrap();
        assert_eq!(f.actual_hash, "MISSING");
    }

    #[test]
    fn test_fj016_detect_drift_multiple_resources_mixed() {
        // Lock with 3 resources: file (no drift), file (drifted), package (skipped)
        let dir = tempfile::tempdir().unwrap();
        let good_file = dir.path().join("good.txt");
        let bad_file = dir.path().join("bad.txt");
        std::fs::write(&good_file, "good").unwrap();
        std::fs::write(&bad_file, "changed").unwrap();

        let good_hash = hasher::hash_file(&good_file).unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut good_details = std::collections::HashMap::new();
        good_details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(good_file.to_str().unwrap().to_string()),
        );
        good_details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(good_hash),
        );
        resources.insert(
            "good-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "desired".to_string(),
                details: good_details,
            },
        );

        let mut bad_details = std::collections::HashMap::new();
        bad_details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(bad_file.to_str().unwrap().to_string()),
        );
        bad_details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:stale-hash".to_string()),
        );
        resources.insert(
            "bad-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "desired".to_string(),
                details: bad_details,
            },
        );

        // Package resource should be skipped by detect_drift
        resources.insert(
            "pkg".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Package,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "desired".to_string(),
                details: std::collections::HashMap::new(),
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert_eq!(findings.len(), 1, "only the drifted file should appear");
        assert_eq!(findings[0].resource_id, "bad-file");
    }

    #[test]
    fn test_fj016_detect_drift_failed_resource_skipped() {
        // Failed resources should not be drift-checked
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
                hash: "".to_string(),
                details,
            },
        );
        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        let findings = detect_drift(&lock);
        assert!(findings.is_empty(), "failed resources should be skipped");
    }

    #[test]
    fn test_fj016_check_file_drift_empty_file() {
        // Empty file should still have a valid hash
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        std::fs::write(&file, "").unwrap();
        let hash = hasher::hash_file(&file).unwrap();
        let finding = check_file_drift("empty", file.to_str().unwrap(), &hash);
        assert!(finding.is_none(), "empty file with correct hash = no drift");
    }

    #[test]
    fn test_fj016_check_file_drift_wrong_hash_format() {
        // Even a non-blake3 hash should trigger drift if it doesn't match
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "test").unwrap();
        let finding = check_file_drift("f", file.to_str().unwrap(), "sha256:wrong");
        assert!(finding.is_some(), "wrong hash format should show as drift");
    }

    #[test]
    fn test_fj016_check_file_drift_transport_directory() {
        // Transport drift check on a directory
        let dir = tempfile::tempdir().unwrap();
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        // Using a directory path should work via transport (ls -la)
        let finding = check_file_drift_via_transport(
            "d",
            dir.path().to_str().unwrap(),
            "blake3:definitely-wrong",
            &machine,
        );
        assert!(finding.is_some(), "directory hash should differ from dummy");
    }

    // ── FJ-128: Drift detection edge case tests ──────────────────

    #[test]
    fn test_fj016_detect_drift_full_matching_live_hash() {
        // Non-file resource where live state matches stored live_hash → no drift
        let dir = tempfile::tempdir().unwrap();
        let test_file = dir.path().join("state-output.txt");
        std::fs::write(&test_file, "ActiveState=active\nSubState=running\n").unwrap();

        // Build a resource whose state_query_script cats a local file
        let mut config_resources = indexmap::IndexMap::new();
        config_resources.insert(
            "test-svc".to_string(),
            Resource {
                resource_type: ResourceType::Service,
                machine: MachineTarget::Single("m".to_string()),
                state: Some("present".to_string()),
                depends_on: vec![],
                provider: None,
                packages: vec![],
                version: None,
                path: None,
                content: None,
                source: None,
                target: None,
                owner: None,
                group: None,
                mode: None,
                name: Some("nginx".to_string()),
                enabled: None,
                restart_on: vec![],
                fs_type: None,
                options: None,
                uid: None,
                shell: None,
                home: None,
                groups: vec![],
                ssh_authorized_keys: vec![],
                system_user: false,
                schedule: None,
                command: None,
                image: None,
                ports: vec![],
                environment: vec![],
                volumes: vec![],
                restart: None,
                protocol: None,
                port: None,
                action: None,
                from_addr: None,
                recipe: None,
                inputs: std::collections::HashMap::new(),
                arch: vec![],
                tags: vec![],
            },
        );

        // Compute what the state_query_script for this service would produce
        let query =
            crate::core::codegen::state_query_script(config_resources.get("test-svc").unwrap())
                .unwrap();
        // Run it locally to get the output
        let machine = Machine {
            hostname: "test".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        let output = crate::transport::exec_script(&machine, &query).unwrap();
        let live_hash = hasher::hash_string(&output.stdout);

        let mut lock_resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String(live_hash),
        );
        lock_resources.insert(
            "test-svc".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Service,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        };

        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(
            findings.is_empty(),
            "matching live_hash should show no drift"
        );
    }

    #[test]
    fn test_fj016_detect_drift_full_mismatched_live_hash() {
        // Non-file resource where live state differs → drift detected
        let mut config_resources = indexmap::IndexMap::new();
        config_resources.insert(
            "test-svc".to_string(),
            Resource {
                resource_type: ResourceType::Service,
                machine: MachineTarget::Single("m".to_string()),
                state: Some("present".to_string()),
                depends_on: vec![],
                provider: None,
                packages: vec![],
                version: None,
                path: None,
                content: None,
                source: None,
                target: None,
                owner: None,
                group: None,
                mode: None,
                name: Some("nginx".to_string()),
                enabled: None,
                restart_on: vec![],
                fs_type: None,
                options: None,
                uid: None,
                shell: None,
                home: None,
                groups: vec![],
                ssh_authorized_keys: vec![],
                system_user: false,
                schedule: None,
                command: None,
                image: None,
                ports: vec![],
                environment: vec![],
                volumes: vec![],
                restart: None,
                protocol: None,
                port: None,
                action: None,
                from_addr: None,
                recipe: None,
                inputs: std::collections::HashMap::new(),
                arch: vec![],
                tags: vec![],
            },
        );

        let machine = Machine {
            hostname: "test".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        // Use a stale live_hash that won't match current systemctl output
        let mut lock_resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:stale-from-yesterday".to_string()),
        );
        lock_resources.insert(
            "test-svc".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Service,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        };

        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert_eq!(findings.len(), 1, "stale live_hash should detect drift");
        assert_eq!(findings[0].resource_id, "test-svc");
        assert!(findings[0].detail.contains("state changed"));
    }

    #[test]
    fn test_fj016_detect_drift_full_codegen_error_skips() {
        // Resource present in lock + config but codegen fails → should be skipped
        let mut config_resources = indexmap::IndexMap::new();
        // A resource with no name and no useful state_query info
        config_resources.insert(
            "broken-res".to_string(),
            Resource {
                resource_type: ResourceType::Service,
                machine: MachineTarget::Single("m".to_string()),
                state: Some("present".to_string()),
                depends_on: vec![],
                provider: None,
                packages: vec![],
                version: None,
                path: None,
                content: None,
                source: None,
                target: None,
                owner: None,
                group: None,
                mode: None,
                name: None, // no name — state_query_script should still work (defaults to "unknown")
                enabled: None,
                restart_on: vec![],
                fs_type: None,
                options: None,
                uid: None,
                shell: None,
                home: None,
                groups: vec![],
                ssh_authorized_keys: vec![],
                system_user: false,
                schedule: None,
                command: None,
                image: None,
                ports: vec![],
                environment: vec![],
                volumes: vec![],
                restart: None,
                protocol: None,
                port: None,
                action: None,
                from_addr: None,
                recipe: None,
                inputs: std::collections::HashMap::new(),
                arch: vec![],
                tags: vec![],
            },
        );

        let machine = Machine {
            hostname: "test".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        let mut lock_resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:old".to_string()),
        );
        lock_resources.insert(
            "broken-res".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Service,
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
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        };

        // This should not panic — codegen may succeed or fail, but drift detection should handle it
        let _findings = detect_drift_full(&lock, &machine, &config_resources);
        // The test verifies no panic occurs, and drift detection gracefully handles the case
    }

    #[test]
    fn test_fj016_detect_drift_full_file_plus_service() {
        // Mixed: file resource (local) + service resource (live_hash) in same lock
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mixed.txt");
        std::fs::write(&file, "stable").unwrap();
        let file_hash = hasher::hash_file(&file).unwrap();

        let mut config_resources = indexmap::IndexMap::new();
        config_resources.insert(
            "my-svc".to_string(),
            Resource {
                resource_type: ResourceType::Service,
                machine: MachineTarget::Single("m".to_string()),
                state: Some("present".to_string()),
                depends_on: vec![],
                provider: None,
                packages: vec![],
                version: None,
                path: None,
                content: None,
                source: None,
                target: None,
                owner: None,
                group: None,
                mode: None,
                name: Some("nginx".to_string()),
                enabled: None,
                restart_on: vec![],
                fs_type: None,
                options: None,
                uid: None,
                shell: None,
                home: None,
                groups: vec![],
                ssh_authorized_keys: vec![],
                system_user: false,
                schedule: None,
                command: None,
                image: None,
                ports: vec![],
                environment: vec![],
                volumes: vec![],
                restart: None,
                protocol: None,
                port: None,
                action: None,
                from_addr: None,
                recipe: None,
                inputs: std::collections::HashMap::new(),
                arch: vec![],
                tags: vec![],
            },
        );

        let machine = Machine {
            hostname: "test".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        // Run the real state query to get current live_hash
        let query =
            crate::core::codegen::state_query_script(config_resources.get("my-svc").unwrap())
                .unwrap();
        let output = crate::transport::exec_script(&machine, &query).unwrap();
        let svc_live_hash = hasher::hash_string(&output.stdout);

        let mut lock_resources = indexmap::IndexMap::new();

        // File resource — no drift
        let mut file_details = std::collections::HashMap::new();
        file_details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
        );
        file_details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(file_hash),
        );
        lock_resources.insert(
            "my-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details: file_details,
            },
        );

        // Service resource — no drift (live_hash matches)
        let mut svc_details = std::collections::HashMap::new();
        svc_details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String(svc_live_hash),
        );
        lock_resources.insert(
            "my-svc".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Service,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details: svc_details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        };

        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(
            findings.is_empty(),
            "no drift expected when both file and service hashes match"
        );
    }

    // ── FJ-131: drift edge case tests ─────────────────────────

    #[test]
    fn test_fj131_check_file_drift_directory() {
        // check_file_drift should hash directories too
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("a.txt"), "content").unwrap();
        let hash = hasher::hash_directory(&sub).unwrap();

        let result = check_file_drift("dir-resource", sub.to_str().unwrap(), &hash);
        assert!(
            result.is_none(),
            "directory with matching hash should not drift"
        );
    }

    #[test]
    fn test_fj131_check_file_drift_directory_changed() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("a.txt"), "original").unwrap();
        let hash = hasher::hash_directory(&sub).unwrap();

        // Modify directory contents
        std::fs::write(sub.join("a.txt"), "modified").unwrap();

        let result = check_file_drift("dir-resource", sub.to_str().unwrap(), &hash);
        assert!(result.is_some(), "changed directory should drift");
    }

    #[test]
    fn test_fj131_drift_finding_debug() {
        let f = DriftFinding {
            resource_id: "test".to_string(),
            resource_type: ResourceType::File,
            expected_hash: "a".to_string(),
            actual_hash: "b".to_string(),
            detail: "changed".to_string(),
        };
        let debug = format!("{:?}", f);
        assert!(debug.contains("test"));
        assert!(debug.contains("changed"));
    }

    #[test]
    fn test_fj131_drift_finding_clone() {
        let f = DriftFinding {
            resource_id: "res".to_string(),
            resource_type: ResourceType::Service,
            expected_hash: "h1".to_string(),
            actual_hash: "h2".to_string(),
            detail: "state changed".to_string(),
        };
        let cloned = f.clone();
        assert_eq!(cloned.resource_id, "res");
        assert_eq!(cloned.actual_hash, "h2");
    }

    #[test]
    fn test_fj131_detect_drift_skips_failed_resources() {
        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String("/tmp/nonexistent".to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:stale".to_string()),
        );
        resources.insert(
            "failed-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Failed, // not converged
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:x".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(findings.is_empty(), "failed resources should be skipped");
    }

    #[test]
    fn test_fj131_detect_drift_skips_non_string_path() {
        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        // Path is a number instead of string — should be skipped
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42)),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:x".to_string()),
        );
        resources.insert(
            "bad-path".to_string(),
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
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(findings.is_empty(), "non-string path should be skipped");
    }

    #[test]
    fn test_fj131_detect_drift_skips_non_string_content_hash() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "content").unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
        );
        // content_hash is a bool instead of string
        details.insert("content_hash".to_string(), serde_yaml_ng::Value::Bool(true));
        resources.insert(
            "bad-hash".to_string(),
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
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
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
    fn test_fj131_detect_drift_no_content_hash_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "content").unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        // Has path but no content_hash at all
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
        );
        resources.insert(
            "no-hash".to_string(),
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
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(
            findings.is_empty(),
            "missing content_hash should be skipped"
        );
    }

    #[test]
    fn test_fj131_detect_drift_skips_non_file_resources() {
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "my-svc".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Service,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:x".to_string(),
                details: std::collections::HashMap::new(),
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        // detect_drift (not detect_drift_full) only checks files
        let findings = detect_drift(&lock);
        assert!(
            findings.is_empty(),
            "service resources should be skipped by detect_drift"
        );
    }

    #[test]
    fn test_fj131_detect_drift_multiple_resources() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = dir.path().join("ok.txt");
        let file2 = dir.path().join("drifted.txt");
        std::fs::write(&file1, "stable").unwrap();
        std::fs::write(&file2, "original").unwrap();
        let hash1 = hasher::hash_file(&file1).unwrap();
        let hash2 = hasher::hash_file(&file2).unwrap();

        // Tamper with file2
        std::fs::write(&file2, "tampered").unwrap();

        let mut resources = indexmap::IndexMap::new();
        for (name, file, hash) in [
            ("ok-file", &file1, &hash1),
            ("drifted-file", &file2, &hash2),
        ] {
            let mut details = std::collections::HashMap::new();
            details.insert(
                "path".to_string(),
                serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
            );
            details.insert(
                "content_hash".to_string(),
                serde_yaml_ng::Value::String(hash.clone()),
            );
            resources.insert(
                name.to_string(),
                crate::core::types::ResourceLock {
                    resource_type: ResourceType::File,
                    status: ResourceStatus::Converged,
                    applied_at: None,
                    duration_seconds: None,
                    hash: "blake3:desired".to_string(),
                    details,
                },
            );
        }

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert_eq!(findings.len(), 1, "only drifted file should be reported");
        assert_eq!(findings[0].resource_id, "drifted-file");
    }

    // --- FJ-132: Additional drift edge case tests ---

    #[test]
    fn test_fj132_detect_drift_with_local_machine() {
        // detect_drift_with_machine on a local machine (addr: 127.0.0.1)
        // should use local hash, not transport
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("local.txt");
        std::fs::write(&file, "local content").unwrap();
        let hash = hasher::hash_file(&file).unwrap();

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
            "local-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        let findings = detect_drift_with_machine(&lock, &machine);
        assert!(
            findings.is_empty(),
            "no drift for matching file with local machine"
        );
    }

    #[test]
    fn test_fj132_detect_drift_with_machine_drift_detected() {
        // detect_drift_with_machine on local machine with tampered file
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("tamper.txt");
        std::fs::write(&file, "original").unwrap();
        let hash = hasher::hash_file(&file).unwrap();
        std::fs::write(&file, "tampered").unwrap();

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
            "tampered-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let machine = Machine {
            hostname: "local".to_string(),
            addr: "localhost".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        let findings = detect_drift_with_machine(&lock, &machine);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].resource_id, "tampered-file");
    }

    #[test]
    fn test_fj132_detect_drift_drifted_status_skipped() {
        // Resources with Drifted status should be skipped (only Converged checked)
        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String("/tmp/nonexistent".to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:abc".to_string()),
        );
        resources.insert(
            "drifted-resource".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Drifted,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(
            findings.is_empty(),
            "drifted status resources should be skipped"
        );
    }

    #[test]
    fn test_fj132_detect_drift_unknown_status_skipped() {
        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String("/tmp/nonexistent".to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:abc".to_string()),
        );
        resources.insert(
            "unknown-resource".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Unknown,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };

        let findings = detect_drift(&lock);
        assert!(
            findings.is_empty(),
            "unknown status resources should be skipped"
        );
    }

    #[test]
    fn test_fj132_detect_drift_full_skips_file_resources() {
        // detect_drift_full should not double-count file resources
        // (they're handled by detect_drift_impl, not the non-file loop)
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file.txt");
        std::fs::write(&file, "content").unwrap();
        let hash = hasher::hash_file(&file).unwrap();

        let mut lock_resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(hash),
        );
        lock_resources.insert(
            "my-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        };

        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        let config_resources = indexmap::IndexMap::new();
        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(
            findings.is_empty(),
            "matching file should not trigger drift in full mode"
        );
    }

    #[test]
    fn test_fj132_detect_drift_full_non_file_no_live_hash() {
        // Non-file resource without live_hash in details should be skipped
        let mut lock_resources = indexmap::IndexMap::new();
        let details = std::collections::HashMap::new(); // no live_hash
        lock_resources.insert(
            "my-service".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Service,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        };

        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        let config_resources = indexmap::IndexMap::new();
        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(
            findings.is_empty(),
            "service without live_hash should be skipped"
        );
    }

    #[test]
    fn test_fj132_detect_drift_full_non_file_non_string_live_hash() {
        // Non-file resource with non-string live_hash should be skipped
        let mut lock_resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42)),
        );
        lock_resources.insert(
            "my-package".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Package,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        };

        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        let config_resources = indexmap::IndexMap::new();
        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(
            findings.is_empty(),
            "non-string live_hash should be skipped"
        );
    }

    #[test]
    fn test_fj132_detect_drift_full_non_file_missing_config_resource() {
        // Non-file resource with live_hash but no matching config resource should be skipped
        let mut lock_resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:abc123".to_string()),
        );
        lock_resources.insert(
            "orphaned-service".to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::Service,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test".to_string(),
            generated_at: "now".to_string(),
            generator: "test".to_string(),
            blake3_version: "1.8".to_string(),
            resources: lock_resources,
        };

        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };

        // Empty config resources — the lock has a resource that config doesn't
        let config_resources = indexmap::IndexMap::new();
        let findings = detect_drift_full(&lock, &machine, &config_resources);
        assert!(
            findings.is_empty(),
            "orphaned lock resource should be skipped"
        );
    }

    #[test]
    fn test_fj132_drift_finding_resource_type_preserved() {
        // DriftFinding should preserve the resource type from the check
        let result = check_file_drift("test", "/nonexistent/path.txt", "blake3:abc");
        let finding = result.unwrap();
        assert_eq!(finding.resource_type, ResourceType::File);
        assert_eq!(finding.actual_hash, "MISSING");
    }

    #[test]
    fn test_fj132_check_file_drift_hash_error_format() {
        // When hash_file returns ERROR:, drift is detected with error prefix
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        std::fs::write(&file, "content").unwrap();

        // Use a wrong hash — result should show the real hash (no error)
        let result = check_file_drift("test", file.to_str().unwrap(), "blake3:wrong");
        let finding = result.unwrap();
        assert!(
            finding.actual_hash.starts_with("blake3:"),
            "actual hash should be valid blake3"
        );
        assert_ne!(finding.actual_hash, "blake3:wrong");
    }
}
