//! FJ-2604: Infrastructure mutation runner with sandbox integration.
//!
//! Executes the mutation testing algorithm: for each resource, apply baseline,
//! apply mutation, detect drift, re-converge, verify. Bridges MutationOperator
//! types to real (or simulated) sandbox execution.

use crate::core::types::{MutationOperator, MutationReport, MutationResult, SandboxBackend};

/// Configuration for a mutation test run.
#[derive(Debug, Clone)]
pub struct MutationRunConfig {
    /// Sandbox backend to use (pepita, container, chroot).
    pub backend: SandboxBackend,
    /// Maximum mutations per resource (default: 50).
    pub mutations_per_resource: usize,
    /// Maximum parallel sandboxes (default: 4).
    pub parallelism: usize,
    /// Whether to attempt re-convergence after mutation.
    pub test_reconvergence: bool,
}

impl Default for MutationRunConfig {
    fn default() -> Self {
        Self {
            backend: SandboxBackend::Pepita,
            mutations_per_resource: 50,
            parallelism: 4,
            test_reconvergence: true,
        }
    }
}

/// A resource target for mutation testing.
#[derive(Debug, Clone)]
pub struct MutationTarget {
    /// Resource ID.
    pub resource_id: String,
    /// Resource type (file, package, service, mount, etc.).
    pub resource_type: String,
    /// Apply script for establishing baseline.
    pub apply_script: String,
    /// Drift detection script.
    pub drift_script: String,
    /// Expected content hash at baseline.
    pub expected_hash: String,
}

