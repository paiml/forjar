//! File drift: locked bytes against the bytes on the target.
//!
//! Split out of `mod.rs` when forjar#380 added the task detector and the
//! census, which took the file past its 500-line budget. `mod.rs` keeps the
//! orchestration — which detectors run, in what order, and what the census
//! says about the result — and each detector's mechanics live beside it.

use super::census::{DriftCensus, SkipReason};
use super::ignore::should_ignore_drift;
use super::{DriftFinding, DRIFT_QUERY_TIMEOUT_SECS};
use crate::core::types::{Machine, Resource, ResourceStatus, ResourceType, StateLock};
use crate::tripwire::hasher;
use std::path::Path;

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
    // STRONG contract: `hash_string` rejects empty input. Drift queries may
    // legitimately return empty stdout when the file is missing or empty —
    // use `hash_string_or_sentinel` to stay inside the contract.
    if out.stdout.trim() == "__DIR__" {
        let ls_script = format!("ls -la '{path}'");
        match crate::transport::exec_script_timeout(
            machine,
            &ls_script,
            Some(DRIFT_QUERY_TIMEOUT_SECS),
        ) {
            Ok(ls_out) if ls_out.success() => Some(hasher::hash_string_or_sentinel(&ls_out.stdout)),
            _ => None,
        }
    } else {
        Some(hasher::hash_string_or_sentinel(&out.stdout))
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
    match crate::transport::exec_script_timeout(machine, &script, Some(DRIFT_QUERY_TIMEOUT_SECS)) {
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

/// Drift detection for file resources, respecting lifecycle.ignore_drift.
pub(super) fn detect_drift_with_lifecycle(
    lock: &StateLock,
    machine: Option<&Machine>,
    resources: &indexmap::IndexMap<String, Resource>,
    census: &mut DriftCensus,
) -> Vec<DriftFinding> {
    let mut findings = Vec::new();

    for (id, rl) in &lock.resources {
        if rl.resource_type != ResourceType::File {
            continue;
        }
        if rl.status != ResourceStatus::Converged {
            census.skipped(id, &rl.resource_type, SkipReason::NotConverged);
            continue;
        }
        // FJ-1220: skip resources with ignore_drift
        if should_ignore_drift(id, resources) {
            census.skipped(id, &rl.resource_type, SkipReason::IgnoreDrift);
            continue;
        }
        // A file with no `path`/`content_hash` in the lock is not comparable
        // HERE, but `detect_nonfile_drift` may still observe it through its
        // state query — which is why the census lets a later `inspected`
        // overwrite this skip rather than the other way round.
        let Some((path, expected)) = locked_file_target(rl) else {
            census.skipped(id, &rl.resource_type, SkipReason::NoLockedHash);
            continue;
        };
        census.inspected(id, &rl.resource_type);
        if let Some(f) = check_file_resource_drift(id, path, expected, machine) {
            findings.push(f);
        }
    }

    findings
}

/// The `(path, content_hash)` pair a file lock entry records, if it has one.
pub(super) fn locked_file_target(rl: &crate::core::types::ResourceLock) -> Option<(&str, &str)> {
    let path = match rl.details.get("path") {
        Some(serde_yaml_ng::Value::String(s)) => s.as_str(),
        _ => return None,
    };
    let expected = match rl.details.get("content_hash") {
        Some(serde_yaml_ng::Value::String(s)) => s.as_str(),
        _ => return None,
    };
    Some((path, expected))
}

/// Compare a locked file's `content_hash` against the bytes on the target.
pub(super) fn check_file_resource_drift(
    id: &str,
    path: &str,
    expected: &str,
    machine: Option<&Machine>,
) -> Option<DriftFinding> {
    // IF WE KNOW THE MACHINE, ASK THE MACHINE.
    //
    // This routed through the transport ONLY for container transports. Every
    // other machine — INCLUDING PLAIN SSH — fell to `check_file_drift`, which
    // takes no machine and hashes the CONTROLLER's filesystem, then reports the
    // answer as the remote host's state.
    //
    // That is forjar#305's root cause, still live in the other arm. Measured
    // 2026-08-24 against a real SSH host:
    //
    //     file at <path>            : present on the CONTROLLER, ABSENT on intel
    //     content_hash              : matches the controller's copy
    //     forjar drift (machine intel) -> "No drift detected."
    //
    // A false CLEAN over a file that does not exist on the target. The inverse
    // is equally reachable: a controller that happens to hold different bytes
    // at the same path produces a false DRIFT for a host that is perfectly
    // converged.
    //
    // `exec_script` already dispatches pepita > container > local > SSH, so a
    // local machine still executes locally and nothing needs a special case.
    // The container branch was not wrong — it was just the only one anybody
    // had needed yet.
    //
    // `None` means no machine is known (bare `detect_drift`, no config loaded).
    // The controller is then the only filesystem there is, and reading it is the
    // honest best effort rather than a wrong answer about somewhere else.
    match machine {
        // LOCAL means the controller IS the target, so a direct read is not
        // merely allowed — it is the same filesystem, and it is far cheaper.
        //
        // Routing local machines through the transport was correct and much too
        // slow: it spawns a shell per file resource instead of reading the file,
        // and CI's `behavior` and `benchmark` lanes both hit their 15-minute
        // timeout at exactly 15m01s. The defect being fixed is answering about
        // the WRONG HOST; for a local machine there is no other host to be
        // wrong about.
        Some(m) if !crate::transport::is_local_addr(&m.addr) => {
            check_file_drift_via_transport(id, path, expected, m)
        }
        _ => check_file_drift(id, path, expected),
    }
}

/// File drift with no config in hand: `lifecycle` cannot be consulted, so this
/// is `detect_drift_with_lifecycle` over an empty resource map.
pub(super) fn detect_drift_impl(
    lock: &StateLock,
    machine: Option<&Machine>,
    census: &mut DriftCensus,
) -> Vec<DriftFinding> {
    detect_drift_with_lifecycle(lock, machine, &indexmap::IndexMap::new(), census)
}
