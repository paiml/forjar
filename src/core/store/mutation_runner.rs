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
pub fn mutation_script(operator: MutationOperator, resource_id: &str) -> String {
    match operator {
        MutationOperator::DeleteFile => {
            format!("rm -f '/etc/forjar/{resource_id}' 2>/dev/null; true")
        }
        MutationOperator::ModifyContent => {
            format!("echo 'MUTATED_CONTENT' >> '/etc/forjar/{resource_id}' 2>/dev/null; true")
        }
        MutationOperator::ChangePermissions => {
            format!("chmod 000 '/etc/forjar/{resource_id}' 2>/dev/null; true")
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
            format!("umount '/mnt/{resource_id}' 2>/dev/null; true")
        }
        MutationOperator::CorruptConfig => {
            format!("sed -i 's/^/#CORRUPTED /' '/etc/forjar/{resource_id}' 2>/dev/null; true")
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

/// Run a single mutation test (simulated mode).
///
/// Algorithm (from spec):
/// 1. Apply baseline in sandbox
/// 2. Apply mutation
/// 3. Run drift detection
/// 4. Assert drift was detected
/// 5. Re-converge (if configured)
/// 6. Verify convergence
pub fn run_mutation_test(
    target: &MutationTarget,
    operator: MutationOperator,
    config: &MutationRunConfig,
) -> MutationResult {
    let start = std::time::Instant::now();

    // Step 1: Apply baseline
    let baseline = simulate_apply(&target.apply_script);
    if let Err(e) = baseline {
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

    // Step 2: Apply mutation
    let mutation_cmd = mutation_script(operator, &target.resource_id);
    let _ = simulate_apply(&mutation_cmd);

    // Step 3: Detect drift
    let detected = simulate_drift_detection(&target.drift_script, &target.expected_hash);

    // Step 4-6: Re-convergence
    let reconverged = if config.test_reconvergence && detected {
        let reapply = simulate_apply(&target.apply_script);
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
                    all_results.lock().unwrap().extend(results);
                }
            }
        });
    }

    MutationReport::from_results(all_results.into_inner().unwrap())
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

/// Simulate applying a script (returns hash).
fn simulate_apply(script: &str) -> Result<String, String> {
    if script.is_empty() {
        return Err("empty script".into());
    }
    let refs = [script];
    Ok(crate::tripwire::hasher::composite_hash(&refs))
}

/// Simulate drift detection.
///
/// In a real implementation, this would run the drift script in the sandbox
/// and compare actual vs expected hash. Here we simulate by checking if
/// the mutation changed the state.
fn simulate_drift_detection(_drift_script: &str, _expected_hash: &str) -> bool {
    // Simulated: mutations are always detected in test mode
    // Real implementation would compare hashes after mutation
    true
}

// RunnerMode lives in convergence_runner.rs (single source of truth)
