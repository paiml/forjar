//! FJ-2106/E15: image drift — the deployed digest against the built one.
//!
//! Split out of `mod.rs` when forjar#380 added the task detector and the
//! census: the file was at 497 of its 500-line budget, and the image path is
//! the piece with the fewest ties to the rest.

use super::census::{DriftCensus, SkipReason};
use super::ignore::should_ignore_drift;
use super::{DriftFinding, DRIFT_QUERY_TIMEOUT_SECS};
use crate::core::types::{Machine, Resource, ResourceStatus, ResourceType, StateLock};

/// FJ-2106/E15: Check all image-type resources for drift.
///
/// For each converged image resource, compares the manifest digest stored
/// in the lock file against the running container's image digest
/// (via `docker inspect`).
pub(super) fn detect_image_drift(
    lock: &StateLock,
    machine: &Machine,
    resources: &indexmap::IndexMap<String, Resource>,
    census: &mut DriftCensus,
) -> Vec<DriftFinding> {
    let mut findings = Vec::new();
    for (id, rl) in &lock.resources {
        if rl.resource_type != ResourceType::Image {
            continue;
        }
        if rl.status != ResourceStatus::Converged {
            census.skipped(id, &rl.resource_type, SkipReason::NotConverged);
            continue;
        }
        if should_ignore_drift(id, resources) {
            census.skipped(id, &rl.resource_type, SkipReason::IgnoreDrift);
            continue;
        }
        // A manifest digest or a container name the lock never recorded is the
        // same blindness as an unobserved task: there is nothing to compare, so
        // say so rather than passing over it in silence.
        let Some(expected_digest) = rl.detail_str("manifest_digest") else {
            census.skipped(id, &rl.resource_type, SkipReason::NoLockedHash);
            continue;
        };
        let Some(container_name) = rl.detail_str("container_name") else {
            census.skipped(id, &rl.resource_type, SkipReason::NoLockedHash);
            continue;
        };
        census.inspected(id, &rl.resource_type);
        if let Some(f) = check_image_drift(id, container_name, expected_digest, machine) {
            findings.push(f);
        }
    }
    findings
}

/// FJ-2106/E15: Check a single image resource for drift.
///
/// Runs `docker inspect <container> --format '{{.Image}}'` on the target
/// machine and compares the actual image digest to the expected manifest
/// digest from the build.
pub fn check_image_drift(
    resource_id: &str,
    container_name: &str,
    expected_digest: &str,
    machine: &Machine,
) -> Option<DriftFinding> {
    let script = format!(
        "docker inspect {container_name} --format '{{{{.Image}}}}' 2>/dev/null || echo 'NOT_RUNNING'"
    );
    match crate::transport::exec_script_timeout(machine, &script, Some(DRIFT_QUERY_TIMEOUT_SECS)) {
        Ok(out) if out.success() => {
            let actual = out.stdout.trim().to_string();
            if actual == "NOT_RUNNING" {
                Some(DriftFinding {
                    resource_id: resource_id.to_string(),
                    resource_type: ResourceType::Image,
                    expected_hash: expected_digest.to_string(),
                    actual_hash: "NOT_RUNNING".to_string(),
                    detail: format!("container {container_name} is not running"),
                })
            } else if actual != expected_digest {
                Some(DriftFinding {
                    resource_id: resource_id.to_string(),
                    resource_type: ResourceType::Image,
                    expected_hash: expected_digest.to_string(),
                    actual_hash: actual,
                    detail: "deployed image differs from built image".to_string(),
                })
            } else {
                None
            }
        }
        Ok(out) => Some(DriftFinding {
            resource_id: resource_id.to_string(),
            resource_type: ResourceType::Image,
            expected_hash: expected_digest.to_string(),
            actual_hash: "ERROR".to_string(),
            detail: format!("docker inspect failed: {}", out.stderr.trim()),
        }),
        Err(e) => Some(DriftFinding {
            resource_id: resource_id.to_string(),
            resource_type: ResourceType::Image,
            expected_hash: expected_digest.to_string(),
            actual_hash: "ERROR".to_string(),
            detail: format!("transport error: {e}"),
        }),
    }
}
