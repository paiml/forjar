//! FJ-016: Drift detection — compare live state to lock hashes.

use crate::core::types::{Machine, Resource, ResourceStatus, ResourceType, StateLock};
use crate::tripwire::hasher;
use file::{detect_drift_impl, detect_drift_with_lifecycle};
use ignore::should_ignore_drift;

/// A single drift finding.
#[derive(Debug, Clone)]
pub struct DriftFinding {
    /// Resource identifier.
    pub resource_id: String,
    /// Type of resource that drifted.
    pub resource_type: ResourceType,
    /// Expected hash from the lock file.
    pub expected_hash: String,
    /// Actual hash from live state.
    pub actual_hash: String,
    /// Human-readable drift description.
    pub detail: String,
}

/// Check all file-type resources in a lock for drift.
/// Bound on every transport call the DRIFT DETECTOR makes.
///
/// forjar#310. `check_nonfile_drift` used bare `transport::exec_script`, which
/// has no timeout, while the identical query at its original call site
/// (`executor/resource_ops.rs:46`) has always used `exec_script_timeout`.
/// Harmless while drift detection was a reporting command a human ran and could
/// Ctrl-C. #307 put it on the APPLY path, so one host that accepts a TCP
/// connection and then stalls hangs `apply` forever — measured: 0 bytes of
/// output, and every healthy machine in the same run left unconverged.
///
/// This fleet has documented wedged-switch and hung-NAS-mount history, so that
/// is a live shape, not a hypothetical. A state query is a `stat`, a `cat` and
/// a hash; if it has not answered in this long, the answer is not coming.
const DRIFT_QUERY_TIMEOUT_SECS: u64 = 60;

/// Findings plus the DENOMINATOR they were drawn from.
///
/// forjar#380: every entry point that returns a bare `Vec<DriftFinding>` hands
/// its caller a numerator with no population attached, and an empty vector then
/// renders as "No drift detected." whether it looked at everything or nothing.
/// The detectors now fill a census as they go; the bare-`Vec` wrappers below are
/// kept for callers that genuinely only want findings.
pub struct DriftReport {
    /// What drifted.
    pub findings: Vec<DriftFinding>,
    /// What was inspected, what was skipped, and why.
    pub census: DriftCensus,
}

/// Uses local filesystem hashing (for local machines without transport context).
pub fn detect_drift(lock: &StateLock) -> Vec<DriftFinding> {
    detect_drift_reported(lock, None).findings
}

/// Check all file-type resources in a lock for drift, using transport for remote/container machines.
pub fn detect_drift_with_machine(lock: &StateLock, machine: &Machine) -> Vec<DriftFinding> {
    detect_drift_reported(lock, Some(machine)).findings
}