/// Generate the mutation script for a given operator and resource.
///
/// Scripts use `$FORJAR_SANDBOX` prefix when set (local sandbox mode),
/// falling back to absolute paths for container/remote execution.
pub fn mutation_script(operator: MutationOperator, resource_id: &str) -> String {
    // Use $FORJAR_SANDBOX prefix for sandbox-aware paths (double-quoted for expansion)
    match operator {
        MutationOperator::DeleteFile => {
            format!(r#"rm -f "${{FORJAR_SANDBOX:-}}/etc/forjar/{resource_id}" 2>/dev/null; true"#)
        }
        MutationOperator::ModifyContent => {
            format!(
                r#"echo 'MUTATED_CONTENT' >> "${{FORJAR_SANDBOX:-}}/etc/forjar/{resource_id}" 2>/dev/null; true"#
            )
        }
        MutationOperator::ChangePermissions => {
            format!(
                r#"chmod 000 "${{FORJAR_SANDBOX:-}}/etc/forjar/{resource_id}" 2>/dev/null; true"#
            )
        }
        MutationOperator::StopService => {
            format!("systemctl stop '{resource_id}' 2>/dev/null; true")
        }
        MutationOperator::RemovePackage => {
            format!("apt-get remove -y '{resource_id}' 2>/dev/null; true")
        }
        MutationOperator::KillProcess => {
            format!("pkill -f '{resource_id}' 2>/dev/null; true")
        }
        MutationOperator::UnmountFilesystem => {
            format!(r#"umount "${{FORJAR_SANDBOX:-}}/mnt/{resource_id}" 2>/dev/null; true"#)
        }
        MutationOperator::CorruptConfig => {
            format!(
                r#"sed -i 's/^/#CORRUPTED /' "${{FORJAR_SANDBOX:-}}/etc/forjar/{resource_id}" 2>/dev/null; true"#
            )
        }
    }
}

/// Get applicable mutation operators for a resource type.
pub fn applicable_operators(resource_type: &str) -> Vec<MutationOperator> {
    let all = [
        MutationOperator::DeleteFile,
        MutationOperator::ModifyContent,
        MutationOperator::ChangePermissions,
        MutationOperator::StopService,
        MutationOperator::RemovePackage,
        MutationOperator::KillProcess,
        MutationOperator::UnmountFilesystem,
        MutationOperator::CorruptConfig,
    ];

    all.iter()
        .filter(|op| op.applicable_types().contains(&resource_type))
        .copied()
        .collect()
}

/// Run a mutation test with mode dispatch.
///
/// When a container runtime is available and the backend is Container,
/// runs inside a real ephemeral container. Otherwise falls back to simulated.
pub fn run_mutation_test_dispatch(
    target: &MutationTarget,
    operator: MutationOperator,
    config: &MutationRunConfig,
) -> MutationResult {
    let mode = super::convergence_runner::resolve_mode(config.backend);
    match (mode, config.backend) {
        (super::convergence_runner::RunnerMode::Sandbox, SandboxBackend::Container) => {
            super::mutation_container::run_mutation_test_container(target, operator, config)
        }
        _ => run_mutation_test(target, operator, config),
    }
}

/// Check if a mutation operator is safe for local (non-container) execution.
///
/// Only file-scoped operators that work within $FORJAR_SANDBOX are safe.
/// System operators (systemctl, apt-get, pkill, umount) MUST NEVER run
/// on the host — they require a container or remote sandbox.
fn is_safe_for_local(operator: MutationOperator) -> bool {
    matches!(
        operator,
        MutationOperator::DeleteFile
            | MutationOperator::ModifyContent
            | MutationOperator::ChangePermissions
            | MutationOperator::CorruptConfig
    )
}

/// Run a single mutation test in a local tempdir sandbox.
///
/// Only file-scoped operators run locally. System operators (StopService,
/// RemovePackage, KillProcess, UnmountFilesystem) are skipped with an
/// error indicating they require a container backend.
pub fn run_mutation_test(
    target: &MutationTarget,
    operator: MutationOperator,
    config: &MutationRunConfig,
) -> MutationResult {
    let start = std::time::Instant::now();

    // SAFETY: Never run system-scoped mutations on the host
    if !is_safe_for_local(operator) {
        return MutationResult {
            resource_id: target.resource_id.clone(),
            resource_type: target.resource_type.clone(),
            operator,
            detected: false,
            reconverged: None,
            duration_ms: start.elapsed().as_millis() as u64,
            error: Some(format!(
                "{operator} requires container backend (unsafe for local execution)"
            )),
        };
    }

    // Create isolated sandbox directory
    let sandbox_dir = std::env::temp_dir().join(format!(
        "forjar-mut-{}-{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let _ = std::fs::create_dir_all(&sandbox_dir);

    let result = run_mutation_in_sandbox(target, operator, config, &sandbox_dir, start);

    // Cleanup sandbox
    let _ = std::fs::remove_dir_all(&sandbox_dir);
    result
}

fn run_mutation_in_sandbox(
    target: &MutationTarget,
    operator: MutationOperator,
    config: &MutationRunConfig,
    sandbox_dir: &std::path::Path,
    start: std::time::Instant,
) -> MutationResult {
    // Step 1: Apply baseline
    if let Err(e) = local_apply(&target.apply_script, sandbox_dir) {
        return MutationResult {
            resource_id: target.resource_id.clone(),
            resource_type: target.resource_type.clone(),
            operator,
            detected: false,
            reconverged: None,
            duration_ms: start.elapsed().as_millis() as u64,
            error: Some(format!("baseline apply failed: {e}")),
        };
    }

    // Step 2: Capture baseline state
    let baseline_hash = local_apply(&target.drift_script, sandbox_dir).unwrap_or_default();

    // Step 3: Apply mutation
    let mutation_cmd = mutation_script(operator, &target.resource_id);
    let _ = local_apply(&mutation_cmd, sandbox_dir);

    // Step 4: Detect drift (real comparison)
    let detected = local_drift_detection(&target.drift_script, &baseline_hash, sandbox_dir);

    // Step 5-6: Re-convergence
    let reconverged = if config.test_reconvergence && detected {
        let reapply = local_apply(&target.apply_script, sandbox_dir);
        Some(reapply.is_ok())
    } else {
        None
    };

    MutationResult {
        resource_id: target.resource_id.clone(),
        resource_type: target.resource_type.clone(),
        operator,
        detected,
        reconverged,
        duration_ms: start.elapsed().as_millis() as u64,
        error: None,
    }
}

/// Run mutation tests for all applicable operators on all targets.
pub fn run_mutation_suite(
    targets: &[MutationTarget],
    config: &MutationRunConfig,
) -> MutationReport {
    let mut all_results = Vec::new();

    for target in targets {
        let operators = applicable_operators(&target.resource_type);
        let ops_to_run: Vec<_> = operators
            .into_iter()
            .take(config.mutations_per_resource)
            .collect();

        for operator in ops_to_run {
            let result = run_mutation_test(target, operator, config);
            all_results.push(result);
        }
    }

    MutationReport::from_results(all_results)
}

/// Run mutation tests in parallel across targets.
pub fn run_mutation_parallel(
    targets: Vec<MutationTarget>,
    config: &MutationRunConfig,
) -> MutationReport {
    if targets.is_empty() {
        return MutationReport::default();
    }

    let par = config.parallelism.max(1);
    let all_results = std::sync::Mutex::new(Vec::new());
    let chunks: Vec<_> = targets.chunks(par).collect();

    for chunk in chunks {
        std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|target| {
                    s.spawn(|| {
                        let operators = applicable_operators(&target.resource_type);
                        let mut results = Vec::new();
                        for op in operators {
                            results.push(run_mutation_test_dispatch(target, op, config));
                        }
                        results
                    })
                })
                .collect();

            for handle in handles {
                if let Ok(results) = handle.join() {
                    all_results
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .extend(results);
                }
            }
        });
    }

    MutationReport::from_results(all_results.into_inner().unwrap_or_else(|e| e.into_inner()))
}

/// Format a mutation run summary.
pub fn format_mutation_run(report: &MutationReport) -> String {
    let mut out = report.format_summary();
    out.push_str(&format!(
        "\n{} targets, {} mutations total\n",
        report.by_type.len(),
        report.score.total,
    ));
    out
}

/// Execute a script locally in a sandbox directory, returning stdout hash.
///
/// SAFETY: Only executes scripts scoped to the sandbox directory.
/// System-modifying commands are blocked by the `is_safe_for_local` gate
/// at the caller level (`run_mutation_test`).
fn local_apply(script: &str, sandbox_dir: &std::path::Path) -> Result<String, String> {
    if script.is_empty() {
        return Err("empty script".into());
    }
    let output = std::process::Command::new("bash")
        .args(["-euo", "pipefail", "-c", script])
        .current_dir(sandbox_dir)
        .env("FORJAR_SANDBOX", sandbox_dir)
        .output()
        .map_err(|e| format!("local exec: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "exit {}: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let refs = [stdout.as_ref()];
    Ok(crate::tripwire::hasher::composite_hash(&refs))
}

/// Detect drift by running the drift script locally and comparing output hash.
///
/// Executes the drift script in a tempdir sandbox via `bash -euo pipefail`,
/// hashes the stdout output, and compares against the baseline hash.
fn local_drift_detection(
    drift_script: &str,
    baseline_hash: &str,
    sandbox_dir: &std::path::Path,
) -> bool {
    let output = std::process::Command::new("bash")
        .args(["-euo", "pipefail", "-c", drift_script])
        .current_dir(sandbox_dir)
        .env("FORJAR_SANDBOX", sandbox_dir)
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let refs = [stdout.as_ref()];
            let current_hash = crate::tripwire::hasher::composite_hash(&refs);
            current_hash != baseline_hash
        }
        Err(_) => true, // execution failure = drift detected
    }
}

// RunnerMode lives in convergence_runner.rs (single source of truth)
