//! forjar#380: drift for `type: task` — execute the assertion, don't hash it.
//!
//! # The defect
//!
//! paiml/infra's policy is that a guard IS a forjar resource: `completion_check`
//! is the assertion, `command` reports the violation. Drift saw none of them.
//! Files were compared by content hash, images by manifest digest, and every
//! other type by re-running `state_query_script` and comparing its digest to
//! the one the lock recorded — which skips any resource whose lock entry has no
//! observed state at all. `executor::refresh_seed::converged_entry` writes
//! precisely that entry (`observed: None`, empty details) for every resource an
//! `apply --refresh` found already satisfied, i.e. for every CI checkout and
//! every reimaged box. Measured on 1.21.1, one task guard, marker deleted after
//! the apply:
//!
//!     forjar apply --refresh   -> 0 converged, 1 unchanged
//!     rm <the file the check asserts>
//!     forjar drift -m box      -> "No drift detected."   (exit 0)
//!
//! # Why this is not the hash path with a wider filter
//!
//! For a resource whose observable is an ASSERTION, the digest comparison asks
//! the wrong question. `hash("task=pending") != hash("task=completed")` detects
//! a CHANGE against a recorded baseline; with no baseline recorded there is
//! nothing to compare and the hash path reports clean. But an assertion needs
//! no baseline — a `completion_check` that fails right now is drift whether or
//! not anything was ever written down about it. So this detector runs the check
//! and reads its EXIT CODE, exactly as `apply` and `--refresh` do.
//!
//! # Side effects — the assumption this makes load-bearing
//!
//! A `completion_check` is supposed to be a pure predicate and NOTHING enforces
//! that. Running it from drift makes that assumption load-bearing on a command
//! operators cron. It is not a new class of exposure — `apply`, `plan` and
//! `--refresh` already execute the same script through the same transport, so
//! any check with side effects has been firing on every apply — but the
//! FREQUENCY is new, and a check that mutates will now mutate hourly. If that
//! matters for a given host, `--no-task-checks` turns it off for the run, and
//! the census reports how many resources that silenced.

use super::census::{DriftCensus, SkipReason};
use super::ignore::should_ignore_drift;
use super::{DriftFinding, DRIFT_QUERY_TIMEOUT_SECS};
use crate::core::types::{Machine, Resource, ResourceStatus, ResourceType, TaskMode};

/// Per-invocation bounds on how much work a drift run may do on the target.
#[derive(Debug, Clone, Copy)]
pub struct DriftOptions {
    /// Execute the `completion_check` of every converged task (default true).
    ///
    /// DEFAULT ON, deliberately. A flag that must be remembered to make a guard
    /// look at anything is the fleet's most repeated failure shape — a check
    /// that no-ops unless invoked just so reports green and gets trusted. The
    /// cost argument also does not survive contact with the code it replaces:
    /// for a task carrying a `completion_check`, `state_query_script` IS
    /// `verdict::single(<the check>, ...)`, so drift was already executing that
    /// command whenever the lock had an observed hash. This detector spends the
    /// same one execution per task and answers a better question with it. The
    /// genuinely new executions are the ones for tasks that were being SKIPPED
    /// — the population whose invisibility is the bug.
    pub run_task_checks: bool,
}

impl Default for DriftOptions {
    fn default() -> Self {
        Self {
            run_task_checks: true,
        }
    }
}

/// Does the task detector own this resource's drift verdict?
///
/// Service-mode tasks are excluded: their `check_script` asserts a PID file
/// rather than the declared `completion_check`, so they keep the state-query
/// path and its digest, which is what their lock entries were written against.
pub(super) fn owns(resource: &Resource) -> bool {
    resource.resource_type == ResourceType::Task
        && resource.completion_check.is_some()
        && resource.task_mode.as_ref() != Some(&TaskMode::Service)
}

