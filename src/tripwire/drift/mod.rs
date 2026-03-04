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
            detail: format!("{path} does not exist"),
        });
    }

    let actual = if file_path.is_dir() {
        hasher::hash_directory(file_path).unwrap_or_else(|e| format!("ERROR:{e}"))
    } else {
        hasher::hash_file(file_path).unwrap_or_else(|e| format!("ERROR:{e}"))
    };

    if actual != expected_hash {
        Some(DriftFinding {
            resource_id: resource_id.to_string(),
            resource_type: ResourceType::File,
            expected_hash: expected_hash.to_string(),
            actual_hash: actual,
            detail: format!("{path} content changed"),
        })
    } else {
        None
    }
}

/// Compute the hash of a remote file or directory via transport.
fn hash_remote_content(
    out: &crate::transport::ExecOutput,
    path: &str,
    machine: &Machine,
) -> Option<String> {
    if out.stdout.trim() == "__DIR__" {
        let ls_script = format!("ls -la '{path}'");
        match crate::transport::exec_script(machine, &ls_script) {
            Ok(ls_out) if ls_out.success() => Some(hasher::hash_string(&ls_out.stdout)),
            _ => None,
        }
    } else {
        Some(hasher::hash_string(&out.stdout))
    }
}

/// Build a DriftFinding for a changed file.
fn file_drift_finding(
    resource_id: &str,
    expected_hash: &str,
    actual_hash: String,
    detail: String,
) -> DriftFinding {
    DriftFinding {
        resource_id: resource_id.to_string(),
        resource_type: ResourceType::File,
        expected_hash: expected_hash.to_string(),
        actual_hash,
        detail,
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
        "set -euo pipefail\nif [ -d '{path}' ]; then echo '__DIR__'; else cat '{path}'; fi"
    );
    match crate::transport::exec_script(machine, &script) {
        Ok(out) if out.success() => {
            let actual = hash_remote_content(&out, path, machine)?;
            if actual != expected_hash {
                Some(file_drift_finding(
                    resource_id,
                    expected_hash,
                    actual,
                    format!("{path} content changed"),
                ))
            } else {
                None
            }
        }
        Ok(out) => Some(file_drift_finding(
            resource_id,
            expected_hash,
            "MISSING".to_string(),
            format!("{} not accessible: {}", path, out.stderr.trim()),
        )),
        Err(e) => Some(file_drift_finding(
            resource_id,
            expected_hash,
            "ERROR".to_string(),
            format!("transport error: {e}"),
        )),
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

/// Check a non-file resource for drift by running its state_query_script.
fn check_nonfile_drift(
    id: &str,
    rl: &crate::core::types::ResourceLock,
    resource: &Resource,
    machine: &Machine,
    stored_live_hash: &str,
) -> Option<DriftFinding> {
    let query = match crate::core::codegen::state_query_script(resource) {
        Ok(q) => q,
        Err(_) => return None,
    };

    match crate::transport::exec_script(machine, &query) {
        Ok(out) if out.success() => {
            let actual_hash = hasher::hash_string(&out.stdout);
            if actual_hash != stored_live_hash {
                Some(DriftFinding {
                    resource_id: id.to_string(),
                    resource_type: rl.resource_type.clone(),
                    expected_hash: stored_live_hash.to_string(),
                    actual_hash,
                    detail: format!("{} state changed", rl.resource_type),
                })
            } else {
                None
            }
        }
        Ok(out) => Some(DriftFinding {
            resource_id: id.to_string(),
            resource_type: rl.resource_type.clone(),
            expected_hash: stored_live_hash.to_string(),
            actual_hash: "ERROR".to_string(),
            detail: format!("state query failed: {}", out.stderr.trim()),
        }),
        Err(e) => Some(DriftFinding {
            resource_id: id.to_string(),
            resource_type: rl.resource_type.clone(),
            expected_hash: stored_live_hash.to_string(),
            actual_hash: "ERROR".to_string(),
            detail: format!("transport error: {e}"),
        }),
    }
}

/// Full drift detection: files via hash comparison, non-file resources via state_query_script.
/// Requires the config resources to reconstruct state query scripts.
/// FJ-1220: Resources with lifecycle.ignore_drift are skipped.
pub fn detect_drift_full(
    lock: &StateLock,
    machine: &Machine,
    resources: &indexmap::IndexMap<String, Resource>,
) -> Vec<DriftFinding> {
    let mut findings = detect_drift_with_lifecycle(lock, Some(machine), resources);

    for (id, rl) in &lock.resources {
        if rl.status != ResourceStatus::Converged || rl.resource_type == ResourceType::File {
            continue;
        }
        // FJ-1220: skip resources with ignore_drift containing "*" or the resource type
        if should_ignore_drift(id, resources) {
            continue;
        }
        let stored_live_hash = match rl.details.get("live_hash") {
            Some(serde_yaml_ng::Value::String(s)) => s.as_str(),
            _ => continue,
        };
        let resource = match resources.get(id) {
            Some(r) => r,
            None => continue,
        };
        if let Some(f) = check_nonfile_drift(id, rl, resource, machine, stored_live_hash) {
            findings.push(f);
        }
    }

    findings
}

/// FJ-1220: Check if a resource's lifecycle rules say to ignore drift.
fn should_ignore_drift(
    resource_id: &str,
    resources: &indexmap::IndexMap<String, Resource>,
) -> bool {
    if let Some(resource) = resources.get(resource_id) {
        if let Some(ref lifecycle) = resource.lifecycle {
            // ignore_drift: ["*"] means skip all drift
            // ignore_drift: ["content", "mode"] means skip specific fields (treated as skip-all for now)
            return !lifecycle.ignore_drift.is_empty();
        }
    }
    false
}

/// Drift detection for file resources, respecting lifecycle.ignore_drift.
fn detect_drift_with_lifecycle(
    lock: &StateLock,
    machine: Option<&Machine>,
    resources: &indexmap::IndexMap<String, Resource>,
) -> Vec<DriftFinding> {
    let mut findings = Vec::new();

    for (id, rl) in &lock.resources {
        if rl.status != ResourceStatus::Converged || rl.resource_type != ResourceType::File {
            continue;
        }
        // FJ-1220: skip resources with ignore_drift
        if should_ignore_drift(id, resources) {
            continue;
        }
        if let Some(f) = check_file_resource_drift(id, rl, machine) {
            findings.push(f);
        }
    }

    findings
}

/// Extract path and content_hash from a file resource lock entry and check for drift.
fn check_file_resource_drift(
    id: &str,
    rl: &crate::core::types::ResourceLock,
    machine: Option<&Machine>,
) -> Option<DriftFinding> {
    let path = match rl.details.get("path") {
        Some(serde_yaml_ng::Value::String(s)) => s.as_str(),
        _ => return None,
    };
    let expected = match rl.details.get("content_hash") {
        Some(serde_yaml_ng::Value::String(s)) => s.as_str(),
        _ => return None,
    };
    match machine {
        Some(m) if m.is_container_transport() => {
            check_file_drift_via_transport(id, path, expected, m)
        }
        _ => check_file_drift(id, path, expected),
    }
}

fn detect_drift_impl(lock: &StateLock, machine: Option<&Machine>) -> Vec<DriftFinding> {
    let mut findings = Vec::new();

    for (id, rl) in &lock.resources {
        if rl.status != ResourceStatus::Converged || rl.resource_type != ResourceType::File {
            continue;
        }
        if let Some(f) = check_file_resource_drift(id, rl, machine) {
            findings.push(f);
        }
    }

    findings
}

#[cfg(test)]
mod tests_basic;
#[cfg(test)]
mod tests_basic_b;
#[cfg(test)]
mod tests_edge_fj131;
#[cfg(test)]
mod tests_edge_fj132;
#[cfg(test)]
mod tests_edge_fj132_b;
#[cfg(test)]
mod tests_fj036;
#[cfg(test)]
mod tests_full;
#[cfg(test)]
mod tests_lifecycle;
#[cfg(test)]
mod tests_transport;
