//! FJ-011/252: SSH execution transport with connection multiplexing.
//!
//! Uses the `ssh` binary directly — no libssh2 dependency.
//! Script is piped to stdin (not passed as argument) to avoid
//! argument length limits and injection vectors.
//!
//! FJ-252: ControlMaster multiplexing reuses a single TCP connection
//! per machine, reducing SSH handshake overhead from O(n) to O(1).

use super::ExecOutput;
use crate::core::types::Machine;
use std::io::Write;
use std::process::{Command, Stdio};

/// Socket directory for SSH ControlMaster sockets.
pub(crate) const CONTROL_DIR: &str = "/tmp/forjar-ssh";

/// ControlPersist timeout in seconds (keep master alive after last connection).
pub(crate) const CONTROL_PERSIST_SECS: u32 = 60;

/// Get the ControlPath for a machine.
pub fn control_path(machine: &Machine) -> String {
    format!("{}/{}@{}", CONTROL_DIR, machine.user, machine.addr)
}

/// Check if a ControlMaster socket exists for a machine.
pub fn has_control_master(machine: &Machine) -> bool {
    let sock = control_path(machine);
    std::path::Path::new(&sock).exists()
}

/// Start a ControlMaster connection for a machine.
/// Returns Ok(true) if started, Ok(false) if already running.
/// Errors if the SSH connection fails.
pub fn start_control_master(machine: &Machine) -> Result<bool, String> {
    // Ensure socket directory exists
    std::fs::create_dir_all(CONTROL_DIR)
        .map_err(|e| format!("cannot create {}: {}", CONTROL_DIR, e))?;

    let sock = control_path(machine);
    if std::path::Path::new(&sock).exists() {
        // Check if master is alive
        let status = Command::new("ssh")
            .args(["-O", "check", "-o", "BatchMode=yes", "-S", &sock])
            .arg(format!("{}@{}", machine.user, machine.addr))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("ssh check failed: {}", e))?;
        if status.success() {
            return Ok(false); // already running
        }
        // Stale socket — remove it
        let _ = std::fs::remove_file(&sock);
    }

    let mut args = vec![
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "ConnectTimeout=5".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "ControlMaster=yes".to_string(),
        "-o".to_string(),
        format!("ControlPath={}", sock),
        "-o".to_string(),
        format!("ControlPersist={}", CONTROL_PERSIST_SECS),
        "-N".to_string(), // no command — just open the master
        "-f".to_string(), // go to background
    ];

    if let Some(ref key) = machine.ssh_key {
        let expanded = expand_tilde(key);
        args.push("-i".to_string());
        args.push(expanded);
    }

    args.push(format!("{}@{}", machine.user, machine.addr));

    let output = Command::new("ssh")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .map_err(|e| format!("failed to start ControlMaster for {}: {}", machine.addr, e))?;

    if output.success() {
        Ok(true)
    } else {
        Err(format!(
            "ControlMaster failed for {}@{} (exit {})",
            machine.user,
            machine.addr,
            output.code().unwrap_or(-1)
        ))
    }
}

/// Stop a ControlMaster connection for a machine.
pub fn stop_control_master(machine: &Machine) -> Result<(), String> {
    let sock = control_path(machine);
    if !std::path::Path::new(&sock).exists() {
        return Ok(()); // nothing to stop
    }

    let _ = Command::new("ssh")
        .args(["-O", "exit", "-o", "BatchMode=yes", "-S", &sock])
        .arg(format!("{}@{}", machine.user, machine.addr))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // Clean up socket file even if exit command fails
    let _ = std::fs::remove_file(&sock);
    Ok(())
}

/// Stop all ControlMaster connections in the socket directory.
pub fn stop_all_control_masters() {
    if let Ok(entries) = std::fs::read_dir(CONTROL_DIR) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    let _ = std::fs::remove_dir(CONTROL_DIR);
}

/// Execute a shell script on a remote machine via SSH.
pub fn exec_ssh(machine: &Machine, script: &str) -> Result<ExecOutput, String> {
    let args = build_ssh_args(machine);
    let mut cmd = Command::new("ssh");
    for arg in &args {
        cmd.arg(arg);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn ssh to {}: {}", machine.addr, e))?;

    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| format!("stdin write error: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("ssh wait error: {}", e))?;

    Ok(ExecOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Expand ~ prefix to $HOME.
pub(crate) fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    path.to_string()
}

/// Build the SSH command arguments (without spawning).
/// Includes ControlMaster multiplexing options if a socket exists.
pub(crate) fn build_ssh_args(machine: &Machine) -> Vec<String> {
    let mut args = vec![
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "ConnectTimeout=5".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
    ];

    // FJ-252: Add multiplexing options
    let sock = control_path(machine);
    if std::path::Path::new(&sock).exists() {
        args.push("-o".to_string());
        args.push("ControlMaster=auto".to_string());
        args.push("-o".to_string());
        args.push(format!("ControlPath={}", sock));
    }

    if let Some(ref key) = machine.ssh_key {
        let expanded = expand_tilde(key);
        args.push("-i".to_string());
        args.push(expanded);
    }

    args.push(format!("{}@{}", machine.user, machine.addr));
    args.push("bash".to_string());
    args
}

