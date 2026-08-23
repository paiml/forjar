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
#[allow(clippy::too_many_arguments)]
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
        let out = serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
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
    let ts = format!("{:?}", SystemTime::now());

    // ACTUALLY REMEDIATE.
    //
    // This was `let auto_applied = config.auto_apply && !drifted.is_empty();`
    // — a boolean derived from the FLAG and the drift count, reported as
    // though it described an action. Nothing was ever applied. So
    // `agent --auto-apply` against real drift printed a report saying it had
    // auto-applied, exited 0, and left the drifted file exactly as it found
    // it. The field named `auto_applied` recorded the INTENT, not the effect.
    // Ledger id agent-blind-to-drift-autoapply-never-fires, confirmed at
    // 1.12.3 and still live at 1.16.0.
    //
    // Reuses drift's own remediation path rather than a second implementation,
    // so the two cannot diverge in what "remediate" means.
    let auto_applied = if config.auto_apply && !drifted.is_empty() {
        super::drift::run_drift_remediation(
            &config.config_file,
            &config.state_dir,
            None, // every machine
            drifted.len(),
            false, // remediation output is text, matching drift's own choice
            false, // not verbose
        )?;
        true
    } else {
        false
    };

    Ok(ReconcileResult {
        iteration,
        timestamp: ts,
        drift_detected: !drifted.is_empty(),
        resources_drifted: drifted.len(),
        auto_applied,
        mode: config.mode,
    })
}

/// Detect drift using forjar's REAL drift detector.
///
/// WHAT THIS REPLACED, AND WHY IT NEVER WORKED. The agent carried its own
/// parallel implementation:
///
///   fn has_drift(state_dir, resource_name) -> bool {
///       let lock_path = state_dir.join(format!("{resource_name}.lock.yaml"));
///       ...
///       content.contains("drift: true") || content.contains("status: failed")
///   }
///
/// Locks do not live at `state_dir/<resource>.lock.yaml` — they live at
/// `state_dir/<machine>/state.lock.yaml` — and no lock file has ever contained
/// the string `drift: true`. It parsed the config with a raw YAML walk instead
/// of the real parser, then string-matched a file that does not exist. So the
/// agent reported "Drift events: 0" against a file that had visibly changed,
/// and `--auto-apply` had nothing to act on. Ledger id
/// agent-blind-to-drift-autoapply-never-fires, confirmed at 1.12.3 and still
/// live at 1.16.0.
///
/// A second, private notion of "drift" is the defect, not the string matching:
/// it can only ever disagree with `forjar drift`, and it did — completely. This
/// now calls `tripwire::drift::detect_drift_full`, the same function `forjar
/// drift` uses, so the agent and the drift command cannot diverge on what drift
/// means.
pub fn detect_drift(config_file: &Path, state_dir: &Path) -> Result<Vec<String>, String> {
    let config = crate::core::parser::parse_and_validate(config_file)?;
    let mut drifted = Vec::new();

    for (machine_name, machine) in &config.machines {
        let Some(lock) = crate::core::state::load_lock(state_dir, machine_name)? else {
            // No lock for this machine: it has never been applied. That is not
            // drift — there is no recorded state to have drifted FROM — and
            // reporting it as drift would make `--auto-apply` re-apply the
            // whole stack on every first run.
            continue;
        };
        for finding in crate::tripwire::drift::detect_drift_full(&lock, machine, &config.resources)
        {
            drifted.push(format!("{machine_name}/{}", finding.resource_id));
        }
    }

    Ok(drifted)
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
        println!(
            "  [{:>3}] {drift}{applied} ({} drifted)",
            r.iteration, r.resources_drifted
        );
    }
}
