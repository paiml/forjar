//! Apply command dispatch — pre-checks, execution, and post-processing.

use super::helpers::*;
use super::helpers_state::*;
#[allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
#[allow(unused_imports)]
use crate::transport;
#[allow(unused_imports)]
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::Path;

/// FJ-1240: Conditionally encrypt state files after successful apply.
pub(super) fn maybe_encrypt_state(encrypt: bool, result: &Result<(), String>, state_dir: &Path) {
    if encrypt && result.is_ok() {
        if let Err(e) = state::encrypt_state_files(state_dir) {
            eprintln!("warning: state encryption failed: {e}");
        }
    }
}

/// Run a shell script and return error if it fails.
pub(super) fn run_script_check(script: &str) -> Result<(), String> {
    let output = std::process::Command::new("bash")
        .arg("-c")
        .arg(script)
        .output();
    match output {
        Ok(o) if !o.status.success() => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => Err(e.to_string()),
        _ => Ok(()),
    }
}

/// Run a pre-script (file path, not inline).
pub(super) fn run_pre_script(script: &Path) -> Result<(), String> {
    let status = std::process::Command::new("bash")
        .arg(script)
        .status()
        .map_err(|e| format!("Failed to run pre-script: {e}"))?;
    if !status.success() {
        return Err(format!(
            "Pre-script {} exited with code {}",
            script.display(),
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

/// Send a webhook notification (best-effort, log errors).
pub(super) fn send_webhook_before(url: &str, file: &Path) {
    let payload = format!(r#"{{"event":"apply_start","config":"{}"}}"#, file.display());
    match std::process::Command::new("curl")
        .args([
            "-sf",
            "-X",
            "POST",
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload,
            url,
        ])
        .output()
    {
        Ok(o) if !o.status.success() => {
            eprintln!(
                "warning: pre-apply webhook failed (exit {})",
                o.status.code().unwrap_or(-1)
            );
        }
        Err(e) => eprintln!("warning: pre-apply webhook error: {e}"),
        _ => {}
    }
}

/// Run post-flight script (warn on failure, don't abort).
pub(super) fn run_post_flight(script: &str) {
    if let Ok(o) = std::process::Command::new("bash")
        .arg("-c")
        .arg(script)
        .output()
    {
        if !o.status.success() {
            eprintln!(
                "Post-flight warning: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            );
        }
    }
}

/// Check cost limit against planned changes.
pub(super) fn check_cost_limit(
    file: &Path,
    sd: &Path,
    machine: Option<&str>,
    tag: Option<&str>,
    limit: usize,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let order = resolver::build_execution_order(&config)?;
    let locks = load_machine_locks(&config, sd, machine)?;
    let plan = planner::plan(&config, &order, &locks, tag);
    let change_count = plan
        .changes
        .iter()
        .filter(|c| c.action != types::PlanAction::NoOp)
        .count();
    if change_count > limit {
        return Err(format!(
            "Cost limit exceeded: {change_count} changes planned, limit is {limit}. Use --cost-limit {change_count} or higher to proceed."
        ));
    }
    Ok(())
}

/// FJ-2300: Check operator authorization against all machines in config.
pub(super) fn check_operator_auth(file: &Path, operator: Option<&str>) -> Result<(), String> {
    let config = super::helpers::parse_and_validate(file)?;
    let identity = types::OperatorIdentity::resolve(operator);
    for (name, m) in &config.machines {
        if !m.is_operator_allowed(&identity.name) {
            return Err(format!(
                "operator '{}' not authorized for machine '{name}'",
                identity.name
            ));
        }
    }
    Ok(())
}

pub(super) use super::dispatch_apply_b::*;
