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
        .map_err(|e| {
            format!(
                "failed to exec in container '{}': {}",
                container_name, e
            )
        })?;

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
        .map_err(|e| format!("failed to inspect container '{}': {}", container_name, e))?;

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

    args.push(image);
    args.push("sleep");
    args.push("infinity");

    let run = Command::new(&config.runtime)
        .args(&args)
        .output()
        .map_err(|e| format!("failed to start container '{}': {}", container_name, e))?;

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
        .map_err(|e| format!("failed to remove container '{}': {}", container_name, e))?;

    if !rm.status.success() {
        return Err(format!(
            "failed to remove container '{}': {}",
            container_name,
            String::from_utf8_lossy(&rm.stderr).trim()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::ContainerConfig;

    fn container_machine() -> Machine {
        Machine {
            hostname: "test-box".to_string(),
            addr: "container".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: Some(ContainerConfig {
                runtime: "docker".to_string(),
                image: Some("ubuntu:22.04".to_string()),
                name: Some("forjar-unit-test".to_string()),
                ephemeral: true,
                privileged: false,
                init: true,
            }),
        }
    }

    #[test]
    fn test_fj021_exec_no_container_config() {
        let machine = Machine {
            hostname: "bad".to_string(),
            addr: "container".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: None,
        };
        let result = exec_container(&machine, "echo hi");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no container config"));
    }

    #[test]
    fn test_fj021_ensure_no_container_config() {
        let machine = Machine {
            hostname: "bad".to_string(),
            addr: "container".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: None,
        };
        let result = ensure_container(&machine);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj021_cleanup_no_container_config() {
        let machine = Machine {
            hostname: "bad".to_string(),
            addr: "container".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: None,
        };
        let result = cleanup_container(&machine);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj021_container_name_from_config() {
        let m = container_machine();
        assert_eq!(m.container_name(), "forjar-unit-test");
    }

    #[test]
    fn test_fj021_ensure_no_image() {
        let machine = Machine {
            hostname: "no-image".to_string(),
            addr: "container".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: Some("container".to_string()),
            container: Some(ContainerConfig {
                runtime: "docker".to_string(),
                image: None,
                name: Some("forjar-no-image".to_string()),
                ephemeral: true,
                privileged: false,
                init: true,
            }),
        };
        // ensure_container on a non-existent container with no image should fail
        // (unless the container already exists, which it won't in unit tests)
        let result = ensure_container(&machine);
        // This will either fail because docker isn't available or because no image
        assert!(result.is_err());
    }
}
