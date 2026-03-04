//! FJ-021: Container execution transport.
//!
//! Executes scripts inside containers via `docker exec -i` or `podman exec -i`.
//! Shares the same mechanism as local/SSH: pipe shell script to bash stdin.

use super::ExecOutput;
use crate::core::types::Machine;
use std::io::Write;
use std::process::{Command, Stdio};

/// Execute a shell script inside a running container.
pub fn exec_container(machine: &Machine, script: &str) -> Result<ExecOutput, String> {
    let config = machine
        .container
        .as_ref()
        .ok_or_else(|| format!("machine '{}' has no container config", machine.hostname))?;

    let container_name = machine.container_name();

    let mut child = Command::new(&config.runtime)
        .args(["exec", "-i", &container_name, "bash"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to exec in container '{container_name}': {e}"))?;

    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| format!("stdin write error: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait error: {e}"))?;

    Ok(ExecOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Ensure a container is running for the given machine.
/// For ephemeral containers, creates one from the image.
/// For attached containers (ephemeral=false), verifies the named container exists.
pub fn ensure_container(machine: &Machine) -> Result<(), String> {
    let config = machine
        .container
        .as_ref()
        .ok_or_else(|| format!("machine '{}' has no container config", machine.hostname))?;

    let container_name = machine.container_name();

    // Check if container already exists and is running
    let check = Command::new(&config.runtime)
        .args(["inspect", "-f", "{{.State.Running}}", &container_name])
        .output()
        .map_err(|e| format!("failed to inspect container '{container_name}': {e}"))?;

    if check.status.success() {
        let running = String::from_utf8_lossy(&check.stdout);
        if running.trim() == "true" {
            return Ok(());
        }
    }

    // Container doesn't exist or isn't running — create it
    let image = config.image.as_deref().ok_or_else(|| {
        format!(
            "machine '{}' container has no image (required to create)",
            machine.hostname
        )
    })?;

    let mut args = vec!["run", "-d", "--name", &container_name];

    if config.init {
        args.push("--init");
    }
    if config.privileged {
        args.push("--privileged");
    }

    let gpus_value;
    if let Some(ref gpus) = config.gpus {
        gpus_value = gpus.clone();
        args.push("--gpus");
        args.push(&gpus_value);
    }

    // Device passthrough (AMD ROCm: /dev/kfd, /dev/dri; Intel: /dev/dri)
    let device_values: Vec<String> = config.devices.clone();
    for dev in &device_values {
        args.push("--device");
        args.push(dev);
    }

    // Group-add for device access (e.g., video, render for AMD GPUs)
    let group_values: Vec<String> = config.group_add.clone();
    for grp in &group_values {
        args.push("--group-add");
        args.push(grp);
    }

    // Environment variables (CUDA_VISIBLE_DEVICES, ROCR_VISIBLE_DEVICES, etc.)
    let env_pairs: Vec<String> = config
        .env
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    for pair in &env_pairs {
        args.push("--env");
        args.push(pair);
    }

    // Volume mounts (Docker socket, data dirs, etc.)
    let volume_values: Vec<String> = config.volumes.clone();
    for vol in &volume_values {
        args.push("-v");
        args.push(vol);
    }

    args.push(image);
    args.push("sleep");
    args.push("infinity");

    let run = Command::new(&config.runtime)
        .args(&args)
        .output()
        .map_err(|e| format!("failed to start container '{container_name}': {e}"))?;

    if !run.status.success() {
        return Err(format!(
            "failed to start container '{}': {}",
            container_name,
            String::from_utf8_lossy(&run.stderr).trim()
        ));
    }

    Ok(())
}

/// Remove a container (used for ephemeral cleanup).
pub fn cleanup_container(machine: &Machine) -> Result<(), String> {
    let config = machine
        .container
        .as_ref()
        .ok_or_else(|| format!("machine '{}' has no container config", machine.hostname))?;

    let container_name = machine.container_name();

    let rm = Command::new(&config.runtime)
        .args(["rm", "-f", &container_name])
        .output()
        .map_err(|e| format!("failed to remove container '{container_name}': {e}"))?;

    if !rm.status.success() {
        return Err(format!(
            "failed to remove container '{}': {}",
            container_name,
            String::from_utf8_lossy(&rm.stderr).trim()
        ));
    }

    Ok(())
}
