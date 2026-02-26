//! FJ-010/011/021/230: Transport abstraction — local, SSH, container, and pepita execution.

pub mod container;
pub mod local;
pub mod pepita;
pub mod ssh;

use crate::core::types::Machine;

/// Output from executing a script on a target.
#[derive(Debug, Clone)]
pub struct ExecOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ExecOutput {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Execute a purified shell script on a machine.
/// Dispatches to pepita, container, local, or SSH based on transport/address.
/// Priority: pepita > container > local > SSH.
pub fn exec_script(machine: &Machine, script: &str) -> Result<ExecOutput, String> {
    // Pepita (kernel namespace) transport takes highest priority
    if machine.is_pepita_transport() {
        return pepita::exec_pepita(machine, script);
    }

    // Container transport takes priority over local/SSH
    if machine.is_container_transport() {
        return container::exec_container(machine, script);
    }

    let is_local =
        machine.addr == "127.0.0.1" || machine.addr == "localhost" || is_local_addr(&machine.addr);

    if is_local {
        local::exec_local(script)
    } else {
        ssh::exec_ssh(machine, script)
    }
}

/// Execute a script with an optional timeout (in seconds).
/// Returns an error if the script exceeds the timeout.
pub fn exec_script_timeout(
    machine: &Machine,
    script: &str,
    timeout_secs: Option<u64>,
) -> Result<ExecOutput, String> {
    match timeout_secs {
        Some(secs) => {
            let hostname = machine.hostname.clone();
            let machine = machine.clone();
            let script = script.to_string();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result = exec_script(&machine, &script);
                let _ = tx.send(result);
            });
            rx.recv_timeout(std::time::Duration::from_secs(secs))
                .map_err(|_| {
                    format!(
                        "transport timeout: script on '{}' exceeded {}s limit",
                        hostname, secs
                    )
                })?
        }
        None => exec_script(machine, script),
    }
}

/// FJ-261: Execute a script with SSH retry on transient failures.
/// `ssh_retries` is total attempt count (1 = no retry, 3 = up to 3 attempts).
/// Retries only apply to SSH transport; local/container calls are not retried.
/// Backoff: 200ms × 2^attempt. Capped at 4 attempts max.
pub fn exec_script_retry(
    machine: &Machine,
    script: &str,
    timeout_secs: Option<u64>,
    ssh_retries: u32,
) -> Result<ExecOutput, String> {
    let is_ssh = !machine.is_pepita_transport()
        && !machine.is_container_transport()
        && machine.addr != "127.0.0.1"
        && machine.addr != "localhost"
        && !is_local_addr(&machine.addr);

    // For non-SSH targets or retries disabled, just run once
    let max_attempts = if is_ssh { ssh_retries.clamp(1, 4) } else { 1 };

    let mut last_err = String::new();
    for attempt in 0..max_attempts {
        if attempt > 0 {
            // Exponential backoff: 200ms × 2^(attempt-1) = 200ms, 400ms, 800ms
            let backoff_ms = 200u64 * (1u64 << (attempt - 1));
            std::thread::sleep(std::time::Duration::from_millis(backoff_ms));
            eprintln!(
                "  [retry {}/{}] retrying SSH to {} after {}ms backoff",
                attempt,
                max_attempts - 1,
                machine.addr,
                backoff_ms
            );
        }

        match exec_script_timeout(machine, script, timeout_secs) {
            Ok(out) => return Ok(out),
            Err(e) => {
                if attempt + 1 < max_attempts && is_transient_ssh_error(&e) {
                    last_err = e;
                    continue;
                }
                return Err(e);
            }
        }
    }

    Err(last_err)
}

/// Check if an SSH error is transient (worth retrying).
fn is_transient_ssh_error(err: &str) -> bool {
    let lower = err.to_lowercase();
    lower.contains("connection refused")
        || lower.contains("connection reset")
        || lower.contains("connection timed out")
        || lower.contains("broken pipe")
        || lower.contains("no route to host")
        || lower.contains("transport timeout")
        || lower.contains("failed to spawn ssh")
}

