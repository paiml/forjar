//! Drift detection.

use super::apply::*;
use super::apply_helpers::*;
use super::colors::{red, yellow};
use super::drift_lockless::{dry_run_lockless, scan_lockless};
use super::drift_report::{
    census_json, print_drift_summary, render_finding, run_drift_alert, send_drift_notification,
};
use super::drift_state::{collect_machine_locks, machine_state_dirs, refuse_out_of_scope};
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

/// Check one machine for drift, accumulating into `acc` (JSON rows) or printing text.
///
/// Accumulates the drift count AND the census — forjar#380. Returning the count
/// alone is what let `No drift detected.` stand in for both "checked 62
/// resources, all clean" and "checked none of them".
fn check_machine_drift(
    name: &str,
    lock: &types::StateLock,
    config: Option<&types::ForjarConfig>,
    acc: &mut DriftScan,
    scan: ScanOptions,
) {
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
    report_machine_findings(name, report, acc, scan);
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
    acc: &mut DriftScan,
    scan: ScanOptions,
) {
    let ScanOptions { json, .. } = scan;
    let drift::DriftReport { findings, census } = report;
    // forjar#549: a query the target never answered is its own verdict — not a
    // drift finding and not a clean one — so it is counted, rendered and gated
    // apart from both.
    let (unmeasured, drifted): (Vec<_>, Vec<_>) =
        findings.into_iter().partition(|f| f.is_unmeasured());

    // THE DENOMINATOR PRINTS EVERY TIME, drift or no drift. It is worth least
    // when there IS drift (the findings speak for themselves) and most when
    // there is none, which is exactly why it cannot be conditional on findings.
    if !json {
        for line in census.summary_lines() {
            println!("  {line}");
        }
    }
    acc.censuses.push(census_json(name, &census));
    acc.total_drift += drifted.len();
    acc.total_unmeasured += unmeasured.len();

    if drifted.is_empty() && unmeasured.is_empty() {
        if !json {
            println!("  No drift detected.");
        }
        return;
    }
    for f in &drifted {
        render_finding(name, f, red("DRIFTED"), json, &mut acc.findings);
    }
    for f in &unmeasured {
        render_finding(name, f, yellow("UNMEASURED"), json, &mut acc.unmeasured);
    }
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
    scope: Option<&[String]>,
    scan_opts: ScanOptions,
) -> Result<DriftScan, String> {
    let Some(machine_locks) = collect_machine_locks(state_dir, machine_filter, scope)? else {
        return scan_lockless(state_dir, machine_filter, config, scan_opts);
    };

    if machine_locks.len() <= 1 {
        return scan_sequential(&machine_locks, config, scan_opts);
    }

    // Parallel: check each machine in its own thread
    let parts: Vec<DriftScan> = std::thread::scope(|s| {
        let handles: Vec<_> = machine_locks
            .iter()
            .map(|(name, lock)| {
                s.spawn(move || {
                    let mut part = DriftScan::default();
                    check_machine_drift(name, lock, config, &mut part, scan_opts);
                    part
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });

    let mut scan = DriftScan {
        machines_checked: parts.len() as u32,
        ..Default::default()
    };
    for part in parts {
        scan.absorb(part);
    }
    Ok(scan)
}

/// One `forjar drift` run: the verdict AND the population it was drawn from.
#[derive(Default)]
pub(super) struct DriftScan {
    pub(super) machines_checked: u32,
    pub(super) total_drift: usize,
    pub(super) findings: Vec<serde_json::Value>,
    /// forjar#549: resources whose target never answered. Counted in both
    /// output modes; rows are collected under `--json` only, like `findings`.
    pub(super) total_unmeasured: usize,
    pub(super) unmeasured: Vec<serde_json::Value>,
    pub(super) censuses: Vec<serde_json::Value>,
}

impl DriftScan {
    /// Fold one machine's scan into the run's.
    fn absorb(&mut self, mut part: DriftScan) {
        self.total_drift += part.total_drift;
        self.total_unmeasured += part.total_unmeasured;
        self.findings.append(&mut part.findings);
        self.unmeasured.append(&mut part.unmeasured);
        self.censuses.append(&mut part.censuses);
    }
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
        check_machine_drift(name, lock, config, &mut scan, scan_opts);
    }
    Ok(scan)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_drift(
    config_path: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    // PMAT-562 (forjar#562, paiml/infra#605): accepted and ignored. The verdict
    // reaches the exit code on EVERY run now; `--tripwire` stays parseable so
    // no cron line on the fleet breaks, and it changes nothing.
    _tripwire_compat: bool,
    alert_cmd: Option<&str>,
    auto_remediate: bool,
    dry_run: bool,
    json: bool,
    verbose: bool,
    env_file: Option<&Path>,
    all_stacks: bool,
    no_task_checks: bool,
) -> Result<(), String> {
    let config = load_drift_config(config_path, env_file)?;

    // forjar#488: WHAT THIS RUN IS ABOUT.
    //
    // `Some(names)` is "the machines this config declares" and is the default
    // whenever a config was loaded. `None` is the whole state dir, which is now
    // reached only by asking for it (`--all-stacks`) or by having no config to
    // scope with — never by pointing `-f` at one machine and being answered
    // about thirty others.
    let scope: Option<Vec<String>> = match (&config, all_stacks) {
        (Some(cfg), false) => Some(cfg.machines.keys().cloned().collect()),
        _ => None,
    };
    let scope_ref = scope.as_deref();

    if dry_run {
        return cmd_drift_dry_run(
            config.as_ref(),
            state_dir,
            machine_filter,
            scope_ref,
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
    let scan = scan_machines_for_drift(
        state_dir,
        machine_filter,
        config.as_ref(),
        scope_ref,
        scan_opts,
    )?;
    print_drift_summary(&scan, json)?;
    // PMAT-564: a run that inspected NONE of what it was asked about declines
    // before any verdict is drawn from it.
    if let Some(cfg) = config.as_ref().filter(|_| !all_stacks) {
        decline_on_empty_scope(cfg, &scan)?;
    }
    let DriftScan {
        total_drift,
        total_unmeasured,
        ..
    } = scan;

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

    // PMAT-562: the verdict IS the exit code, with no flag. `Drift detected:
    // 2 resource(s)` followed by rc=0 was measured on yoga and gx10 (paiml/
    // infra#605): the default was fail-open and `--tripwire` was the way to
    // ask for the truth — the forjar#352 class, where the flag that makes the
    // tool honest is opt-in. A drift finding is a reject (1); a remediated
    // finding was still a finding.
    if total_drift > 0 {
        return Err(format!("{total_drift} drift finding(s)"));
    }
    // forjar#549: DRIFT WINS when both are present, because a definite finding
    // is the stronger statement. Unmeasured alone is not a pass: it exits with
    // the connection class (4), because the question was never answered.
    if total_unmeasured > 0 {
        return Err(crate::core::error::ForjarError::connection(format!(
            "{}: {total_unmeasured} resource(s)",
            crate::core::error::DRIFT_UNMEASURED_MARKER
        ))
        .into_untyped());
    }

    Ok(())
}

/// PMAT-564 (forjar#564, paiml/infra#605): decline when the run inspected
/// zero of the resources the config declares for the machines it scanned.
///
/// Measured on yoga under 1.30.0: `drift -f forjar-ephemeral.yaml` inspected
/// 0 of the 10 resources that manifest declares, graded two files from a
/// manifest it was not given, printed `Drift detected: 2 resource(s)` and
/// exited 0. Zero coverage of the question is not an answer to it — the
/// forjar#488 shape and the fleet's "0 violations over 0 files" signature.
/// Exit 2 is `decline`; the message names the count and, when every declared
/// resource was skipped for want of a lock entry, says so.
fn decline_on_empty_scope(cfg: &types::ForjarConfig, scan: &DriftScan) -> Result<(), String> {
    let scanned: Vec<&str> = scan
        .censuses
        .iter()
        .filter_map(|c| c["machine"].as_str())
        .collect();
    let declared = cfg
        .resources
        .iter()
        .filter(|(_, r)| r.resource_type != types::ResourceType::Recipe)
        .filter(|(_, r)| r.machine.iter().any(|m| scanned.contains(&m)))
        .count();
    let inspected: u64 = scan
        .censuses
        .iter()
        .filter_map(|c| c["inspected"].as_u64())
        .sum();
    // forjar#549: an UNMEASURED resource was asked about and did not answer.
    // That is its own verdict (exit 4), and it outranks a decline: the
    // question was put. A decline is for a run that put no question at all.
    if declared == 0 || inspected > 0 || scan.total_unmeasured > 0 {
        return Ok(());
    }
    let reasons: std::collections::BTreeSet<String> = scan
        .censuses
        .iter()
        .filter_map(|c| c["skipped_by_reason"].as_object())
        .flat_map(|o| o.keys().cloned())
        .filter(|k| k != "in the lock, not in the config")
        .collect();
    let lock_only = !reasons.is_empty()
        && reasons.iter().all(|k| {
            k.starts_with("declared here, absent from the lock") || k.starts_with("no lock")
        });
    let why = if lock_only {
        "no lock holds them".to_string()
    } else {
        format!(
            "skipped: {}",
            reasons.into_iter().collect::<Vec<_>>().join(", ")
        )
    };
    Err(crate::core::error::ForjarError::partial(format!(
        "{} {declared} declared; {why}",
        crate::core::error::DRIFT_DECLINED_MARKER
    ))
    .into_untyped())
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
    scope: Option<&[String]>,
    json: bool,
    no_task_checks: bool,
) -> Result<(), String> {
    // forjar#488: THE PREVIEW REFUSES WHAT THE RUN REFUSES.
    //
    // The scope guard lived in `collect_machine_locks`, which the preview does
    // not go through — it calls `machine_state_dirs` directly. So
    // `drift --dry-run -m <undeclared>` scanned zero machines and printed
    // "0 resource(s) would be checked", exit 0: the same false green the guard
    // exists to prevent, in the command an operator reaches for FIRST when
    // they are unsure. Found by two of three review lanes independently.
    refuse_out_of_scope(machine_filter, scope)?;
    let Some(names) = machine_state_dirs(state_dir, machine_filter, scope)? else {
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
