//! FJ-011: SSH execution transport.
//!
//! Uses the `ssh` binary directly — no libssh2 dependency.
//! Script is piped to stdin (not passed as argument) to avoid
//! argument length limits and injection vectors.

use super::ExecOutput;
use crate::core::types::Machine;
use std::io::Write;
use std::process::{Command, Stdio};

/// Execute a shell script on a remote machine via SSH.
pub fn exec_ssh(machine: &Machine, script: &str) -> Result<ExecOutput, String> {
    let mut cmd = Command::new("ssh");
    cmd.args(["-o", "BatchMode=yes"])
        .args(["-o", "ConnectTimeout=5"])
        .args(["-o", "StrictHostKeyChecking=accept-new"]);

    if let Some(ref key) = machine.ssh_key {
        // Expand ~ to home directory (CB-506: avoid byte indexing)
        let expanded = if let Some(rest) = key.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                format!("{}/{}", home, rest)
            } else {
                key.clone()
            }
        } else {
            key.clone()
        };
        cmd.args(["-i", &expanded]);
    }

    cmd.arg(format!("{}@{}", machine.user, machine.addr))
        .arg("bash")
        .stdin(Stdio::piped())
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_fj011_ssh_key_expansion() {
        // Test that ~ expansion works (unit test, no actual SSH)
        let key = "~/.ssh/id_ed25519";
        let expanded = if let Some(rest) = key.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                format!("{}/{}", home, rest)
            } else {
                key.to_string()
            }
        } else {
            key.to_string()
        };
        assert!(expanded.contains(".ssh/id_ed25519"));
        assert!(!expanded.starts_with('~'));
    }
}
