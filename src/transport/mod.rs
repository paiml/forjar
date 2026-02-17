//! FJ-010/011: Transport abstraction — local and SSH execution.

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
/// Dispatches to local or SSH based on address.
pub fn exec_script(machine: &Machine, script: &str) -> Result<ExecOutput, String> {
    let is_local =
        machine.addr == "127.0.0.1" || machine.addr == "localhost" || is_local_addr(&machine.addr);

    if is_local {
        local::exec_local(script)
    } else {
        ssh::exec_ssh(machine, script)
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
        };
        let out = exec_script(&machine, "echo local").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "local");
    }

    #[test]
    fn test_transport_exec_output_success() {
        let ok = ExecOutput { exit_code: 0, stdout: "ok".into(), stderr: "".into() };
        assert!(ok.success());
        let fail = ExecOutput { exit_code: 1, stdout: "".into(), stderr: "err".into() };
        assert!(!fail.success());
        let sig = ExecOutput { exit_code: 137, stdout: "".into(), stderr: "killed".into() };
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
        };
        let out = query(&machine, "echo query-test").unwrap();
        assert!(out.success());
        assert_eq!(out.stdout.trim(), "query-test");
    }
}
