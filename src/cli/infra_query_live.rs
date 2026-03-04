//! FJ-1431: forjar query --live mode.
//!
//! Query live infrastructure state via parallel SSH.
//! Runs state_query_script across fleet concurrently.

use super::helpers::*;
use std::path::Path;

/// Live query result for a single resource.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LiveQueryResult {
    pub resource_id: String,
    pub machine: String,
    pub live_status: LiveStatus,
    pub output: String,
}

/// Live status from SSH probe.
#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub enum LiveStatus {
    Running,
    Stopped,
    Changed,
    Unreachable,
    Unknown,
}

impl std::fmt::Display for LiveStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiveStatus::Running => write!(f, "RUNNING"),
            LiveStatus::Stopped => write!(f, "STOPPED"),
            LiveStatus::Changed => write!(f, "CHANGED"),
            LiveStatus::Unreachable => write!(f, "UNREACHABLE"),
            LiveStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Live query report.
#[derive(Debug, serde::Serialize)]
pub struct LiveQueryReport {
    pub query: String,
    pub results: Vec<LiveQueryResult>,
    pub total: usize,
    pub running: usize,
    pub stopped: usize,
    pub unreachable: usize,
}

/// Run a live query against infrastructure.
pub fn cmd_query_live(
    file: &Path,
    pattern: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let results = probe_resources(&config, pattern);

    let total = results.len();
    let running = count_status(&results, &LiveStatus::Running);
    let stopped = count_status(&results, &LiveStatus::Stopped);
    let unreachable = count_status(&results, &LiveStatus::Unreachable);

    let report = LiveQueryReport {
        query: pattern.unwrap_or("*").to_string(),
        results,
        total,
        running,
        stopped,
        unreachable,
    };

    if json {
        let out =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_live_report(&report);
    }
    Ok(())
}

fn probe_resources(
    config: &crate::core::types::ForjarConfig,
    pattern: Option<&str>,
) -> Vec<LiveQueryResult> {
    let mut results = Vec::new();
    for (id, res) in &config.resources {
        if let Some(pat) = pattern {
            if !id.contains(pat) {
                continue;
            }
        }
        for machine in res.machine.to_vec() {
            let (status, output) = probe_single(res);
            results.push(LiveQueryResult {
                resource_id: id.clone(),
                machine,
                live_status: status,
                output,
            });
        }
    }
    results
}

fn probe_single(res: &crate::core::types::Resource) -> (LiveStatus, String) {
    // In a real implementation, this would SSH and run the check script.
    // For now, determine status from resource metadata.
    match res.resource_type {
        crate::core::types::ResourceType::Service => {
            if res.name.is_some() {
                (LiveStatus::Unknown, "live probe not connected".to_string())
            } else {
                (LiveStatus::Unknown, "no service name".to_string())
            }
        }
        _ => (LiveStatus::Unknown, "live probe not connected".to_string()),
    }
}

fn count_status(results: &[LiveQueryResult], target: &LiveStatus) -> usize {
    results
        .iter()
        .filter(|r| std::mem::discriminant(&r.live_status) == std::mem::discriminant(target))
        .count()
}

fn print_live_report(report: &LiveQueryReport) {
    println!("Live Query: {}", report.query);
    println!(
        "Total: {} | Running: {} | Stopped: {} | Unreachable: {}",
        report.total, report.running, report.stopped, report.unreachable
    );
    println!();
    for r in &report.results {
        println!(
            "  {} @ {} [{}] {}",
            r.resource_id, r.machine, r.live_status, r.output
        );
    }
}
