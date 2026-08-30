//! Drift detection.

use super::apply::*;
use super::apply_helpers::*;
use super::drift_lockless::{dry_run_lockless, scan_lockless};
use super::drift_report::{
    census_json, print_drift_summary, run_drift_alert, send_drift_notification,
};
use super::drift_state::{collect_machine_locks, machine_state_dirs};
use super::helpers::*;
use crate::core::{state, types};
use crate::tripwire::drift;
use std::path::Path;

/// How one drift scan reports itself, and how much work it may do on the host.
#[derive(Clone, Copy)]
pub(super) struct ScanOptions {
    pub(super) json: bool,
    pub(super) verbose: bool,
    pub(super) detect: drift::DriftOptions,
}

/// Check one machine for drift, appending findings to all_findings (JSON) or printing text.
///
/// Returns the drift count AND the census — forjar#380. Returning the count
/// alone is what let `No drift detected.` stand in for both "checked 62
/// resources, all clean" and "checked none of them".
fn check_machine_drift(
    name: &str,
    lock: &types::StateLock,
    config: Option<&types::ForjarConfig>,
    all_findings: &mut Vec<serde_json::Value>,
    scan: ScanOptions,
) -> (usize, drift::DriftCensus) {
    let ScanOptions { detect: opts, .. } = scan;
    print_machine_header(name, &format!("{} resources", lock.resources.len()), scan);

    let machine = config.and_then(|c| c.machines.get(name));
    let report = match (machine, config) {
        (Some(m), Some(cfg)) => {
            // PMAT-197: resources MUST be template-resolved before they are
            // compared against live machine state. Passing raw `cfg.resources`
            // made every `{{params.*}}`-bearing resource report permanent false
            // drift; because the apply-time gate is global, that blocked every
            // targeted apply fleet-wide.
            let resolved = crate::core::resolver::resolve_all(
                &cfg.resources,
                &cfg.params,
                &cfg.machines,
                &cfg.secrets,
            );
            drift::detect_drift_full_reported(lock, m, &resolved, opts)
        }
        (Some(m), None) => drift::detect_drift_reported(lock, Some(m)),
        _ => drift::detect_drift_reported(lock, None),
    };
    report_machine_findings(name, report, all_findings, scan)
}

/// `Checking <machine> (<scope>)...`, before the scan that may take a minute.
///
/// The scope note is what the run is about to look at — `62 resources` from a
/// lock, or `no lock — assertions only` when there is none (forjar#385). It is
/// never a bare count with no provenance, because two very different runs used
/// to print the same header.
pub(super) fn print_machine_header(name: &str, scope: &str, scan: ScanOptions) {
    if scan.verbose {
        eprintln!("Checking {name} ({scope})...");
    }
    if !scan.json {
        println!("Checking {name} ({scope})...");
    }
}

/// Print (or accumulate, under `--json`) one machine's findings and census.
///
/// Shared by the locked and lockless scans so the two cannot disagree about how
/// a verdict is rendered — the census is the thing that keeps a smaller answer
/// honest, and a second copy of this would be free to drop it.
pub(super) fn report_machine_findings(
    name: &str,
    report: drift::DriftReport,
    all_findings: &mut Vec<serde_json::Value>,
    scan: ScanOptions,
) -> (usize, drift::DriftCensus) {
    let ScanOptions { json, .. } = scan;
    let drift::DriftReport { findings, census } = report;

    // THE DENOMINATOR PRINTS EVERY TIME, drift or no drift. It is worth least
    // when there IS drift (the findings speak for themselves) and most when
    // there is none, which is exactly why it cannot be conditional on findings.
    if !json {
        for line in census.summary_lines() {
            println!("  {line}");
        }
    }

    if findings.is_empty() {
        if !json {
            println!("  No drift detected.");
        }
        return (0, census);
    }

    for f in &findings {
        if json {
            all_findings.push(serde_json::json!({
                "machine": name,
                "resource": f.resource_id,
                "detail": f.detail,
                "expected_hash": f.expected_hash,
                "actual_hash": f.actual_hash,
            }));
        } else {
            println!("  {}: {} ({})", red("DRIFTED"), f.resource_id, f.detail);
            println!("    Expected: {}", f.expected_hash);
            println!("    Actual:   {}", f.actual_hash);
        }
    }
    (findings.len(), census)
}

