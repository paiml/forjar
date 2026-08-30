//! forjar#385: drift for a machine that has NO lock at all.
//!
//! # The defect
//!
//! Every other detector in this module walks `lock.resources`, so the whole of
//! `forjar drift` was gated on a state dir being readable. paiml/infra gitignores
//! `state/` — the lock lives on whichever box ran `apply` — so the fleet's
//! nightly drift lane produced this on every CI checkout, for as long as it has
//! existed:
//!
//! ```text
//! FAIL gx10         forjar drift exited 1: error: cannot read state dir .../infra/state
//! drift-tripwire: 0 of 2 requested machine(s) measured
//! FAIL: no machine was measured — this run measured NOTHING
//! ```
//!
//! # Why a missing lock does not have to mean a missing answer
//!
//! `task_check`'s reasoning, applied one level up: for a `type: task` the
//! observable is an ASSERTION, not a baseline. A `completion_check` that fails
//! right now is drift whether or not anything was ever written down about it,
//! so a run with no lock can still execute every check and give a TRUE answer
//! about the host. What it cannot do is compare a hash against a baseline that
//! does not exist — File content, Image digests, every `state_query_script`
//! comparison. That is a SMALLER answer, not an invalid one, and the census is
//! what keeps the difference visible: those resources are counted and
//! attributed to `SkipReason::NoLock` rather than folded into a clean verdict.
//!
//! # The line this does not cross
//!
//! ABSENT is not UNREADABLE. "Never applied from here" is the routine state of
//! a fresh checkout; "present and I cannot read it" is a broken host, and the
//! caller (`cli::drift_state`) keeps that fatal. Collapsing the two would be
//! the same reported-not-measured defect in a new place.

use super::census::{DriftCensus, SkipReason};
use super::ignore::should_ignore_drift;
use super::task_check::{self, DriftOptions};
use super::DriftReport;
use crate::core::types::{Machine, Resource, ResourceType};

/// Drift over a machine with no lock: run every assertion, count everything
/// else as uninspected.
///
/// `resources` must already be template-resolved (PMAT-197) — an unresolved
/// `{{params.*}}` in a `completion_check` would execute a different script than
/// the one `apply` runs, which is a wrong answer, not a missing one.
pub fn detect_drift_lockless(
    machine_name: &str,
    machine: &Machine,
    resources: &indexmap::IndexMap<String, Resource>,
    opts: DriftOptions,
) -> DriftReport {
    let mut census = DriftCensus::new();
    let mut findings = Vec::new();
    for (id, resource) in resources {
        if !targets(resource, machine_name) {
            continue;
        }
        if let Some(reason) = skip_reason(id, resource, resources, opts) {
            census.skipped(id, &resource.resource_type, reason);
            continue;
        }
        census.inspected(id, &resource.resource_type);
        if let Some(f) = task_check::check_task_drift(id, resource, machine) {
            findings.push(f);
        }
    }
    DriftReport { findings, census }
}

/// Is this resource in scope for this machine's lockless scan?
///
/// `Recipe` is excluded for the same reason `census_declared_but_unlocked`
/// excludes it: a recipe is expanded into concrete resources before apply, so
/// its own id is never a lock key and counting it would manufacture a permanent
/// phantom in the denominator.
fn targets(resource: &Resource, machine_name: &str) -> bool {
    resource.resource_type != ResourceType::Recipe
        && resource.machine.iter().any(|m| m == machine_name)
}

/// Why this resource would not be measured, or `None` to run its assertion.
///
/// The reason answers "why was this not measured", so the BINDING constraint
/// wins. For anything the task detector does not own, that constraint is the
/// missing lock — it has no assertion to execute and no baseline to compare
/// against, and reporting `lifecycle.ignore_drift` there would suggest the
/// exemption is what stopped forjar looking when the lock is.
fn skip_reason(
    id: &str,
    resource: &Resource,
    resources: &indexmap::IndexMap<String, Resource>,
    opts: DriftOptions,
) -> Option<SkipReason> {
    if !task_check::owns(resource) {
        return Some(SkipReason::NoLock);
    }
    if should_ignore_drift(id, resources) {
        return Some(SkipReason::IgnoreDrift);
    }
    if !opts.run_task_checks {
        return Some(SkipReason::TaskChecksDisabled);
    }
    None
}

