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

    #[test]
    fn test_fj010_local_empty_script() {
        let out = exec_local("").unwrap();
        assert!(out.success());
        assert!(out.stdout.is_empty());
    }

    #[test]
    fn test_fj010_local_environment_variables() {
        let out = exec_local("FORJAR_TEST=yes; echo $FORJAR_TEST").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "yes");
    }

    #[test]
    fn test_fj010_local_heredoc() {
        let out = exec_local("cat <<'EOF'\nhello heredoc\nEOF").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "hello heredoc");
    }

    #[test]
    fn test_fj010_local_exit_code_range() {
        for code in [0, 1, 2, 126, 127, 255] {
            let out = exec_local(&format!("exit {}", code)).unwrap();
            assert_eq!(out.exit_code, code);
        }
    }

    #[test]
    fn test_fj010_local_large_output() {
        // Generate 1000 lines
        let out = exec_local("seq 1 1000").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.lines().count(), 1000);
    }

    #[test]
    fn test_fj153_local_binary_output() {
        // Test that binary-ish output is handled via lossy UTF-8
        let out = exec_local("printf '\\x00\\x01\\x02'").unwrap();
        assert!(out.success());
        // Should not panic on non-UTF-8 bytes
        assert!(!out.stdout.is_empty());
    }

    #[test]
    fn test_fj153_local_stdout_and_stderr() {
        let out = exec_local("echo out; echo err >&2").unwrap();
        assert!(out.success());
        assert!(out.stdout.contains("out"));
        assert!(out.stderr.contains("err"));
    }

    #[test]
    fn test_fj153_local_long_running_script() {
        // Script with multiple sequential commands
        let out =
            exec_local("for i in $(seq 1 100); do echo \"line_$i\"; done").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.lines().count(), 100);
    }

    #[test]
    fn test_fj010_local_set_euo() {
        // set -euo pipefail is standard forjar preamble
        let out = exec_local("set -euo pipefail\necho ok").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "ok");
    }
}
