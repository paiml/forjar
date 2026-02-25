//! FJ-010/011/021: Transport abstraction — local, SSH, and container execution.

pub mod container;
pub mod local;
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
/// Dispatches to container, local, or SSH based on transport/address.
pub fn exec_script(machine: &Machine, script: &str) -> Result<ExecOutput, String> {
    // Container transport takes priority
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
            cost: 0,
        };
        // With container transport, exec_script dispatches to container, not local
        // /bin/echo as runtime won't run bash properly, so it will fail or produce empty output
        let result = exec_script(&machine, "echo should-not-reach-local");
        match result {
            Ok(out) => {
                // If /bin/echo handled it, stdout won't contain "should-not-reach-local"
                // because echo doesn't execute bash
                assert_ne!(
                    out.stdout.trim(),
                    "should-not-reach-local",
                    "container transport should intercept before local dispatch"
                );
            }
            Err(_) => {} // Expected: /bin/echo can't exec bash
        }
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
            cost: 0,
        };
        let out = exec_script(&machine, "exit 77").unwrap();
        assert!(!out.success());
        assert_eq!(out.exit_code, 77);
    }
}