/// The ids a lockless run would execute, for `drift --dry-run`.
///
/// A preview that dies where the run succeeds is a worse answer than no
/// preview, and one that names resources the run will not touch is worse
/// still — so this is the same predicate, not a second copy of it.
pub fn lockless_dry_run_ids(
    machine_name: &str,
    resources: &indexmap::IndexMap<String, Resource>,
    opts: DriftOptions,
) -> Vec<String> {
    resources
        .iter()
        .filter(|(id, r)| targets(r, machine_name) && skip_reason(id, r, resources, opts).is_none())
        .map(|(id, _)| id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn machine() -> Machine {
        serde_yaml_ng::from_str("hostname: sandbox\naddr: 127.0.0.1").unwrap()
    }

    fn task(check: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Task,
            machine: MachineTarget::Single("sandbox".to_string()),
            command: Some("exit 1".to_string()),
            completion_check: Some(check.to_string()),
            ..Default::default()
        }
    }

    fn file() -> Resource {
        Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("sandbox".to_string()),
            path: Some("/tmp/forjar-385-unit".to_string()),
            content: Some("x\n".to_string()),
            ..Default::default()
        }
    }

    fn resources(pairs: Vec<(&str, Resource)>) -> indexmap::IndexMap<String, Resource> {
        pairs
            .into_iter()
            .map(|(id, r)| (id.to_string(), r))
            .collect()
    }

    /// A satisfied assertion is not drift, and it IS inspected.
    #[test]
    fn a_satisfied_assertion_is_inspected_and_clean() {
        let res = resources(vec![("guard", task("true"))]);
        let report = detect_drift_lockless("sandbox", &machine(), &res, DriftOptions::default());
        assert!(report.findings.is_empty());
        assert_eq!(report.census.inspected_total(), 1);
        assert_eq!(report.census.skipped_total(), 0);
    }

    /// A violated assertion is drift with no baseline anywhere.
    #[test]
    fn a_violated_assertion_is_drift_without_a_lock() {
        let res = resources(vec![("guard", task("false"))]);
        let report = detect_drift_lockless("sandbox", &machine(), &res, DriftOptions::default());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].resource_id, "guard");
    }

    /// What needs a baseline is COUNTED, attributed to the missing lock.
    #[test]
    fn a_baseline_resource_is_skipped_as_no_lock() {
        let res = resources(vec![("guard", task("true")), ("hosts", file())]);
        let report = detect_drift_lockless("sandbox", &machine(), &res, DriftOptions::default());
        assert_eq!(report.census.in_scope(), 2);
        assert_eq!(report.census.inspected_total(), 1);
        assert_eq!(
            report
                .census
                .skipped_by_reason()
                .get("no lock (never applied from here)"),
            Some(&1)
        );
    }

    /// Another machine's resources are not in this machine's denominator.
    #[test]
    fn a_resource_for_another_machine_is_out_of_scope() {
        let mut elsewhere = task("false");
        elsewhere.machine = MachineTarget::Single("other".to_string());
        let res = resources(vec![("guard", elsewhere)]);
        let report = detect_drift_lockless("sandbox", &machine(), &res, DriftOptions::default());
        assert_eq!(report.census.in_scope(), 0);
        assert!(report.findings.is_empty());
    }

    /// `--no-task-checks` declines the work and says which flag declined it.
    #[test]
    fn no_task_checks_is_reported_not_silent() {
        let res = resources(vec![("guard", task("false"))]);
        let opts = DriftOptions {
            run_task_checks: false,
        };
        let report = detect_drift_lockless("sandbox", &machine(), &res, opts);
        assert!(report.findings.is_empty());
        assert_eq!(
            report.census.skipped_by_reason().get("--no-task-checks"),
            Some(&1)
        );
    }

    /// The preview names exactly what the run will execute.
    #[test]
    fn the_dry_run_ids_are_the_ones_the_run_executes() {
        let res = resources(vec![("guard", task("true")), ("hosts", file())]);
        let ids = lockless_dry_run_ids("sandbox", &res, DriftOptions::default());
        assert_eq!(ids, vec!["guard".to_string()]);
    }
}
