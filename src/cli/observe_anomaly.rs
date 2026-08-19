//! `forjar anomaly` — statistical anomaly detection over the event log.
//!
//! Split out of `observe.rs` to keep both modules small.
//!
//! Dogfood #208 (`anomaly-never-detects-anything-and-min-events-inert`): every
//! detector was population-relative, so a lone resource that failed half its
//! applies scored zero, and `--min-events` was echoed into the summary without
//! filtering the population it claimed to have analysed.

use crate::core::types;
use crate::tripwire::anomaly;
use std::path::Path;

/// Detect anomalous resource behavior from event history.
///
/// Analyzes event logs to find resources with abnormally high change frequency,
/// failure rates, or drift counts. Uses statistical z-score to flag outliers.
pub(crate) fn cmd_anomaly(
    state_dir: &Path,
    machine_filter: Option<&str>,
    min_events: usize,
    json: bool,
) -> Result<(), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    // Per-resource metrics: (converge_count, fail_count, drift_count)
    let mut metrics: std::collections::HashMap<String, (u32, u32, u32)> =
        std::collections::HashMap::new();
    // Dogfood #208: convergence durations per resource, for the outlier detector.
    let mut durations: std::collections::HashMap<String, Vec<f64>> =
        std::collections::HashMap::new();

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

        let log_path = entry.path().join("events.jsonl");
        if !log_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&log_path)
            .map_err(|e| format!("cannot read {}: {}", log_path.display(), e))?;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(te) = serde_json::from_str::<types::TimestampedEvent>(line) {
                match te.event {
                    types::ProvenanceEvent::ResourceConverged {
                        ref resource,
                        duration_seconds,
                        ..
                    } => {
                        let key = format!("{name}:{resource}");
                        let entry = metrics.entry(key.clone()).or_insert((0, 0, 0));
                        entry.0 += 1;
                        durations.entry(key).or_default().push(duration_seconds);
                    }
                    types::ProvenanceEvent::ResourceFailed { ref resource, .. } => {
                        let key = format!("{name}:{resource}");
                        let entry = metrics.entry(key).or_insert((0, 0, 0));
                        entry.1 += 1;
                    }
                    types::ProvenanceEvent::DriftDetected { ref resource, .. } => {
                        let key = format!("{name}:{resource}");
                        let entry = metrics.entry(key).or_insert((0, 0, 0));
                        entry.2 += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    // Convert metrics HashMap to Vec for detect_anomalies()
    let mut metrics_vec: Vec<(String, u32, u32, u32)> = metrics
        .into_iter()
        .map(|(k, (c, f, d))| (k, c, f, d))
        .collect();
    metrics_vec.sort_by(|a, b| a.0.cmp(&b.0));

    // Dogfood #208 (anomaly-never-detects-anything-and-min-events-inert):
    // --min-events was echoed into the summary but the "N resources analyzed"
    // count was the UNFILTERED population, so `--min-events 9999` still claimed
    // to analyze every resource. Compute the analysed set once, here, and
    // report from it.
    let analyzed = analyzed_count(&metrics_vec, min_events);

    // FJ-051: Use anomaly module for detection
    let mut findings = anomaly::detect_anomalies(&metrics_vec, min_events);

    let mut duration_vec: Vec<(String, Vec<f64>)> = durations.into_iter().collect();
    duration_vec.sort_by(|a, b| a.0.cmp(&b.0));
    for finding in anomaly::detect_duration_anomalies(&duration_vec, min_events) {
        merge_finding(&mut findings, finding);
    }
    findings.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.resource.cmp(&b.resource))
    });

    if findings.is_empty() {
        if json {
            println!("{{\"anomalies\":0,\"findings\":[]}}");
        } else {
            let total = analyzed;
            println!(
                "No anomalies detected ({total} resources analyzed, min {min_events} events)."
            );
        }
        return Ok(());
    }

    output_anomaly_findings(&findings, json)?;
    Ok(())
}

/// How many resources actually pass the `--min-events` threshold.
///
/// Dogfood #208: the summary reported the UNFILTERED population, so
/// `anomaly --min-events 9999` still claimed "2 resources analyzed" — the flag
/// looked live while changing nothing an operator could see.
pub(crate) fn analyzed_count(metrics: &[(String, u32, u32, u32)], min_events: usize) -> usize {
    metrics
        .iter()
        .filter(|(_, c, f, d)| (*c + *f + *d) as usize >= min_events)
        .count()
}

/// Fold a finding into the list, merging reasons when the resource is already
/// present so one resource never appears twice.
fn merge_finding(findings: &mut Vec<anomaly::AnomalyFinding>, incoming: anomaly::AnomalyFinding) {
    if let Some(existing) = findings
        .iter_mut()
        .find(|f| f.resource == incoming.resource)
    {
        existing.reasons.extend(incoming.reasons);
        if incoming.score > existing.score {
            existing.score = incoming.score;
            existing.status = incoming.status;
        }
    } else {
        findings.push(incoming);
    }
}

/// Output anomaly findings in JSON or text format.
pub(super) fn output_anomaly_findings(
    findings: &[anomaly::AnomalyFinding],
    json: bool,
) -> Result<(), String> {
    if json {
        let json_findings: Vec<serde_json::Value> = findings
            .iter()
            .map(|f| {
                serde_json::json!({
                    "resource": f.resource,
                    "score": f.score,
                    "status": format!("{:?}", f.status),
                    "reasons": f.reasons,
                })
            })
            .collect();
        let report = serde_json::json!({
            "anomalies": json_findings.len(),
            "findings": json_findings,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{output}");
    } else {
        for finding in findings {
            let status_label = match finding.status {
                anomaly::DriftStatus::Drift => "DRIFT",
                anomaly::DriftStatus::Warning => "WARNING",
                anomaly::DriftStatus::Stable => "STABLE",
            };
            println!(
                "  ANOMALY: {} [{}] (score={:.2}) — {}",
                finding.resource,
                status_label,
                finding.score,
                finding.reasons.join("; ")
            );
        }
        println!();
        println!("Anomaly detection: {} anomaly(ies) found.", findings.len());
    }
    Ok(())
}
