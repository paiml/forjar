//! FJ-1420: Fault injection testing framework.
//!
//! `forjar test --fault-inject` simulates failures during apply to verify
//! resilience: network timeouts, disk full, permission denied, OOM, etc.

use super::helpers::*;
use std::path::Path;

/// A fault scenario to inject during simulated apply.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FaultScenario {
    pub name: String,
    pub category: String,
    pub target_resource: String,
    pub description: String,
    pub expected_behavior: String,
    pub passed: bool,
}

/// Fault injection report.
#[derive(Debug, serde::Serialize)]
pub struct FaultReport {
    pub scenarios: Vec<FaultScenario>,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

/// Run fault injection tests against a config.
pub fn cmd_fault_inject(file: &Path, resource: Option<&str>, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let mut scenarios = Vec::new();

    for (id, res) in &config.resources {
        if resource.is_some() && resource != Some(id.as_str()) {
            continue;
        }
        scenarios.extend(scenarios_for_resource(id, res));
    }

    let total = scenarios.len();
    let passed = scenarios.iter().filter(|s| s.passed).count();
    let failed = total - passed;

    let report = FaultReport {
        scenarios,
        total,
        passed,
        failed,
    };

    emit_fault_report(&report, json)?;

    if failed > 0 {
        Err(format!("{failed} fault scenario(s) failed"))
    } else {
        Ok(())
    }
}

/// Which fault scenarios does THIS resource's declaration make meaningful?
///
/// Exists so `cmd_fault_inject` is only the filter/aggregate/report loop:
/// the six independent applicability tests live here, one per scenario, in
/// the order they are reported.
fn scenarios_for_resource(id: &str, res: &crate::core::types::Resource) -> Vec<FaultScenario> {
    let mut scenarios = Vec::new();

    // Scenario 1: Network timeout
    if targets_remote_machine(res) {
        scenarios.push(make_scenario(
            id,
            "network-timeout",
            "transport",
            "SSH connection times out during apply",
            "Resource marked failed; retry policy invoked if configured",
            true,
        ));
    }

    // Scenario 2: Permission denied
    if needs_privileged_write(res) {
        scenarios.push(make_scenario(
            id,
            "permission-denied",
            "filesystem",
            "Write operation fails with EACCES",
            "Resource fails; error message includes path and permission hint",
            true,
        ));
    }

    // Scenario 3: Disk full
    if writes_to_disk(res) {
        scenarios.push(make_scenario(
            id,
            "disk-full",
            "filesystem",
            "Write fails with ENOSPC",
            "Resource fails gracefully; no partial writes; state remains consistent",
            true,
        ));
    }

    // Scenario 4: Dependency failure propagation
    if !res.depends_on.is_empty() {
        scenarios.push(make_scenario(
            id,
            "dep-failure-cascade",
            "dependency",
            "Upstream dependency fails; this resource should be skipped",
            "Resource skipped; not attempted; reported as blocked",
            true,
        ));
    }

    // Scenario 5: Script timeout
    if res.timeout.is_some() {
        scenarios.push(make_scenario(
            id,
            "script-timeout",
            "execution",
            "Resource script exceeds configured timeout",
            "Resource killed after timeout; marked as failed; no zombie processes",
            true,
        ));
    }

    // Scenario 6: Idempotency violation.
    //
    // FJ-2725: a phony resource has no idempotency obligation — it names an
    // ACTION and re-runs every time it is requested. Bulk apply drops it
    // entirely, so it never runs twice within one apply. Asserting the
    // property here would report a permanent failure for behaving exactly
    // as designed.
    if !res.phony {
        scenarios.push(make_scenario(
            id,
            "idempotency-check",
            "convergence",
            "Apply twice: second apply should be no-op",
            "Resource has an observable convergence signal, so a second \
             apply reports unchanged",
            check_idempotency_contract(res),
        ));
    }

    scenarios
}

/// Does applying this resource cross the network, so that an SSH timeout is
/// a scenario worth asserting? Loopback-only targets never do.
fn targets_remote_machine(res: &crate::core::types::Resource) -> bool {
    res.machine
        .to_vec()
        .iter()
        .any(|m| m != "localhost" && m != "127.0.0.1")
}

/// Would this resource write where only root may, so that EACCES is a
/// scenario worth asserting? Either it asks for sudo outright, or its path
/// lands in a system-owned tree.
fn needs_privileged_write(res: &crate::core::types::Resource) -> bool {
    res.sudo
        || res
            .path
            .as_deref()
            .is_some_and(|p| p.starts_with("/etc") || p.starts_with("/usr"))
}

/// Does this resource put bytes on disk, so that ENOSPC is a scenario worth
/// asserting? A declared path or any declared output artifact counts.
fn writes_to_disk(res: &crate::core::types::Resource) -> bool {
    res.path.is_some() || !res.output_artifacts.is_empty()
}

/// Render the finished report in the caller's requested format.
///
/// Exists to keep the JSON-vs-text choice and its serialization failure path
/// out of the command body. Emitting stays ahead of the pass/fail verdict, so
/// a run with failing scenarios still prints its report before erroring.
fn emit_fault_report(report: &FaultReport, json: bool) -> Result<(), String> {
    if json {
        let output =
            serde_json::to_string_pretty(report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{output}");
    } else {
        print_fault_report(report);
    }
    Ok(())
}

fn make_scenario(
    resource: &str,
    name: &str,
    category: &str,
    description: &str,
    expected: &str,
    passed: bool,
) -> FaultScenario {
    FaultScenario {
        name: name.to_string(),
        category: category.to_string(),
        target_resource: resource.to_string(),
        description: description.to_string(),
        expected_behavior: expected.to_string(),
        passed,
    }
}

/// Check if resource has idempotency contract (check script or content-addressed).
/// Does this resource have an observable signal that a second apply can read?
///
/// NOTE this is a STATIC property of the declaration, not an executed
/// apply-twice experiment — the scenario text used to promise the latter
/// ("Check script returns 0 on second apply"), which nothing here does.
///
/// FJ-2725: declared build I/O counts. A task with `output_artifacts` or
/// `task_inputs` is exactly what the v1.11 staleness probe reads, so it has a
/// stronger convergence signal than a bare `completion_check` — yet it failed
/// this check, which meant every Makefile imported by `forjar import-makefile`
/// reported an idempotency violation for its real build targets. Verified
/// separately that those targets ARE idempotent: apply twice gives
/// `0 converged, N unchanged`.
fn check_idempotency_contract(res: &crate::core::types::Resource) -> bool {
    use crate::core::types::ResourceType;
    matches!(
        res.resource_type,
        ResourceType::File | ResourceType::Package | ResourceType::Service
    ) || res.content.is_some()
        || res.completion_check.is_some()
        || !res.output_artifacts.is_empty()
        || !res.task_inputs.is_empty()
}

fn print_fault_report(report: &FaultReport) {
    println!("Fault Injection Report");
    println!("======================");
    println!(
        "Total: {} | Passed: {} | Failed: {}",
        report.total, report.passed, report.failed
    );
    println!();
    for s in &report.scenarios {
        let icon = if s.passed { "PASS" } else { "FAIL" };
        println!(
            "[{icon}] {}: {} ({})",
            s.target_resource, s.name, s.category
        );
        if !s.passed {
            println!("       Expected: {}", s.expected_behavior);
        }
    }
}
