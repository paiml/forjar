//! FJ-230: Pepita kernel namespace transport.
//!
//! Executes scripts inside isolated Linux kernel namespaces using
//! `unshare(1)` + `nsenter(1)`. Same mechanism as container/SSH/local:
//! pipe purified shell script to `bash` stdin inside the namespace.
//!
//! Requires `CAP_SYS_ADMIN` or root. Zero Docker dependency — uses
//! kernel primitives directly (CLONE_NEWPID | CLONE_NEWNET | CLONE_NEWNS).

use super::ExecOutput;
use crate::core::types::Machine;
use std::io::Write;
use std::process::{Command, Stdio};

/// Execute a shell script inside a pepita kernel namespace.
///
/// Uses `nsenter` to enter the namespace identified by the PID file,
/// then pipes the script to `bash` stdin.
pub fn exec_pepita(machine: &Machine, script: &str) -> Result<ExecOutput, String> {
    let config = machine
        .pepita
        .as_ref()
        .ok_or_else(|| format!("machine '{}' has no pepita config", machine.hostname))?;

    let ns_name = machine.pepita_name();
    let pidfile = format!("/run/forjar/{}.pid", ns_name);

    // Read the PID of the namespace init process
    let pid = std::fs::read_to_string(&pidfile)
        .map_err(|e| format!("cannot read pidfile '{}': {} — is the namespace running?", pidfile, e))?
        .trim()
        .to_string();

    let mut args = vec![
        "--target".to_string(),
        pid,
        "--mount".to_string(),
        "--pid".to_string(),
    ];

    // Join network namespace if isolated
    if config.network == "isolated" {
        args.push("--net".to_string());
    }

    args.extend(["--".to_string(), "bash".to_string()]);

    let mut child = Command::new("nsenter")
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to nsenter namespace '{}': {}", ns_name, e))?;

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

/// Create a new kernel namespace for the given machine.
///
/// Uses `unshare(1)` to create isolated PID + mount + (optionally) network namespaces.
/// The init process runs `sleep infinity` as PID 1 inside the namespace.
/// The actual PID is written to `/run/forjar/<name>.pid`.
pub fn ensure_namespace(machine: &Machine) -> Result<(), String> {
    let config = machine
        .pepita
        .as_ref()
        .ok_or_else(|| format!("machine '{}' has no pepita config", machine.hostname))?;

    let ns_name = machine.pepita_name();
    let pidfile = format!("/run/forjar/{}.pid", ns_name);

    // Check if namespace already exists
    if std::path::Path::new(&pidfile).exists() {
        if let Ok(pid) = std::fs::read_to_string(&pidfile) {
            let pid = pid.trim();
            if std::path::Path::new(&format!("/proc/{}", pid)).exists() {
                return Ok(()); // Already running
            }
        }
    }

    // Create /run/forjar directory
    std::fs::create_dir_all("/run/forjar")
        .map_err(|e| format!("cannot create /run/forjar: {}", e))?;

    // Build unshare command
    let mut args = vec!["--fork", "--pid", "--mount"];

    if config.network == "isolated" {
        args.push("--net");
    }

    // Mount proc for PID namespace visibility
    args.push("--mount-proc");

    args.extend(["--", "sleep", "infinity"]);

    let child = Command::new("unshare")
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to create namespace '{}': {}", ns_name, e))?;

    // Write PID file
    std::fs::write(&pidfile, child.id().to_string())
        .map_err(|e| format!("cannot write pidfile '{}': {}", pidfile, e))?;

    // Apply cgroup limits if configured
    if config.memory_mb.is_some() || config.cpus.is_some() {
        apply_cgroup_limits(&ns_name, config.memory_mb, config.cpus)?;
    }

    Ok(())
}

/// Apply cgroup v2 limits to a namespace.
fn apply_cgroup_limits(
    ns_name: &str,
    memory_mb: Option<u64>,
    cpus: Option<f64>,
) -> Result<(), String> {
    let cgroup_path = format!("/sys/fs/cgroup/forjar/{}", ns_name);

    std::fs::create_dir_all(&cgroup_path)
        .map_err(|e| format!("cannot create cgroup '{}': {}", cgroup_path, e))?;

    if let Some(mb) = memory_mb {
        let bytes = mb * 1024 * 1024;
        std::fs::write(format!("{}/memory.max", cgroup_path), bytes.to_string())
            .map_err(|e| format!("cannot set memory limit: {}", e))?;
    }

    if let Some(cpus) = cpus {
        // cpu.max format: "quota period" in microseconds
        let period = 100_000u64; // 100ms
        let quota = (cpus * period as f64) as u64;
        std::fs::write(
            format!("{}/cpu.max", cgroup_path),
            format!("{} {}", quota, period),
        )
        .map_err(|e| format!("cannot set cpu limit: {}", e))?;
    }

    Ok(())
}

/// Teardown a pepita kernel namespace.
///
/// Kills the init process (which tears down all namespaces), removes the PID file,
/// and cleans up cgroup directory.
pub fn cleanup_namespace(machine: &Machine) -> Result<(), String> {
    let _config = machine
        .pepita
        .as_ref()
        .ok_or_else(|| format!("machine '{}' has no pepita config", machine.hostname))?;

    let ns_name = machine.pepita_name();
    let pidfile = format!("/run/forjar/{}.pid", ns_name);

    // Kill the init process if the pidfile exists
    if let Ok(pid_str) = std::fs::read_to_string(&pidfile) {
        let pid = pid_str.trim();
        // Send SIGKILL to the init process — this tears down the entire namespace
        let _ = Command::new("kill")
            .args(["-9", pid])
            .output();
    }

    // Remove pidfile
    let _ = std::fs::remove_file(&pidfile);

    // Remove cgroup directory
    let cgroup_path = format!("/sys/fs/cgroup/forjar/{}", ns_name);
    let _ = std::fs::remove_dir_all(&cgroup_path);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::PepitaTransportConfig;

    fn pepita_machine() -> Machine {
        Machine {
            hostname: "test-ns".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
           pepita: Some(PepitaTransportConfig {
                rootfs: "debootstrap:jammy".to_string(),
                memory_mb: Some(512),
                cpus: Some(2.0),
                network: "isolated".to_string(),
                filesystem: "overlay".to_string(),
                ephemeral: true,
            }),
            cost: 0,
        }
    }

    #[test]
    fn test_fj230_exec_no_pepita_config() {
        let machine = Machine {
            hostname: "bad".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
           pepita: None,
            cost: 0,
        };
        let result = exec_pepita(&machine, "echo hi");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no pepita config"));
    }

    #[test]
    fn test_fj230_ensure_no_pepita_config() {
        let machine = Machine {
            hostname: "bad".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
           pepita: None,
            cost: 0,
        };
        let result = ensure_namespace(&machine);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no pepita config"));
    }

    #[test]
    fn test_fj230_cleanup_no_pepita_config() {
        let machine = Machine {
            hostname: "bad".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("pepita".to_string()),
            container: None,
           pepita: None,
            cost: 0,
        };
        let result = cleanup_namespace(&machine);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no pepita config"));
    }

    #[test]
    fn test_fj230_pepita_name_derivation() {
        let m = pepita_machine();
        assert_eq!(m.pepita_name(), "forjar-ns-test-ns");
    }

    #[test]
    fn test_fj230_is_pepita_transport() {
        let m = pepita_machine();
        assert!(m.is_pepita_transport());
        assert!(!m.is_container_transport());
    }

    #[test]
    fn test_fj230_is_pepita_transport_by_addr() {
        let machine = Machine {
            hostname: "ns-box".to_string(),
            addr: "pepita".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
           pepita: Some(PepitaTransportConfig {
                rootfs: "/opt/rootfs".to_string(),
                memory_mb: None,
                cpus: None,
                network: "host".to_string(),
                filesystem: "bind".to_string(),
                ephemeral: false,
            }),
            cost: 0,
        };
        assert!(machine.is_pepita_transport());
    }

    #[test]
    fn test_fj230_pepita_config_defaults() {
        let yaml = r#"
rootfs: "debootstrap:jammy"
"#;
        let config: PepitaTransportConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.rootfs, "debootstrap:jammy");
        assert_eq!(config.network, "isolated");
        assert_eq!(config.filesystem, "overlay");
        assert!(config.ephemeral);
        assert!(config.memory_mb.is_none());
        assert!(config.cpus.is_none());
    }

    #[test]
    fn test_fj230_exec_missing_pidfile() {
        let m = pepita_machine();
        let result = exec_pepita(&m, "echo hi");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("pidfile") || err.contains("namespace"),
            "error should mention pidfile: {}",
            err
        );
    }

    #[test]
    fn test_fj230_cleanup_nonexistent_succeeds() {
        // Cleanup of a non-running namespace should succeed silently
        let m = pepita_machine();
        let result = cleanup_namespace(&m);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj230_pepita_yaml_parsing() {
        let yaml = r#"
version: "1.0"
name: pepita-test
machines:
  ns-box:
    hostname: ns-box
    addr: pepita
    transport: pepita
    pepita:
      rootfs: "debootstrap:jammy"
      memory_mb: 1024
      cpus: 4.0
      network: isolated
      filesystem: overlay
      ephemeral: true
resources:
  f:
    type: file
    machine: ns-box
    path: /tmp/test.txt
    content: "hello"
"#;
        let config: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let m = &config.machines["ns-box"];
        assert!(m.is_pepita_transport());
        assert_eq!(m.pepita.as_ref().unwrap().rootfs, "debootstrap:jammy");
        assert_eq!(m.pepita.as_ref().unwrap().memory_mb, Some(1024));
        assert_eq!(m.pepita.as_ref().unwrap().cpus, Some(4.0));
    }
}
