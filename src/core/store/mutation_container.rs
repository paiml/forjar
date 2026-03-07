//! FJ-2604: Container-based mutation testing.
//!
//! Executes mutation tests inside ephemeral containers (Docker/Podman).
//! Each mutation gets a fresh container: baseline → mutate → detect drift → re-converge.

use super::convergence_container::detect_container_runtime;
use super::mutation_runner::{mutation_script, MutationRunConfig, MutationTarget};
use crate::core::types::{MutationOperator, MutationResult};

/// Execute a script inside a container and return stdout.
fn container_exec(
    runtime: &str,
    container_name: &str,
    script: &str,
) -> Result<String, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(runtime)
        .args(["exec", "-i", container_name, "bash"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("container exec failed: {e}"))?;

    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| format!("stdin write: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait: {e}"))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a single mutation test inside an ephemeral container.
///
/// 1. Start container
/// 2. Apply baseline
/// 3. Apply mutation
/// 4. Run drift detection (compare state before/after mutation)
/// 5. Re-converge if configured
/// 6. Teardown
pub fn run_mutation_test_container(
    target: &MutationTarget,
    operator: MutationOperator,
    config: &MutationRunConfig,
) -> MutationResult {
    use std::process::Command;

    let start = std::time::Instant::now();
    let runtime = match detect_container_runtime() {
        Some(rt) => rt,
        None => {
            return MutationResult {
                resource_id: target.resource_id.clone(),
                resource_type: target.resource_type.clone(),
                operator,
                detected: false,
                reconverged: None,
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some("no container runtime available".into()),
            }
        }
    };

    let container_name = format!(
        "forjar-mut-{}-{}",
        &target.resource_id,
        operator.to_string().replace('_', "-")
    );

    // Start ephemeral container
    let run = Command::new(&runtime)
        .args([
            "run", "-d", "--rm", "--name", &container_name,
            "debian:bookworm-slim", "sleep", "300",
        ])
        .output();

    if let Err(e) = run {
        return err_result(target, operator, start, &format!("container start: {e}"));
    }
    if let Ok(ref o) = run {
        if !o.status.success() {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return err_result(
                target, operator, start,
                &format!("container start failed: {}", stderr.trim()),
            );
        }
    }

    // Step 1: Apply baseline
    if let Err(e) = container_exec(&runtime, &container_name, &target.apply_script) {
        let _ = Command::new(&runtime).args(["rm", "-f", &container_name]).output();
        return err_result(target, operator, start, &format!("baseline apply: {e}"));
    }

    // Step 2: Capture baseline state
    let baseline_state = container_exec(&runtime, &container_name, &target.drift_script)
        .unwrap_or_default();
    let baseline_hash = {
        let refs = [baseline_state.as_str()];
        crate::tripwire::hasher::composite_hash(&refs)
    };

    // Step 3: Apply mutation
    let mutation_cmd = mutation_script(operator, &target.resource_id);
    let _ = container_exec(&runtime, &container_name, &mutation_cmd);

    // Step 4: Detect drift (compare state after mutation to baseline)
    let mutated_state = container_exec(&runtime, &container_name, &target.drift_script)
        .unwrap_or_default();
    let mutated_hash = {
        let refs = [mutated_state.as_str()];
        crate::tripwire::hasher::composite_hash(&refs)
    };
    let detected = baseline_hash != mutated_hash;

    // Step 5: Re-convergence
    let reconverged = if config.test_reconvergence && detected {
        let reapply = container_exec(&runtime, &container_name, &target.apply_script);
        Some(reapply.is_ok())
    } else {
        None
    };

    // Cleanup
    let _ = Command::new(&runtime)
        .args(["rm", "-f", &container_name])
        .output();

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

fn err_result(
    target: &MutationTarget,
    operator: MutationOperator,
    start: std::time::Instant,
    msg: &str,
) -> MutationResult {
    MutationResult {
        resource_id: target.resource_id.clone(),
        resource_type: target.resource_type.clone(),
        operator,
        detected: false,
        reconverged: None,
        duration_ms: start.elapsed().as_millis() as u64,
        error: Some(msg.to_string()),
    }
}
