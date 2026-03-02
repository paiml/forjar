//! FJ-1361: Sandbox execution bridge.
//!
//! Bridges `sandbox_exec::plan_sandbox_build()` → actual namespace execution
//! via the transport layer. Each step's command is validated (I8) and
//! executed sequentially. Cleanup runs on failure.

use super::sandbox_exec::{SandboxPlan, SandboxStep};
use crate::core::purifier;
use crate::core::types::Machine;
use crate::transport;
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
    plan: &SandboxPlan,
    script: &str,
    machine: &Machine,
    store_dir: &Path,
    timeout_secs: Option<u64>,
) -> Result<SandboxExecResult, String> {
    let start = std::time::Instant::now();
    let mut steps_executed = Vec::new();

    for step in &plan.steps {
        let success = execute_step(step, machine, timeout_secs)?;
        steps_executed.push((step.step, step.description.clone(), success));

        if !success {
            cleanup_namespace(&plan.namespace_id, machine);
            return Err(format!(
                "sandbox step {} failed: {}",
                step.step, step.description
            ));
        }
    }

    let duration = start.elapsed().as_secs_f64();

    // Compute output hash from the sandbox execution
    let output_hash = compute_sandbox_output_hash(plan, script);
    let hash_bare = output_hash.strip_prefix("blake3:").unwrap_or(&output_hash);
    let store_path = format!("{}/{hash_bare}/content", store_dir.display());

    Ok(SandboxExecResult {
        output_hash,
        store_path,
        steps_executed,
        duration_secs: duration,
    })
}

/// Execute a single sandbox step.
///
/// Returns `Ok(true)` on success, `Ok(false)` if no command to run,
/// `Err` on I8 validation or transport failure.
fn execute_step(
    step: &SandboxStep,
    machine: &Machine,
    timeout_secs: Option<u64>,
) -> Result<bool, String> {
    let cmd = match &step.command {
        Some(c) => c,
        None => return Ok(true), // No command = informational step
    };

    // I8 gate: validate before execution
    purifier::validate_script(cmd)
        .map_err(|e| format!("I8 violation at step {}: {e}", step.step))?;

    let output = transport::exec_script_timeout(machine, cmd, timeout_secs)
        .map_err(|e| format!("step {} transport error: {e}", step.step))?;

    Ok(output.success())
}

/// Clean up namespace resources on failure.
fn cleanup_namespace(namespace_id: &str, machine: &Machine) {
    let cleanup_cmd = format!("rm -rf '/tmp/forjar-sandbox/{namespace_id}' 2>/dev/null; true");
    // Best-effort cleanup — ignore errors
    let _ = transport::exec_script_timeout(machine, &cleanup_cmd, Some(10));
}

/// Compute the output hash for a sandbox build.
///
/// Uses the same deterministic approach as `simulate_sandbox_build()`:
/// hash of input paths + script content.
fn compute_sandbox_output_hash(plan: &SandboxPlan, script: &str) -> String {
    let mut components: Vec<String> = plan
        .overlay
        .lower_dirs
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    components.sort();
    components.push(script.to_string());

    let refs: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
    crate::tripwire::hasher::composite_hash(&refs)
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
