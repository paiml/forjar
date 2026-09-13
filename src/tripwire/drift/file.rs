//! File drift: locked bytes against the bytes on the target.
//!
//! Split out of `mod.rs` when forjar#380 added the task detector and the
//! census, which took the file past its 500-line budget. `mod.rs` keeps the
//! orchestration — which detectors run, in what order, and what the census
//! says about the result — and each detector's mechanics live beside it.

use super::census::{DriftCensus, SkipReason};
use super::ignore::should_ignore_drift;
use super::unmeasured::{self, Reading};
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

/// What the target holds at a path, as both the drift and the apply side read it.
#[derive(Debug)]
pub(super) enum RemoteContent {
    /// The target answered: a file's bytes, or a directory's `ls -la`, digested.
    Digest(String),
    /// The target answered with a failure: a directory whose listing it could
    /// not produce. The text is the listing's stderr.
    Failed(String),
    /// The target never answered the listing (forjar#549).
    Unmeasured(String),
}

/// Digest a remote file from the first query's output, or a directory from the
/// second query (`ls -la`) this function makes.
fn remote_content(
    out: &crate::transport::ExecOutput,
    path: &str,
    machine: &Machine,
) -> RemoteContent {
    // STRONG contract: `hash_string` rejects empty input. Drift queries may
    // legitimately return empty stdout when the file is missing or empty —
    // use `hash_string_or_sentinel` to stay inside the contract.
    if out.stdout.trim() != "__DIR__" {
        return RemoteContent::Digest(hasher::hash_string_or_sentinel(&out.stdout));
    }
    listing_digest(unmeasured::read(machine, &format!("ls -la '{path}'")))
}

/// forjar#549, the second query. A directory's digest comes from a listing, and
/// that query can go unanswered or fail as easily as the first. Both used to
/// become `None`, which the drift caller's `?` turned into no finding at all: a
/// clean verdict over a listing nobody read.
pub(super) fn listing_digest(listing: Reading) -> RemoteContent {
    match listing {
        Reading::Answered(ls) if ls.success() => {
            RemoteContent::Digest(hasher::hash_string_or_sentinel(&ls.stdout))
        }
        Reading::Answered(ls) => RemoteContent::Failed(ls.stderr.trim().to_string()),
        Reading::Unmeasured(why) => RemoteContent::Unmeasured(why),
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

/// THE ONE READER: what does that machine hold at this path, as a digest?
///
/// forjar#485. The apply path writes the baseline this module later reads, and
/// the two used different scripts. Drift asks `if [ -d ]; then echo __DIR__;
/// else cat; fi` and, on seeing `__DIR__`, digests `ls -la` instead. A plain
/// `cat` on the writing side therefore disagreed with it in two places: on a
/// DIRECTORY, where `cat` fails and drift digests a listing, and on a file
/// whose entire content is the literal `__DIR__`, where the writer digests the
/// string and the reader digests a listing — a permanent, confident mismatch on
/// a resource that is perfectly converged. A review lane found the second one.
///
/// Both sides call this now, so the protocol cannot be half-adopted. Its
/// lossiness is shared too: `exec_script` decodes stdout with
/// `String::from_utf8_lossy`, so a non-UTF-8 byte becomes U+FFFD on the way in.
/// That is a real limit of reading a file through a shell, and it is now
/// SYMMETRIC — both sides lose the same bytes — where before only one did.
pub fn remote_path_digest(path: &str, machine: &Machine) -> Option<String> {
    let script = format!(
        "set -euo pipefail\nif [ -d '{path}' ]; then echo '__DIR__'; else cat '{path}'; fi"
    );
    match crate::transport::exec_script_timeout(machine, &script, Some(DRIFT_QUERY_TIMEOUT_SECS)) {
        // The apply side records no baseline it did not read: a listing that
        // failed or never came back leaves no `content_hash`, and drift then
        // says `no hash recorded in the lock` instead of comparing to nothing.
        Ok(out) if out.success() => match remote_content(&out, path, machine) {
            RemoteContent::Digest(digest) => Some(digest),
            RemoteContent::Failed(_) | RemoteContent::Unmeasured(_) => None,
        },
        _ => None,
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
    let out = match unmeasured::read(machine, &script) {
        Reading::Answered(out) => out,
        // forjar#549: `ssh` exiting 255 used to reach the arm below as `Ok`
        // and be reported MISSING — the verdict for a file a REACHED host lacks.
        Reading::Unmeasured(why) => {
            return Some(DriftFinding::unmeasured(
                resource_id,
                ResourceType::File,
                expected_hash,
                format!("{path} not measured: {why}"),
            ))
        }
    };
    if !out.success() {
        return Some(file_drift_finding(
            resource_id,
            expected_hash,
            "MISSING".to_string(),
            format!("{} not accessible: {}", path, out.stderr.trim()),
        ));
    }
    content_verdict(
        resource_id,
        path,
        expected_hash,
        remote_content(&out, path, machine),
    )
}

/// The drift verdict for what the target holds at `path`.
pub(super) fn content_verdict(
    resource_id: &str,
    path: &str,
    expected_hash: &str,
    content: RemoteContent,
) -> Option<DriftFinding> {
    match content {
        RemoteContent::Digest(actual) if actual == expected_hash => None,
        RemoteContent::Digest(actual) => Some(file_drift_finding(
            resource_id,
            expected_hash,
            actual,
            format!("{path} content changed"),
        )),
        RemoteContent::Unmeasured(why) => Some(DriftFinding::unmeasured(
            resource_id,
            ResourceType::File,
            expected_hash,
            format!("{path} listing not measured: {why}"),
        )),
        // The host answered, and what it said is that the listing the baseline
        // was digested from cannot be produced now. That is not clean.
        RemoteContent::Failed(stderr) => Some(file_drift_finding(
            resource_id,
            expected_hash,
            "ERROR".to_string(),
            format!("{path} listing failed: {stderr}"),
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
        Some(m) if !reads_the_controller(m) => {
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

/// Is this machine's filesystem the controller's own?
///
/// Only a machine reached by the LOCAL transport is. A container or pepita
/// machine commonly declares `addr: 127.0.0.1` (or nothing routable at all),
/// and its files live inside the namespace — reading the controller's path of
/// the same name answers about the wrong host, which is forjar#407's defect
/// shape one transport over (E05 quorum, agy lane).
pub(super) fn reads_the_controller(m: &Machine) -> bool {
    // forjar#485: ONE definition, in transport, shared with the apply path that
    // writes the baseline this function reads. They disagreed once.
    crate::transport::controller_answers_for(m)
}
