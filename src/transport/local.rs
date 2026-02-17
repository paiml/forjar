//! FJ-010: Local execution transport.

use super::ExecOutput;
use std::io::Write;
use std::process::{Command, Stdio};

/// Execute a shell script locally via `bash`.
/// Uses bash (not sh/dash) because generated scripts use `set -o pipefail`.
pub fn exec_local(script: &str) -> Result<ExecOutput, String> {
    let mut child = Command::new("bash")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn bash: {}", e))?;

    if let Some(ref mut stdin) = child.stdin {
        stdin
            .write_all(script.as_bytes())
            .map_err(|e| format!("stdin write error: {}", e))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait error: {}", e))?;

    Ok(ExecOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj010_local_echo() {
        let out = exec_local("echo hello").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "hello");
    }

    #[test]
    fn test_fj010_local_failure() {
        let out = exec_local("exit 42").unwrap();
        assert!(!out.success());
        assert_eq!(out.exit_code, 42);
    }

    #[test]
    fn test_fj010_local_multiline() {
        let out = exec_local("echo line1\necho line2").unwrap();
        assert!(out.success());
        let lines: Vec<_> = out.stdout.lines().collect();
        assert_eq!(lines, vec!["line1", "line2"]);
    }

    #[test]
    fn test_fj010_local_stderr() {
        let out = exec_local("echo err >&2").unwrap();
        assert!(out.success());
        assert!(out.stderr.contains("err"));
    }

    #[test]
    fn test_fj010_local_signal_killed() {
        // Process killed by signal has no exit code; unwrap_or(-1) returns -1
        let out = exec_local("kill -9 $$").unwrap();
        assert_eq!(out.exit_code, -1);
    }

    #[test]
    fn test_fj010_local_pipefail() {
        let out = exec_local("set -euo pipefail\nfalse | true").unwrap();
        assert!(!out.success(), "pipefail should catch false in pipeline");
    }
}
