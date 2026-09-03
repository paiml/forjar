//! FJ-1361: Sandbox execution bridge.
//!
//! Bridges `sandbox_exec::plan_sandbox_build()` → actual namespace execution
//! via the transport layer. Each step's command is validated (I8) and
//! executed sequentially. Cleanup runs on failure.

use super::sandbox_exec::SandboxPlan;
use crate::core::purifier;
use crate::core::types::Machine;
use std::path::Path;

/// Result of a completed sandbox execution.
#[derive(Debug, Clone)]
pub struct SandboxExecResult {
    /// BLAKE3 hash of the output directory
    pub output_hash: String,
    /// Store path where the output was placed
    pub store_path: String,
    /// Steps executed: (step number, description, success)
    pub steps_executed: Vec<(u8, String, bool)>,
    /// Total execution time in seconds
    pub duration_secs: f64,
}

/// Execute a sandbox build plan via the transport layer.
///
/// Each step's command is validated via bashrs (I8 invariant) and executed
/// sequentially. On failure, cleanup is attempted.
pub fn execute_sandbox_plan(
    _plan: &SandboxPlan,
    _script: &str,
    _machine: &Machine,
    _store_dir: &Path,
    _timeout_secs: Option<u64>,
) -> Result<SandboxExecResult, String> {
    Err("not implemented: sandbox execution needs seccomp-bpf and forjar-hash-dir, and neither exists as a binary on any host (Refs #410); refusing by name rather than simulating a build".to_string())
}

/// Execute a sandbox plan in dry-run mode (validate all commands without running).
///
/// Returns the list of commands that would be executed. Useful for
/// pre-flight validation and CI gating.
pub fn dry_run_sandbox_plan(plan: &SandboxPlan) -> Result<Vec<String>, String> {
    let mut commands = Vec::new();

    for step in &plan.steps {
        if let Some(cmd) = &step.command {
            purifier::validate_script(cmd)
                .map_err(|e| format!("I8 dry-run violation at step {}: {e}", step.step))?;
            commands.push(cmd.clone());
        }
    }

    Ok(commands)
}

/// Check if a sandbox plan is executable (all commands pass I8 validation).
pub fn validate_sandbox_commands(plan: &SandboxPlan) -> Vec<String> {
    let mut errors = Vec::new();

    for step in &plan.steps {
        if let Some(cmd) = &step.command {
            if let Err(e) = purifier::validate_script(cmd) {
                errors.push(format!("step {}: {e}", step.step));
            }
        }
    }

    errors
}
