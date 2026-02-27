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
mod tests_basic;
#[cfg(test)]
mod tests_transport;
#[cfg(test)]
mod tests_full;
#[cfg(test)]
mod tests_edge_fj131;
#[cfg(test)]
mod tests_edge_fj132;
#[cfg(test)]
mod tests_fj036;