/// Auto-remediate drifted resources by re-applying.
pub(crate) fn run_drift_remediation(
    config_path: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    total_drift: usize,
    json: bool,
    verbose: bool,
) -> Result<(), String> {
    if !json {
        println!();
        println!("Auto-remediating {total_drift} drifted resource(s)...");
    }
    cmd_apply(
        config_path,
        state_dir,
        machine_filter,
        None,  // no resource filter — force re-applies all
        None,  // no tag filter
        None,  // no group filter
        true,  // force
        false, // not dry-run
        false, // tripwire on
        &[],   // no param overrides
        false, // no auto-commit
        None,  // no timeout
        false, // no json (remediation output is text)
        verbose,
        None,  // no env_file
        None,  // no workspace
        false, // no report
        false, // no force_unlock
        None,  // no output mode
        false, // no progress
        false, // no timing
        0,     // no retry
        true,  // yes (skip prompt)
        false,
        None,  // no resource_timeout
        false, // no rollback_on_failure
        None,  // no max_parallel
        None,  // no notify,
        None,  // subset
        false, // confirm_destructive
        None,  // exclude
        false, // sequential
        None,  // telemetry_endpoint
        false, // refresh
        None,  // force_tag
        &[],
    )?;
    if !json {
        println!("Remediation complete.");
    }
    Ok(())
}

/// Load config if the config file exists.
fn load_drift_config(
    config_path: &Path,
    env_file: Option<&Path>,
) -> Result<Option<types::ForjarConfig>, String> {
    if !config_path.exists() {
        return Ok(None);
    }
    let mut cfg = parse_and_validate(config_path)?;
    if let Some(path) = env_file {
        load_env_params(&mut cfg, path)?;
    }
    Ok(Some(cfg))
}

