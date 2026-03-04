//! FJ-1436: Saga-pattern multi-stack apply.
//!
//! Each stack apply records a compensating snapshot.
//! On failure, prior stacks revert to their snapshot.
//! Coordinator tracks completion across stacks.

use super::helpers::*;
use std::path::Path;

/// A saga step representing one stack's apply.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SagaStep {
    pub stack_name: String,
    pub config_path: String,
    pub snapshot_path: Option<String>,
    pub status: SagaStepStatus,
    pub error: Option<String>,
}

/// Status of a saga step.
#[derive(Debug, Clone, serde::Serialize)]
pub enum SagaStepStatus {
    Pending,
    Applied,
    Failed,
    Compensated,
}

impl std::fmt::Display for SagaStepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SagaStepStatus::Pending => write!(f, "PENDING"),
            SagaStepStatus::Applied => write!(f, "APPLIED"),
            SagaStepStatus::Failed => write!(f, "FAILED"),
            SagaStepStatus::Compensated => write!(f, "COMPENSATED"),
        }
    }
}

/// Saga execution report.
#[derive(Debug, serde::Serialize)]
pub struct SagaReport {
    pub steps: Vec<SagaStep>,
    pub total: usize,
    pub applied: usize,
    pub failed: usize,
    pub compensated: usize,
    pub success: bool,
}

/// Plan a saga-pattern multi-stack apply.
pub fn cmd_saga_plan(
    files: &[std::path::PathBuf],
    state_dir: &Path,
    json: bool,
) -> Result<(), String> {
    let steps = build_saga_steps(files, state_dir)?;
    let report = build_report(&steps);

    if json {
        let out =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_saga_report(&report);
    }
    Ok(())
}

fn build_saga_steps(
    files: &[std::path::PathBuf],
    state_dir: &Path,
) -> Result<Vec<SagaStep>, String> {
    let mut steps = Vec::new();
    for f in files {
        let config = parse_and_validate(f)?;
        let snapshot = compute_snapshot_path(state_dir, &config.name);
        steps.push(SagaStep {
            stack_name: config.name.clone(),
            config_path: f.display().to_string(),
            snapshot_path: Some(snapshot),
            status: SagaStepStatus::Pending,
            error: None,
        });
    }
    Ok(steps)
}

fn compute_snapshot_path(state_dir: &Path, stack_name: &str) -> String {
    state_dir
        .join(format!(".saga-snapshot-{stack_name}"))
        .display()
        .to_string()
}

fn build_report(steps: &[SagaStep]) -> SagaReport {
    let total = steps.len();
    let applied = count_status(steps, "APPLIED");
    let failed = count_status(steps, "FAILED");
    let compensated = count_status(steps, "COMPENSATED");

    SagaReport {
        steps: steps.to_vec(),
        total,
        applied,
        failed,
        compensated,
        success: failed == 0,
    }
}

fn count_status(steps: &[SagaStep], status: &str) -> usize {
    steps
        .iter()
        .filter(|s| format!("{}", s.status) == status)
        .count()
}

fn print_saga_report(report: &SagaReport) {
    println!("Saga Execution Plan");
    println!("===================");
    println!(
        "Stacks: {} | Applied: {} | Failed: {} | Compensated: {}",
        report.total, report.applied, report.failed, report.compensated
    );
    println!("Success: {}", report.success);
    println!();
    for s in &report.steps {
        println!(
            "  [{}] {} ({})",
            s.status, s.stack_name, s.config_path
        );
        if let Some(ref snap) = s.snapshot_path {
            println!("    snapshot: {snap}");
        }
    }
}

/// Create a compensating snapshot for a stack.
pub fn create_snapshot(state_dir: &Path, stack_name: &str) -> Result<String, String> {
    let snapshot_path = compute_snapshot_path(state_dir, stack_name);
    let src = state_dir.join(stack_name);
    if src.exists() {
        let dest = std::path::Path::new(&snapshot_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir: {e}"))?;
        }
        // Copy state directory content
        copy_dir_simple(&src, dest)?;
    }
    Ok(snapshot_path)
}

fn copy_dir_simple(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("mkdir: {e}"))?;
    let entries = std::fs::read_dir(src).map_err(|e| format!("readdir: {e}"))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_file() {
            std::fs::copy(&path, &target).map_err(|e| format!("copy: {e}"))?;
        }
    }
    Ok(())
}
