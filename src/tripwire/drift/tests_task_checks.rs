//! forjar#380: a `type: task` guard is invisible to `forjar drift`.

use super::*;
use crate::core::types::{MachineTarget, Resource, ResourceLock, StateLock};
use indexmap::IndexMap;
use std::collections::HashMap;

fn local_machine() -> Machine {
    serde_yaml_ng::from_str("hostname: box\naddr: 127.0.0.1").unwrap()
}

/// A lock entry exactly as `--refresh` seeding writes it: converged, and with
/// NOTHING observed (see `executor::refresh_seed::converged_entry`).
fn seeded_task_entry() -> ResourceLock {
    ResourceLock {
        resource_type: ResourceType::Task,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: "blake3:desired".to_string(),
        observed: None,
        details: HashMap::new(),
    }
}

fn lock_with(id: &str, entry: ResourceLock) -> StateLock {
    let mut resources = IndexMap::new();
    resources.insert(id.to_string(), entry);
    StateLock {
        schema: "1.0".to_string(),
        machine: "box".to_string(),
        hostname: "box".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    }
}

fn guard_resource(check: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("box".to_string()),
        command: Some("echo 'the guard is violated' >&2; exit 1".to_string()),
        completion_check: Some(check.to_string()),
        ..Default::default()
    }
}

fn config_with(id: &str, resource: Resource) -> IndexMap<String, Resource> {
    let mut resources = IndexMap::new();
    resources.insert(id.to_string(), resource);
    resources
}

#[test]
fn a_failing_completion_check_is_drift_even_with_nothing_observed() {
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("never-created");
    let lock = lock_with("runner-registered", seeded_task_entry());
    let resources = config_with(
        "runner-registered",
        guard_resource(&format!("[ -f '{}' ]", absent.display())),
    );

    let findings = detect_drift_full(&lock, &local_machine(), &resources);

    assert_eq!(
        findings.len(),
        1,
        "a converged task whose completion_check FAILS on the box is drift; got {findings:?}"
    );
    assert_eq!(findings[0].resource_id, "runner-registered");
}

#[test]
fn a_passing_completion_check_is_not_drift() {
    let dir = tempfile::tempdir().unwrap();
    let present = dir.path().join("marker");
    std::fs::write(&present, "x").unwrap();
    let lock = lock_with("runner-registered", seeded_task_entry());
    let resources = config_with(
        "runner-registered",
        guard_resource(&format!("[ -f '{}' ]", present.display())),
    );

    let findings = detect_drift_full(&lock, &local_machine(), &resources);

    assert!(
        findings.is_empty(),
        "a satisfied guard is not drift; got {findings:?}"
    );
}

#[test]
fn one_violated_guard_is_one_finding_not_two() {
    // The task ALSO carries an observed hash, so the state-query detector would
    // report it too — `task::state_query_script` is the very same check. One
    // violation must not read as two drifted resources.
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("never-created");
    let mut entry = seeded_task_entry();
    entry.set_observed_state("blake3:whatever-was-recorded");
    let lock = lock_with("runner-registered", entry);
    let resources = config_with(
        "runner-registered",
        guard_resource(&format!("[ -f '{}' ]", absent.display())),
    );

    let findings = detect_drift_full(&lock, &local_machine(), &resources);

    assert_eq!(findings.len(), 1, "expected one finding, got {findings:?}");
    assert!(
        findings[0].detail.contains("completion_check fails"),
        "the finding must name the assertion that failed, not a digest: {}",
        findings[0].detail
    );
}

#[test]
fn no_task_checks_declines_the_work_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let absent = dir.path().join("never-created");
    let lock = lock_with("runner-registered", seeded_task_entry());
    let resources = config_with(
        "runner-registered",
        guard_resource(&format!("[ -f '{}' ]", absent.display())),
    );

    let report = detect_drift_full_reported(
        &lock,
        &local_machine(),
        &resources,
        DriftOptions {
            run_task_checks: false,
        },
    );

    assert!(
        report.findings.is_empty(),
        "--no-task-checks must not execute the check"
    );
    // AND THE OPT-OUT MUST BE VISIBLE. A flag that silently shrinks the
    // population is the same defect as the blindness it is opting out of.
    assert_eq!(report.census.inspected_total(), 0);
    assert_eq!(
        report.census.skipped_by_reason().get("--no-task-checks"),
        Some(&1),
        "the census must say the check was declined: {:?}",
        report.census.skipped_by_reason()
    );
}

#[test]
fn the_census_counts_what_was_never_looked_at() {
    // One task that IS inspected, one package the lock never observed, and one
    // resource declared for this machine that the lock has never heard of —
    // the gx10 shape that made `No drift detected.` a lie.
    let dir = tempfile::tempdir().unwrap();
    let present = dir.path().join("marker");
    std::fs::write(&present, "x").unwrap();

    let mut lock = lock_with("runner-registered", seeded_task_entry());
    lock.resources.insert(
        "build-deps".to_string(),
        ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            observed: None,
            details: HashMap::new(),
        },
    );

    let mut resources = config_with(
        "runner-registered",
        guard_resource(&format!("[ -f '{}' ]", present.display())),
    );
    resources.insert(
        "build-deps".to_string(),
        Resource {
            resource_type: ResourceType::Package,
            machine: MachineTarget::Single("box".to_string()),
            packages: vec!["ripgrep".to_string()],
            ..Default::default()
        },
    );
    resources.insert(
        "never-applied-here".to_string(),
        Resource {
            resource_type: ResourceType::Task,
            machine: MachineTarget::Single("box".to_string()),
            command: Some("true".to_string()),
            ..Default::default()
        },
    );

    let report =
        detect_drift_full_reported(&lock, &local_machine(), &resources, DriftOptions::default());

    assert!(
        report.findings.is_empty(),
        "nothing here should have drifted"
    );
    assert_eq!(report.census.in_scope(), 3);
    assert_eq!(report.census.inspected_total(), 1);
    assert_eq!(report.census.skipped_total(), 2);
    assert_eq!(
        report.census.inspected_by_type().get("task"),
        Some(&1),
        "the task's completion_check was executed"
    );
    let skipped = report.census.skipped_by_reason();
    assert_eq!(skipped.get("no observed state in the lock"), Some(&1));
    assert_eq!(skipped.get("declared here, absent from the lock"), Some(&1));
    // The summary an operator reads must carry both numbers.
    let summary = report.census.summary_lines().join(" | ");
    assert!(
        summary.contains("inspected 1 of 3") && summary.contains("skipped 2"),
        "summary hides the denominator: {summary}"
    );
    // And so must the machine-readable report.
    let json = report.census.to_json();
    assert_eq!(json["in_scope"], 3);
    assert_eq!(json["inspected"], 1);
    assert_eq!(json["skipped"], 2);
}

#[test]
fn without_a_config_every_non_file_resource_is_reported_as_uninspected() {
    // `detect_drift`/`detect_drift_with_machine` compare file hashes and
    // nothing else. That is a real limit; the census makes it a stated one
    // instead of a clean bill of health over resources nobody looked at.
    let lock = lock_with("runner-registered", seeded_task_entry());

    let report = detect_drift_reported(&lock, Some(&local_machine()));

    assert_eq!(report.census.inspected_total(), 0);
    assert_eq!(
        report
            .census
            .skipped_by_reason()
            .get("no config loaded (file hashes only)"),
        Some(&1)
    );
}
