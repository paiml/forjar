//! PMAT-564: the decline a drift run owes when it inspected none of what it
//! was asked about. Split out of `drift.rs`, which the function took past the
//! 500-line file-health limit (485 -> 549).

use super::drift::DriftScan;
use crate::core::types;

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
pub(super) fn decline_on_empty_scope(
    cfg: &types::ForjarConfig,
    scan: &DriftScan,
) -> Result<(), String> {
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
