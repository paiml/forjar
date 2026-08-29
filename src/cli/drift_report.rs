//! How a completed drift scan is REPORTED.
//!
//! Everything here consumes a finished scan and tells someone about it — stdout
//! for a human, `--json` for a machine reader, `--alert-cmd` and
//! `policy.notify.on_drift` for whoever is watching. Nothing here changes the
//! host: remediation (which does) stays with the scan in `drift.rs`, because
//! "describe what was found" and "go change it" are different authorities and
//! the census bug (forjar#380) lived entirely in the first one.

use super::apply_helpers::run_notify;
use super::colors::{green, red};
use crate::core::types;
use crate::tripwire::drift;

/// One machine's census, tagged with its name, for the `--json` report.
pub(super) fn census_json(name: &str, census: &drift::DriftCensus) -> serde_json::Value {
    let mut value = census.to_json();
    if let Some(obj) = value.as_object_mut() {
        obj.insert("machine".to_string(), serde_json::json!(name));
    }
    value
}

/// Print drift summary (JSON or text).
pub(super) fn print_drift_summary(
    machines_checked: u32,
    total_drift: usize,
    all_findings: &[serde_json::Value],
    censuses: &[serde_json::Value],
    json: bool,
) -> Result<(), String> {
    // A `--json` consumer was as blind as a human reading the text output:
    // `drift_count: 0` over an unstated population. The census ships in both
    // surfaces or the machine-readable one becomes the lying half.
    let inspected: u64 = censuses
        .iter()
        .filter_map(|c| c["inspected"].as_u64())
        .sum();
    let skipped: u64 = censuses.iter().filter_map(|c| c["skipped"].as_u64()).sum();
    if json {
        let report = serde_json::json!({
            "machines_checked": machines_checked,
            "drift_count": total_drift,
            "resources_inspected": inspected,
            "resources_skipped": skipped,
            "census": censuses,
            "findings": all_findings,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{output}");
    } else if total_drift > 0 {
        println!();
        println!(
            "{}",
            red(&format!("Drift detected: {total_drift} resource(s)"))
        );
    } else {
        println!("{}", green("No drift detected."));
        // The verdict and its population on adjacent lines, because the verdict
        // alone is the sentence that has been misread for a year.
        println!("  {inspected} resource(s) inspected, {skipped} not inspected.");
    }
    Ok(())
}

/// Run the alert command when drift is detected.
pub(super) fn run_drift_alert(alert_cmd: &str, total_drift: usize) -> Result<(), String> {
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(alert_cmd)
        .env("FORJAR_DRIFT_COUNT", total_drift.to_string())
        .status()
        .map_err(|e| format!("alert-cmd failed to execute: {e}"))?;
    if !status.success() {
        eprintln!("alert-cmd exited with code {}", status.code().unwrap_or(-1));
    }
    Ok(())
}

/// Send drift notification if configured.
pub(super) fn send_drift_notification(
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