/// Run the `completion_check` of every converged task, over the same transport
/// `apply` uses (`transport::exec_script` dispatches pepita > container > local
/// > ssh), under the same 60s bound as every other drift query.
pub(super) fn detect_task_drift(
    lock: &crate::core::types::StateLock,
    machine: &Machine,
    resources: &indexmap::IndexMap<String, Resource>,
    opts: DriftOptions,
    census: &mut DriftCensus,
) -> Vec<DriftFinding> {
    let mut findings = Vec::new();
    for (id, rl) in &lock.resources {
        if rl.resource_type != ResourceType::Task {
            continue;
        }
        // A task with no `completion_check` has no assertion to execute; its
        // observable is its output artifacts, which the state-query path
        // already digests. Recording nothing here leaves that path to census
        // it, so the two detectors cannot disagree about the same resource.
        let Some(resource) = resources.get(id).filter(|r| owns(r)) else {
            continue;
        };
        if let Some(reason) = skip_reason(rl, id, resources, opts) {
            census.skipped(id, &rl.resource_type, reason);
            continue;
        }
        census.inspected(id, &rl.resource_type);
        if let Some(f) = check_task_drift(id, resource, machine) {
            findings.push(f);
        }
    }
    findings
}

/// Why this task would not be checked, or `None` to check it.
fn skip_reason(
    rl: &crate::core::types::ResourceLock,
    id: &str,
    resources: &indexmap::IndexMap<String, Resource>,
    opts: DriftOptions,
) -> Option<SkipReason> {
    // `Drifted` is re-checked for the same reason the state-query path
    // re-checks it: it means "needs work", not "stop looking" (forjar#310).
    if rl.status != ResourceStatus::Converged && rl.status != ResourceStatus::Drifted {
        return Some(SkipReason::NotConverged);
    }
    if should_ignore_drift(id, resources) {
        return Some(SkipReason::IgnoreDrift);
    }
    if !opts.run_task_checks {
        return Some(SkipReason::TaskChecksDisabled);
    }
    None
}

/// Execute one task's completion check on the target. Non-zero exit is drift.
pub(super) fn check_task_drift(
    resource_id: &str,
    resource: &Resource,
    machine: &Machine,
) -> Option<DriftFinding> {
    // `codegen::check_script`, not the raw `completion_check` string: it is the
    // script `apply`'s pre-check and `--refresh`'s seeding already run, so it
    // carries the declared privilege context. A `sudo: true` guard checked
    // without that wrapper asks a different question than the apply answers
    // (#349), and for a root-only path it would answer "drifted" forever.
    let script = match crate::core::codegen::check_script(resource) {
        Ok(s) => s,
        Err(e) => {
            return Some(finding(
                resource_id,
                "ERROR",
                format!("codegen failed: {e}"),
            ))
        }
    };
    match crate::transport::exec_script_timeout(machine, &script, Some(DRIFT_QUERY_TIMEOUT_SECS)) {
        Ok(out) if out.success() => None,
        Ok(out) => Some(finding(
            resource_id,
            "completion_check: FAIL",
            format!(
                "completion_check fails on {}: {}",
                machine.hostname,
                marker(&out)
            ),
        )),
        Err(e) => Some(finding(
            resource_id,
            "ERROR",
            format!("transport error: {e}"),
        )),
    }
}

/// The verdict marker the check printed (`task=pending`), or its stderr.
///
/// Bounded: a `completion_check` is arbitrary shell and may print a megabyte.
fn marker(out: &crate::transport::ExecOutput) -> String {
    let last = out
        .stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .or_else(|| out.stderr.lines().find(|l| !l.trim().is_empty()))
        .unwrap_or("no output")
        .trim();
    last.chars().take(200).collect()
}

/// A task finding.
///
/// `expected_hash`/`actual_hash` carry words rather than digests here, because
/// an assertion has no hash: the expected state is "the check passes" and the
/// observed state is "it does not". Writing a plausible-looking digest into
/// those fields would be a value forjar had not measured.
fn finding(resource_id: &str, actual: &str, detail: String) -> DriftFinding {
    DriftFinding {
        resource_id: resource_id.to_string(),
        resource_type: ResourceType::Task,
        expected_hash: "completion_check: pass".to_string(),
        actual_hash: actual.to_string(),
        detail,
    }
}