/// FJ-1396: Iterate state dir machines and check each for drift.
///
/// Uses `std::thread::scope` for parallel drift detection across machines.
/// Each machine is checked in its own thread; results are aggregated.
///
/// forjar#385: an ABSENT state dir hands the run to `scan_lockless` instead of
/// killing it. There is no lock to walk, but a `type: task` observable is an
/// ASSERTION rather than a baseline, so the checks can still be executed and
/// the census can still say what went unmeasured. An UNREADABLE state dir is a
/// different fault and `machine_state_dirs` keeps it fatal.
fn scan_machines_for_drift(
    state_dir: &Path,
    machine_filter: Option<&str>,
    config: Option<&types::ForjarConfig>,
    scan_opts: ScanOptions,
) -> Result<DriftScan, String> {
    let Some(machine_locks) = collect_machine_locks(state_dir, machine_filter)? else {
        return scan_lockless(state_dir, machine_filter, config, scan_opts);
    };

    if machine_locks.len() <= 1 {
        return scan_sequential(&machine_locks, config, scan_opts);
    }

    // Parallel: check each machine in its own thread
    let results: Vec<_> = std::thread::scope(|s| {
        let handles: Vec<_> = machine_locks
            .iter()
            .map(|(name, lock)| {
                s.spawn(move || {
                    let mut findings = Vec::new();
                    let (count, census) =
                        check_machine_drift(name, lock, config, &mut findings, scan_opts);
                    (count, findings, census_json(name, &census))
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });

    let mut scan = DriftScan {
        machines_checked: results.len() as u32,
        ..Default::default()
    };
    for (count, mut findings, census) in results {
        scan.total_drift += count;
        scan.findings.append(&mut findings);
        scan.censuses.push(census);
    }
    Ok(scan)
}

/// One `forjar drift` run: the verdict AND the population it was drawn from.
#[derive(Default)]
pub(super) struct DriftScan {
    pub(super) machines_checked: u32,
    pub(super) total_drift: usize,
    pub(super) findings: Vec<serde_json::Value>,
    pub(super) censuses: Vec<serde_json::Value>,
}

/// Sequential scan fallback for 0-1 machines.
fn scan_sequential(
    machine_locks: &[(String, types::StateLock)],
    config: Option<&types::ForjarConfig>,
    scan_opts: ScanOptions,
) -> Result<DriftScan, String> {
    let mut scan = DriftScan {
        machines_checked: machine_locks.len() as u32,
        ..Default::default()
    };
    for (name, lock) in machine_locks {
        let (count, census) =
            check_machine_drift(name, lock, config, &mut scan.findings, scan_opts);
        scan.total_drift += count;
        scan.censuses.push(census_json(name, &census));
    }
    Ok(scan)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_drift(
    config_path: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    tripwire_mode: bool,
    alert_cmd: Option<&str>,
    auto_remediate: bool,
    dry_run: bool,
    json: bool,
    verbose: bool,
    env_file: Option<&Path>,
    no_task_checks: bool,
) -> Result<(), String> {
    let config = load_drift_config(config_path, env_file)?;

    if dry_run {
        return cmd_drift_dry_run(
            config.as_ref(),
            state_dir,
            machine_filter,
            json,
            no_task_checks,
        );
    }

    if let Some(ref cfg) = config {
        for (_, machine) in &cfg.machines {
            if machine.is_container_transport() {
                crate::transport::container::ensure_container(machine)?;
            }
        }
    }

    let scan_opts = ScanOptions {
        json,
        verbose,
        detect: drift::DriftOptions {
            run_task_checks: !no_task_checks,
        },
    };
    let scan = scan_machines_for_drift(state_dir, machine_filter, config.as_ref(), scan_opts)?;
    let DriftScan {
        machines_checked,
        total_drift,
        findings: all_findings,
        censuses,
    } = scan;

    print_drift_summary(
        machines_checked,
        total_drift,
        &all_findings,
        &censuses,
        json,
    )?;

    if total_drift > 0 {
        if let Some(cmd) = alert_cmd {
            run_drift_alert(cmd, total_drift)?;
        }
        if auto_remediate {
            run_drift_remediation(
                config_path,
                state_dir,
                machine_filter,
                total_drift,
                json,
                verbose,
            )?;
        }
        if let Some(ref cfg) = config {
            send_drift_notification(cfg, total_drift, machine_filter);
        }
    }

    if tripwire_mode && total_drift > 0 {
        return Err(format!("{total_drift} drift finding(s)"));
    }

    Ok(())
}

/// Records what a drift check would inspect on one machine: in JSON mode each
/// resource is appended to `checks`, otherwise the machine and its resources are
/// printed. Returns the number of resources accounted for.
fn record_dry_run_checks(
    name: &str,
    lock: &types::StateLock,
    json: bool,
    checks: &mut Vec<serde_json::Value>,
) -> usize {
    if !json {
        println!("Machine: {} ({} resources)", name, lock.resources.len());
    }
    for (res_id, res_state) in &lock.resources {
        if json {
            checks.push(serde_json::json!({
                "machine": name,
                "resource": res_id,
                "status": res_state.status,
                "hash": res_state.hash,
            }));
        } else {
            println!("  would check: {} (status: {})", res_id, res_state.status);
        }
    }
    lock.resources.len()
}

/// Emits the dry-run result: a JSON report, or a human-readable total.
pub(super) fn print_dry_run_report(
    json: bool,
    total: usize,
    checks: &[serde_json::Value],
) -> Result<(), String> {
    if json {
        let report = serde_json::json!({
            "dry_run": true,
            "total_checks": total,
            "checks": checks,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{output}");
    } else {
        println!();
        println!("Dry run: {total} resource(s) would be checked");
    }
    Ok(())
}

/// Dry-run mode for drift: lists resources that would be checked without connecting.
///
/// forjar#385: takes the config, because with an ABSENT state dir the preview
/// has to come from the same place the run does. A preview that dies where the
/// run succeeds is a worse answer than no preview at all.
pub(crate) fn cmd_drift_dry_run(
    config: Option<&types::ForjarConfig>,
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
    no_task_checks: bool,
) -> Result<(), String> {
    let Some(names) = machine_state_dirs(state_dir, machine_filter)? else {
        let opts = drift::DriftOptions {
            run_task_checks: !no_task_checks,
        };
        return dry_run_lockless(state_dir, machine_filter, config, json, opts);
    };
    let mut checks: Vec<serde_json::Value> = Vec::new();
    let mut total = 0usize;

    for name in names {
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            total += record_dry_run_checks(&name, &lock, json, &mut checks);
        }
    }

    print_dry_run_report(json, total, &checks)
}