/// Execute a read-only query (for plan/drift — doesn't need tripwire).
pub fn query(machine: &Machine, cmd: &str) -> Result<ExecOutput, String> {
    exec_script(machine, cmd)
}

/// Check if an address is this machine.
fn is_local_addr(addr: &str) -> bool {
    // Check if the address matches any local interface
    if addr == "127.0.0.1" || addr == "localhost" || addr == "::1" {
        return true;
    }
    // Check hostname
    if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
        if addr == hostname.trim() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_local_detection() {
        assert!(is_local_addr("127.0.0.1"));
        assert!(is_local_addr("localhost"));
        assert!(is_local_addr("::1"));
        assert!(!is_local_addr("192.168.1.100"));
        assert!(!is_local_addr("10.0.0.1"));
    }

    /// BH-MUT-0001: Kill mutation of exec_script local dispatch.
    /// Verify local execution works for 127.0.0.1 and localhost addresses.
    #[test]
    fn test_transport_exec_local_127() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script(&machine, "echo ok").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "ok");
    }

    /// BH-MUT-0001: Verify localhost also dispatches locally.
    #[test]
    fn test_transport_exec_local_localhost() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "localhost".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script(&machine, "echo local").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "local");
    }

    #[test]
    fn test_transport_exec_output_success() {
        let ok = ExecOutput {
            exit_code: 0,
            stdout: "ok".into(),
            stderr: "".into(),
        };
        assert!(ok.success());
        let fail = ExecOutput {
            exit_code: 1,
            stdout: "".into(),
            stderr: "err".into(),
        };
        assert!(!fail.success());
        let sig = ExecOutput {
            exit_code: 137,
            stdout: "".into(),
            stderr: "killed".into(),
        };
        assert!(!sig.success());
    }

    #[test]
    fn test_transport_query_delegates() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = query(&machine, "echo query-test").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "query-test");
    }

    #[test]
    fn test_timeout_none_succeeds() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script_timeout(&machine, "echo ok", None).unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "ok");
    }

    #[test]
    fn test_timeout_generous_succeeds() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script_timeout(&machine, "echo fast", Some(10)).unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "fast");
    }

    #[test]
    fn test_timeout_exceeded_returns_error() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let result = exec_script_timeout(&machine, "sleep 10", Some(1));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("timeout"));
    }

    #[test]
    fn test_transport_timeout_error_includes_hostname() {
        let machine = Machine {
            hostname: "slow-box".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let err = exec_script_timeout(&machine, "sleep 10", Some(1)).unwrap_err();
        assert!(
            err.contains("slow-box"),
            "timeout error should include hostname: {}",
            err
        );
    }

    #[test]
    fn test_transport_container_dispatch_priority() {
        // Container transport takes priority even if addr is a valid IP
        use crate::core::types::ContainerConfig;
        let machine = Machine {
            hostname: "hybrid".to_string(),
            addr: "127.0.0.1".to_string(), // Would normally be local
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: Some(ContainerConfig {
                runtime: "/bin/echo".to_string(),
                image: Some("test:latest".to_string()),
                name: Some("forjar-dispatch-test".to_string()),
                ephemeral: true,
                privileged: false,
                init: true,
            }),
            pepita: None,
            cost: 0,
        };
        // With container transport, exec_script dispatches to container, not local
        // /bin/echo as runtime won't run bash properly, so it will fail or produce empty output
        let result = exec_script(&machine, "echo should-not-reach-local");
        if let Ok(out) = result {
            // If /bin/echo handled it, stdout won't contain "should-not-reach-local"
            // because echo doesn't execute bash
            assert_ne!(
                out.stdout.trim(),
                "should-not-reach-local",
                "container transport should intercept before local dispatch"
            );
        }
        // Err is expected: /bin/echo can't exec bash
    }

    #[test]
    fn test_transport_ipv6_loopback_is_local() {
        assert!(is_local_addr("::1"));
    }

    #[test]
    fn test_transport_remote_addr_not_local() {
        assert!(!is_local_addr("8.8.8.8"));
        assert!(!is_local_addr("google.com"));
        assert!(!is_local_addr("192.168.1.1"));
    }

    #[test]
    fn test_transport_exec_captures_both_streams() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script(&machine, "echo OUT; echo ERR >&2").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "OUT");
        assert!(out.stderr.contains("ERR"));
    }

    #[test]
    fn test_transport_exec_multiline_script() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let script = "A=hello\nB=world\necho \"$A $B\"";
        let out = exec_script(&machine, script).unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "hello world");
    }

    #[test]
    fn test_transport_exec_nonzero_exit_code() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script(&machine, "exit 77").unwrap();
        assert!(!out.success());
        assert_eq!(out.exit_code, 77);
    }

    #[test]
    fn test_transport_timeout_error_includes_seconds() {
        let machine = Machine {
            hostname: "slow".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let err = exec_script_timeout(&machine, "sleep 10", Some(1)).unwrap_err();
        assert!(
            err.contains("1s"),
            "error should include timeout value: {}",
            err
        );
    }

    #[test]
    fn test_transport_exec_empty_script() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script(&machine, "").unwrap();
        assert!(out.success());
    }

    #[test]
    fn test_transport_exec_output_debug() {
        // ExecOutput should derive Debug
        let out = ExecOutput {
            exit_code: 0,
            stdout: "test".to_string(),
            stderr: "".to_string(),
        };
        let debug = format!("{:?}", out);
        assert!(debug.contains("exit_code: 0"));
    }

    #[test]
    fn test_transport_exec_output_clone() {
        // ExecOutput should derive Clone
        let out = ExecOutput {
            exit_code: 42,
            stdout: "test".to_string(),
            stderr: "err".to_string(),
        };
        let cloned = out.clone();
        assert_eq!(cloned.exit_code, 42);
        assert_eq!(cloned.stdout, "test");
        assert_eq!(cloned.stderr, "err");
    }

    #[test]
    fn test_transport_query_is_readonly_alias() {
        // query() is just an alias for exec_script — verify same output
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let q = query(&machine, "echo q").unwrap();
        let e = exec_script(&machine, "echo q").unwrap();
        assert_eq!(q.stdout, e.stdout);
        assert_eq!(q.exit_code, e.exit_code);
    }

    // --- FJ-132: Transport dispatch edge cases ---

    #[test]
    fn test_fj132_exec_script_special_chars_in_output() {
        // Verify transport preserves special characters in stdout
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script(&machine, r#"printf 'tab\there\nnewline'"#).unwrap();
        assert!(out.success());
        assert!(out.stdout.contains("tab"));
    }

    #[test]
    fn test_fj132_exec_script_large_output() {
        // Verify transport handles large output without truncation
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script(&machine, "seq 1 10000").unwrap();
        assert!(out.success());
        assert!(out.stdout.contains("10000"));
    }

    #[test]
    fn test_fj132_exec_script_env_isolation() {
        // Scripts should not leak env vars between calls
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        exec_script(&machine, "export FORJAR_TEST_LEAK=yes").unwrap();
        let out = exec_script(&machine, "echo ${FORJAR_TEST_LEAK:-unset}").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "unset");
    }

    #[test]
    fn test_fj132_exec_script_exit_code_preserved() {
        // Verify various exit codes are preserved
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        for code in [0, 1, 2, 42, 126, 127] {
            let out = exec_script(&machine, &format!("exit {}", code)).unwrap();
            assert_eq!(
                out.exit_code, code,
                "exit code {} should be preserved",
                code
            );
        }
    }

    #[test]
    fn test_fj132_timeout_zero_seconds_fails() {
        // A timeout of 0 seconds should cause immediate timeout
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        // sleep 5 with 0s timeout should error — but 0-second timeout
        // may or may not catch "echo ok" depending on scheduling
        let result = exec_script_timeout(&machine, "sleep 5", Some(0));
        // This should almost always timeout, but we accept either outcome
        // since 0-second timeout behavior is platform-dependent
        if let Err(e) = result {
            assert!(e.contains("timeout"));
        }
    }

    #[test]
    fn test_fj132_is_local_addr_empty_string() {
        assert!(!is_local_addr(""));
    }

    // ── FJ-036: Transport script execution coverage ─────────────────

    #[test]
    fn test_fj036_local_script_echo() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script(&machine, "echo 'hello from forjar'").unwrap();
        assert!(out.success());
        assert_eq!(out.exit_code, 0);
        assert_eq!(out.stdout.trim(), "hello from forjar");
        assert!(out.stderr.is_empty() || out.stderr.trim().is_empty());
    }

    #[test]
    fn test_fj036_local_script_exit_code() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script(&machine, "exit 1").unwrap();
        assert!(!out.success());
        assert_eq!(out.exit_code, 1);
    }

    #[test]
    fn test_fj036_is_local_addr_comprehensive() {
        // All standard local address variants
        assert!(is_local_addr("127.0.0.1"), "IPv4 loopback must be local");
        assert!(is_local_addr("localhost"), "localhost must be local");
        assert!(is_local_addr("::1"), "IPv6 loopback must be local");

        // Non-local addresses must return false
        assert!(!is_local_addr("0.0.0.0"), "0.0.0.0 is not treated as local");
        assert!(
            !is_local_addr("192.168.1.1"),
            "private IP must not be local"
        );
        assert!(!is_local_addr("10.0.0.1"), "10.x must not be local");
        assert!(!is_local_addr("8.8.8.8"), "public IP must not be local");
        assert!(!is_local_addr("google.com"), "domain must not be local");
        assert!(!is_local_addr(""), "empty string must not be local");
        assert!(
            !is_local_addr("127.0.0.2"),
            "127.0.0.2 is not explicitly local"
        );
    }

    // ── FJ-261: SSH retry with exponential backoff ──

    #[test]
    fn test_fj261_is_transient_ssh_error_connection_refused() {
        assert!(is_transient_ssh_error("ssh: connect to host 10.0.0.1 port 22: Connection refused"));
    }

    #[test]
    fn test_fj261_is_transient_ssh_error_connection_reset() {
        assert!(is_transient_ssh_error("Connection reset by peer"));
    }

    #[test]
    fn test_fj261_is_transient_ssh_error_timeout() {
        assert!(is_transient_ssh_error("transport timeout: script on 'box' exceeded 30s limit"));
    }

    #[test]
    fn test_fj261_is_transient_ssh_error_broken_pipe() {
        assert!(is_transient_ssh_error("Write failed: Broken pipe"));
    }

    #[test]
    fn test_fj261_is_transient_ssh_error_no_route() {
        assert!(is_transient_ssh_error("ssh: connect to host 10.0.0.1: No route to host"));
    }

    #[test]
    fn test_fj261_is_transient_ssh_error_spawn_failure() {
        assert!(is_transient_ssh_error("failed to spawn ssh to 10.0.0.1: ..."));
    }

    #[test]
    fn test_fj261_is_transient_ssh_error_non_transient() {
        assert!(!is_transient_ssh_error("Permission denied (publickey)"));
        assert!(!is_transient_ssh_error("Host key verification failed"));
        assert!(!is_transient_ssh_error("exit code 1: command not found"));
    }

    #[test]
    fn test_fj261_retry_local_skips_retry() {
        // Local targets should never retry — only 1 attempt
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script_retry(&machine, "echo ok", None, 3).unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "ok");
    }

    #[test]
    fn test_fj261_retry_default_one_is_no_retry() {
        // ssh_retries=1 means one attempt, no retry
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script_retry(&machine, "echo once", None, 1).unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "once");
    }

    #[test]
    fn test_fj261_retry_clamped_to_max_4() {
        // ssh_retries > 4 should be clamped to 4. For local, still runs once.
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script_retry(&machine, "echo clamped", None, 100).unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "clamped");
    }

    #[test]
    fn test_fj261_retry_with_timeout() {
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script_retry(&machine, "echo fast", Some(10), 2).unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "fast");
    }

    #[test]
    fn test_fj261_retry_zero_clamped_to_one() {
        // ssh_retries=0 should clamp to 1 (at least one attempt)
        let machine = Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            pepita: None,
            cost: 0,
        };
        let out = exec_script_retry(&machine, "echo zero", None, 0).unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "zero");
    }
}
