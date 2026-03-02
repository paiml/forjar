//! Drift detection.

use super::apply::*;
use super::apply_helpers::*;
use super::helpers::*;
use crate::core::{state, types};
use crate::tripwire::drift;
use std::path::Path;

/// Check one machine for drift, appending findings to all_findings (JSON) or printing text.
fn check_machine_drift(
    name: &str,
    lock: &types::StateLock,
    config: Option<&types::ForjarConfig>,
    json: bool,
    verbose: bool,
    all_findings: &mut Vec<serde_json::Value>,
) -> usize {
    if verbose {
        eprintln!("Checking {} ({} resources)...", name, lock.resources.len());
    }
    if !json {
        println!("Checking {} ({} resources)...", name, lock.resources.len());
    }

    let machine = config.and_then(|c| c.machines.get(name));
    let findings = match (machine, config) {
        (Some(m), Some(cfg)) => drift::detect_drift_full(lock, m, &cfg.resources),
        (Some(m), None) => drift::detect_drift_with_machine(lock, m),
        _ => drift::detect_drift(lock),
    };

    if findings.is_empty() {
        if !json {
            println!("  No drift detected.");
        }
        return 0;
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
    findings.len()
}

/// Print drift summary (JSON or text).
fn print_drift_summary(
    machines_checked: u32,
    total_drift: usize,
    all_findings: &[serde_json::Value],
    json: bool,
) -> Result<(), String> {
    if json {
        let report = serde_json::json!({
            "machines_checked": machines_checked,
            "drift_count": total_drift,
            "findings": all_findings,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {}", e))?;
        println!("{}", output);
    } else if total_drift > 0 {
        println!();
        println!(
            "{}",
            red(&format!("Drift detected: {} resource(s)", total_drift))
        );
    } else {
        println!("{}", green("No drift detected."));
    }
    Ok(())
}

/// Run the alert command when drift is detected.
fn run_drift_alert(alert_cmd: &str, total_drift: usize) -> Result<(), String> {
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(alert_cmd)
        .env("FORJAR_DRIFT_COUNT", total_drift.to_string())
        .status()
        .map_err(|e| format!("alert-cmd failed to execute: {}", e))?;
    if !status.success() {
        eprintln!("alert-cmd exited with code {}", status.code().unwrap_or(-1));
    }
    Ok(())
}

/// Auto-remediate drifted resources by re-applying.
fn run_drift_remediation(
    config_path: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    total_drift: usize,
    json: bool,
    verbose: bool,
) -> Result<(), String> {
    if !json {
        println!();
        println!("Auto-remediating {} drifted resource(s)...", total_drift);
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
    )?;
    if !json {
        println!("Remediation complete.");
    }
    Ok(())
}

/// Send drift notification if configured.
fn send_drift_notification(
    config: &types::ForjarConfig,
    total_drift: usize,
    machine_filter: Option<&str>,
) {
    if let Some(ref cmd) = config.policy.notify.on_drift {
        let drift_str = total_drift.to_string();
        let machine_str = machine_filter.unwrap_or("all");
        run_notify(
            cmd,
            &[("machine", machine_str), ("drift_count", &drift_str)],
        );
    }
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

/// Iterate state dir machines and check each for drift.
fn scan_machines_for_drift(
    state_dir: &Path,
    machine_filter: Option<&str>,
    config: Option<&types::ForjarConfig>,
    json: bool,
    verbose: bool,
) -> Result<(u32, usize, Vec<serde_json::Value>), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;
    let mut total_drift = 0;
    let mut machines_checked = 0u32;
    let mut all_findings: Vec<serde_json::Value> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            machines_checked += 1;
            total_drift +=
                check_machine_drift(&name, &lock, config, json, verbose, &mut all_findings);
        }
    }
    Ok((machines_checked, total_drift, all_findings))
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
) -> Result<(), String> {
    let config = load_drift_config(config_path, env_file)?;

    if dry_run {
        return cmd_drift_dry_run(state_dir, machine_filter, json);
    }

    if let Some(ref cfg) = config {
        for (_, machine) in &cfg.machines {
            if machine.is_container_transport() {
                crate::transport::container::ensure_container(machine)?;
            }
        }
    }

    let (machines_checked, total_drift, all_findings) =
        scan_machines_for_drift(state_dir, machine_filter, config.as_ref(), json, verbose)?;

    print_drift_summary(machines_checked, total_drift, &all_findings, json)?;

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
        return Err(format!("{} drift finding(s)", total_drift));
    }

    Ok(())
}

/// Dry-run mode for drift: lists resources that would be checked without connecting.
pub(crate) fn cmd_drift_dry_run(
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut checks: Vec<serde_json::Value> = Vec::new();
    let mut total = 0usize;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(lock) = state::load_lock(state_dir, &name)? {
            if !json {
                println!("Machine: {} ({} resources)", name, lock.resources.len());
            }
            for (res_id, res_state) in &lock.resources {
                total += 1;
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
        }
    }

    if json {
        let report = serde_json::json!({
            "dry_run": true,
            "total_checks": total,
            "checks": checks,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {}", e))?;
        println!("{}", output);
    } else {
        println!();
        println!("Dry run: {} resource(s) would be checked", total);
    }

    Ok(())
}