/// File-only drift, with the census that says so.
///
/// Reached when no config was loaded (`forjar drift` outside a config
/// directory, or over a machine the config does not name). Without the config
/// forjar cannot regenerate a state query, so files are all it can compare —
/// and the census now says that in the output instead of leaving the operator
/// to infer it from a clean bill of health over a package, a service and a
/// task nobody looked at.
pub fn detect_drift_reported(lock: &StateLock, machine: Option<&Machine>) -> DriftReport {
    let mut census = DriftCensus::new();
    let findings = detect_drift_impl(lock, machine, &mut census);
    for (id, rl) in &lock.resources {
        if rl.resource_type != ResourceType::File {
            census.skipped(id, &rl.resource_type, SkipReason::NoConfigLoaded);
        }
    }
    DriftReport { findings, census }
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

    match crate::transport::exec_script_timeout(machine, &query, Some(DRIFT_QUERY_TIMEOUT_SECS)) {
        Ok(out) if out.success() => {
            // STRONG contract: query stdout may be empty when state absent.
            let actual_hash = hasher::hash_string_or_sentinel(&out.stdout);
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
    detect_drift_full_reported(lock, machine, resources, DriftOptions::default()).findings
}

/// Full drift detection, with the census and the per-invocation bounds.
///
/// Detector order is fixed and the census depends on it (first skip reason
/// wins, inspected always wins): files, then tasks, then everything else by
/// state query, then images.
pub fn detect_drift_full_reported(
    lock: &StateLock,
    machine: &Machine,
    resources: &indexmap::IndexMap<String, Resource>,
    opts: DriftOptions,
) -> DriftReport {
    let mut census = DriftCensus::new();
    let mut findings = detect_drift_with_lifecycle(lock, Some(machine), resources, &mut census);
    findings.extend(task_check::detect_task_drift(
        lock,
        machine,
        resources,
        opts,
        &mut census,
    ));
    findings.extend(detect_nonfile_drift(lock, machine, resources, &mut census));
    findings.extend(image::detect_image_drift(
        lock,
        machine,
        resources,
        &mut census,
    ));
    census_declared_but_unlocked(lock, resources, &mut census);
    DriftReport { findings, census }
}

/// Count what this config declares for this machine that the lock has never
/// heard of.
///
/// This is the half of the denominator no detector can see: drift walks the
/// LOCK, so a resource that was never applied through this `--state-dir` is not
/// skipped by any rule — it is absent from the question. Measured on
/// paiml/infra's gx10, whose lock was written by forjar 1.10.0: 30 lock
/// entries against 62 declared resources, and the runner guard that prompted
/// forjar#380 is in the 32 nobody counted. Reporting it as DRIFT would be
/// wrong (drift is live-versus-lock, and "never applied" is a plan verdict);
/// reporting it as UNINSPECTED is exactly true.
///
/// `Recipe` is excluded because a recipe is expanded into concrete resources
/// before apply, so its own id is never a lock key — counting it would
/// manufacture a permanent phantom.
fn census_declared_but_unlocked(
    lock: &StateLock,
    resources: &indexmap::IndexMap<String, Resource>,
    census: &mut DriftCensus,
) {
    for (id, resource) in resources {
        if resource.resource_type == ResourceType::Recipe || lock.resources.contains_key(id) {
            continue;
        }
        if resource.machine.iter().any(|m| m == lock.machine) {
            census.skipped(id, &resource.resource_type, SkipReason::NotInLock);
        }
    }
}

/// Check all non-file converged resources for drift via state_query_script.
fn detect_nonfile_drift(
    lock: &StateLock,
    machine: &Machine,
    resources: &indexmap::IndexMap<String, Resource>,
    census: &mut DriftCensus,
) -> Vec<DriftFinding> {
    let mut findings = Vec::new();
    for (id, rl) in &lock.resources {
        // A task carrying a completion_check belongs to `task_check`, which has
        // already recorded its verdict and its census entry. Running the state
        // query here as well would execute the very same command a second time
        // — `task::state_query_script` IS `verdict::single(<the check>)` — and
        // report one violated guard as two findings.
        if resources.get(id).is_some_and(task_check::owns) {
            continue;
        }
        // FILE RESOURCES ARE NOT EXCLUDED ANY MORE.
        //
        // This read `|| rl.resource_type == ResourceType::File`, added with the
        // comment "already handled by detect_drift_impl" — which was FALSE when
        // written. `source:` support had landed 3h49m earlier the same evening
        // without extending `build_resource_details`, so a `source:` file never
        // gets a `content_hash` and `detect_drift_impl` returns None for it
        // (absence of evidence rendered as cleanliness). A later refactor folded
        // the two ifs together and deleted the comment, so the false premise
        // stopped being visible at the line.
        //
        // Measured on the fleet before this change: 320 of 329 locked file
        // resources carried NO content_hash — 97% invisible to drift — while
        // 323 carried a `live_hash` that nothing read. That hash comes from
        // `state_query_script` run ON THE TARGET through the transport and
        // covers content, owner, group, mode and existence, so it is strictly
        // stronger than the controller-side bytes-only `content_hash`.
        // (forjar#305.)
        // `Drifted` IS RE-CHECKED. It means "needs work", not "stop looking".
        //
        // This read `!= Converged`, which was correct while nothing ever wrote
        // `Drifted`. #307 started writing it — and turned the drift tripwire
        // into a gate that fires ONCE and then reports clean forever over a
        // still-tampered file:
        //
        //     tripwire before        -> 1 (drift detected, correct)
        //     apply --dry-run        -> lock status becomes `drifted`
        //     tripwire after         -> 0 (CLEAN) while bytes are still TAMPERED
        //
        // That is strictly worse than the #305 blindness it replaced: a gate
        // that never fired gets distrusted, a gate that fires once and then
        // lies gets TRUSTED. `--tripwire` is the CI gate. (forjar#310.)
        //
        // Failed/Unknown stay excluded: their lock hash records an apply that
        // did not complete, so it is not a baseline anything can be compared
        // against. `Drifted` is different — it was written by an apply that
        // OBSERVED a converged resource move, so the recorded hash is exactly
        // the baseline drift detection needs.
        if rl.status != ResourceStatus::Converged && rl.status != ResourceStatus::Drifted {
            census.skipped(id, &rl.resource_type, SkipReason::NotConverged);
            continue;
        }
        if should_ignore_drift(id, resources) {
            census.skipped(id, &rl.resource_type, SkipReason::IgnoreDrift);
            continue;
        }
        // `None` = NOT OBSERVED, not "unchanged" (see ResourceLock::observed):
        // this is the call site that read the wrong digest for five months.
        //
        // It is also the line that made every `--refresh`-seeded resource
        // invisible (forjar#380): seeding writes `observed: None`, so this
        // `continue` fires for a resource an apply DID find converged. For a
        // task the assertion is now run regardless, above; for the rest there
        // is genuinely no baseline to compare against, so the honest move is to
        // count it as uninspected rather than pass over it in silence.
        let Some(stored_live_hash) = rl.observed_state() else {
            census.skipped(id, &rl.resource_type, SkipReason::NoObservedState);
            continue;
        };
        let Some(resource) = resources.get(id) else {
            census.skipped(id, &rl.resource_type, SkipReason::NotInConfig);
            continue;
        };
        census.inspected(id, &rl.resource_type);
        if let Some(f) = check_nonfile_drift(id, rl, resource, machine, stored_live_hash) {
            findings.push(f);
        }
    }
    findings
}

mod census;
mod file;
mod ignore;
mod image;
mod lockless;
mod task_check;

pub use census::{DriftCensus, SkipReason};
pub use file::{check_file_drift, check_file_drift_via_transport};
pub use image::check_image_drift;
pub use lockless::{detect_drift_lockless, lockless_dry_run_ids};
pub use task_check::DriftOptions;

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
mod tests_full_b;
#[cfg(test)]
mod tests_image_drift;
#[cfg(test)]
mod tests_lifecycle;
#[cfg(test)]
mod tests_task_checks;
#[cfg(test)]
mod tests_transport;
