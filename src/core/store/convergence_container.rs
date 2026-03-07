//! FJ-2603: Container-based convergence testing.
//!
//! Executes convergence tests inside ephemeral containers (Docker/Podman).
//! Real I/O, real isolation — not hash-based simulation.

use super::convergence_runner::{ConvergenceResult, ConvergenceTarget};

/// Detect which container runtime is available.
pub fn detect_container_runtime() -> Option<String> {
    for rt in &["docker", "podman"] {
        if std::process::Command::new(rt)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(rt.to_string());
        }
    }
    None
}

/// Execute a script inside an ephemeral container and return stdout.
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

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "exit {}: {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a convergence test inside a disposable container.
///
/// Creates an ephemeral container, executes apply twice, queries state
/// after each apply, then tears down the container. Real I/O, real isolation.
pub fn run_convergence_test_container(target: &ConvergenceTarget) -> ConvergenceResult {
    use std::process::Command;

    let start = std::time::Instant::now();
    let runtime = match detect_container_runtime() {
        Some(rt) => rt,
        None => return err_result(target, start, "no container runtime available"),
    };

    let container_name = format!("forjar-conv-{}", &target.resource_id);

    // Start ephemeral container (debian-slim with bash)
    let run = Command::new(&runtime)
        .args([
            "run",
            "-d",
            "--rm",
            "--name",
            &container_name,
            "debian:bookworm-slim",
            "sleep",
            "300",
        ])
        .output();

    match run {
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return err_result(
                target,
                start,
                &format!("container start failed: {}", stderr.trim()),
            );
        }
        Err(e) => return err_result(target, start, &format!("container start: {e}")),
        _ => {}
    }

    // Step 3: First apply
    let first_apply = container_exec(&runtime, &container_name, &target.apply_script);
    if let Err(e) = first_apply {
        let _ = Command::new(&runtime)
            .args(["rm", "-f", &container_name])
            .output();
        return err_result(target, start, &format!("first apply: {e}"));
    }

    // Step 4: Query state after first apply
    let state_after_first =
        container_exec(&runtime, &container_name, &target.state_query_script);
    let first_hash = state_after_first.as_ref().map(|s| {
        let refs = [s.as_str()];
        crate::tripwire::hasher::composite_hash(&refs)
    });
    let converged = first_hash
        .as_ref()
        .map(|h| h == &target.expected_hash)
        .unwrap_or(false);

    // Step 5: Second apply (should be no-op)
    let second_apply = container_exec(&runtime, &container_name, &target.apply_script);
    let idempotent = second_apply.is_ok();

    // Step 6: Query state after second apply — should be unchanged
    let state_after_second =
        container_exec(&runtime, &container_name, &target.state_query_script);
    let second_hash = state_after_second.as_ref().ok().map(|s| {
        let refs = [s.as_str()];
        crate::tripwire::hasher::composite_hash(&refs)
    });
    let preserved = match (&first_hash, &second_hash) {
        (Ok(h1), Some(h2)) => h1 == h2,
        _ => false,
    };

    // Cleanup container
    let _ = Command::new(&runtime)
        .args(["rm", "-f", &container_name])
        .output();

    ConvergenceResult {
        resource_id: target.resource_id.clone(),
        resource_type: target.resource_type.clone(),
        converged,
        idempotent,
        preserved,
        duration_ms: start.elapsed().as_millis() as u64,
        error: None,
    }
}

fn err_result(
    target: &ConvergenceTarget,
    start: std::time::Instant,
    msg: &str,
) -> ConvergenceResult {
    ConvergenceResult {
        resource_id: target.resource_id.clone(),
        resource_type: target.resource_type.clone(),
        converged: false,
        idempotent: false,
        preserved: false,
        duration_ms: start.elapsed().as_millis() as u64,
        error: Some(msg.to_string()),
    }
}
