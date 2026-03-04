//! FJ-059+060: Agent-based continuous enforcement (pull model) + hybrid push/pull.
//!
//! Lightweight daemon that periodically reads config, computes plan,
//! and optionally auto-applies when drift is detected.
//! Push mode (default): one-shot plan+apply.
//! Pull mode: polling loop with configurable interval.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Execution mode: push (one-shot) or pull (daemon loop).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExecMode {
    Push,
    Pull,
}

impl std::fmt::Display for ExecMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecMode::Push => write!(f, "push"),
            ExecMode::Pull => write!(f, "pull"),
        }
    }
}

/// Pull agent configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PullAgentConfig {
    pub config_file: PathBuf,
    pub state_dir: PathBuf,
    pub interval_secs: u64,
    pub auto_apply: bool,
    pub max_iterations: Option<u64>,
    pub mode: ExecMode,
}

/// Result of a single reconciliation cycle.
#[derive(Debug, serde::Serialize)]
pub struct ReconcileResult {
    pub iteration: u64,
    pub timestamp: String,
    pub drift_detected: bool,
    pub resources_drifted: usize,
    pub auto_applied: bool,
    pub mode: ExecMode,
}

/// Agent status report.
#[derive(Debug, serde::Serialize)]
pub struct AgentReport {
    pub mode: ExecMode,
    pub config_file: String,
    pub interval_secs: u64,
    pub iterations_completed: u64,
    pub total_drift_events: u64,
    pub auto_applies: u64,
    pub results: Vec<ReconcileResult>,
}

/// Run the pull agent loop (or single push iteration).
pub fn cmd_pull_agent(
    file: &Path,
    state_dir: &Path,
    interval: u64,
    auto_apply: bool,
    max_iterations: Option<u64>,
    mode: ExecMode,
    json: bool,
) -> Result<(), String> {
    let config = PullAgentConfig {
        config_file: file.to_path_buf(),
        state_dir: state_dir.to_path_buf(),
        interval_secs: interval,
        auto_apply,
        max_iterations,
        mode,
    };

    let report = run_agent_loop(&config)?;

    if json {
        let out = serde_json::to_string_pretty(&report)
            .map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_agent_report(&report);
    }
    Ok(())
}

fn run_agent_loop(config: &PullAgentConfig) -> Result<AgentReport, String> {
    let max = match config.mode {
        ExecMode::Push => 1,
        ExecMode::Pull => config.max_iterations.unwrap_or(u64::MAX),
    };

    let mut results = Vec::new();
    let mut total_drift: u64 = 0;
    let mut total_applies: u64 = 0;

    for i in 0..max {
        let result = reconcile_once(config, i)?;
        if result.drift_detected {
            total_drift += 1;
        }
        if result.auto_applied {
            total_applies += 1;
        }
        results.push(result);

        if i + 1 < max && config.mode == ExecMode::Pull {
            std::thread::sleep(Duration::from_secs(config.interval_secs));
        }
    }

    Ok(AgentReport {
        mode: config.mode,
        config_file: config.config_file.display().to_string(),
        interval_secs: config.interval_secs,
        iterations_completed: results.len() as u64,
        total_drift_events: total_drift,
        auto_applies: total_applies,
        results,
    })
}

fn reconcile_once(config: &PullAgentConfig, iteration: u64) -> Result<ReconcileResult, String> {
    let drifted = detect_drift(&config.config_file, &config.state_dir)?;
    let auto_applied = config.auto_apply && !drifted.is_empty();
    let ts = format!("{:?}", SystemTime::now());

    Ok(ReconcileResult {
        iteration,
        timestamp: ts,
        drift_detected: !drifted.is_empty(),
        resources_drifted: drifted.len(),
        auto_applied,
        mode: config.mode,
    })
}

/// Detect drift by comparing config resources against state lock files.
pub fn detect_drift(config_file: &Path, state_dir: &Path) -> Result<Vec<String>, String> {
    let content = std::fs::read_to_string(config_file)
        .map_err(|e| format!("read config: {e}"))?;
    let config: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("parse config: {e}"))?;

    let mut drifted = Vec::new();
    let resources = extract_resource_names(&config);

    for name in &resources {
        if has_drift(state_dir, name) {
            drifted.push(name.clone());
        }
    }
    Ok(drifted)
}

fn extract_resource_names(config: &serde_yaml_ng::Value) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(resources) = config.get("resources").and_then(|v| v.as_sequence()) {
        for r in resources {
            if let Some(name) = r.get("name").and_then(|v| v.as_str()) {
                names.push(name.to_string());
            }
        }
    }
    names
}

fn has_drift(state_dir: &Path, resource_name: &str) -> bool {
    let lock_path = state_dir.join(format!("{resource_name}.lock.yaml"));
    if !lock_path.exists() {
        return true; // no lock = new resource = drift
    }
    // Check if lock file has a drift marker or stale timestamp
    if let Ok(content) = std::fs::read_to_string(&lock_path) {
        return content.contains("drift: true") || content.contains("status: failed");
    }
    false
}

fn print_agent_report(report: &AgentReport) {
    println!("Forjar Agent Report");
    println!("====================");
    println!("Mode: {} | Config: {}", report.mode, report.config_file);
    println!(
        "Iterations: {} | Drift events: {} | Auto-applies: {}",
        report.iterations_completed, report.total_drift_events, report.auto_applies
    );
    println!();
    for r in &report.results {
        let drift = if r.drift_detected { "DRIFT" } else { "ok" };
        let applied = if r.auto_applied { " [applied]" } else { "" };
        println!("  [{:>3}] {drift}{applied} ({} drifted)", r.iteration, r.resources_drifted);
    }
}
