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

/// Build the SSH command arguments (without spawning).
/// Useful for testing and debugging.
fn build_ssh_args(machine: &Machine) -> Vec<String> {
    let mut args = vec![
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "ConnectTimeout=5".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
    ];

    if let Some(ref key) = machine.ssh_key {
        let expanded = if let Some(rest) = key.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                format!("{}/{}", home, rest)
            } else {
                key.clone()
            }
        } else {
            key.clone()
        };
        args.push("-i".to_string());
        args.push(expanded);
    }

    args.push(format!("{}@{}", machine.user, machine.addr));
    args.push("bash".to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Machine;

    fn make_machine(addr: &str, user: &str, ssh_key: Option<&str>) -> Machine {
        Machine {
            hostname: "test-host".to_string(),
            addr: addr.to_string(),
            user: user.to_string(),
            arch: "x86_64".to_string(),
            ssh_key: ssh_key.map(|s| s.to_string()),
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        }
    }

    #[test]
    fn test_fj011_ssh_key_expansion() {
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

    #[test]
    fn test_fj011_ssh_key_expansion_no_tilde() {
        // Absolute path should be returned unchanged
        let key = "/home/deploy/.ssh/id_ed25519";
        let expanded = if let Some(rest) = key.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                format!("{}/{}", home, rest)
            } else {
                key.to_string()
            }
        } else {
            key.to_string()
        };
        assert_eq!(expanded, "/home/deploy/.ssh/id_ed25519");
    }

    #[test]
    fn test_fj011_build_args_basic() {
        let m = make_machine("10.0.0.1", "root", None);
        let args = build_ssh_args(&m);
        assert!(args.contains(&"BatchMode=yes".to_string()));
        assert!(args.contains(&"ConnectTimeout=5".to_string()));
        assert!(args.contains(&"StrictHostKeyChecking=accept-new".to_string()));
        assert!(args.contains(&"root@10.0.0.1".to_string()));
        assert!(args.contains(&"bash".to_string()));
        // No -i flag without ssh_key
        assert!(!args.contains(&"-i".to_string()));
    }

    #[test]
    fn test_fj011_build_args_with_key() {
        let m = make_machine("10.0.0.1", "deploy", Some("/home/deploy/.ssh/id_ed25519"));
        let args = build_ssh_args(&m);
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"/home/deploy/.ssh/id_ed25519".to_string()));
        assert!(args.contains(&"deploy@10.0.0.1".to_string()));
    }

    #[test]
    fn test_fj011_build_args_with_tilde_key() {
        let m = make_machine("10.0.0.1", "admin", Some("~/.ssh/id_rsa"));
        let args = build_ssh_args(&m);
        // Should have expanded ~ to $HOME
        let key_idx = args.iter().position(|a| a == "-i").unwrap();
        let key_path = &args[key_idx + 1];
        assert!(!key_path.starts_with('~'), "tilde should be expanded");
        assert!(key_path.ends_with(".ssh/id_rsa"));
    }

    #[test]
    fn test_fj011_build_args_user_at_host_format() {
        let m = make_machine("web.example.com", "deployer", None);
        let args = build_ssh_args(&m);
        assert!(args.contains(&"deployer@web.example.com".to_string()));
    }

    #[test]
    fn test_fj011_build_args_ipv6() {
        let m = make_machine("::1", "root", None);
        let args = build_ssh_args(&m);
        assert!(args.contains(&"root@::1".to_string()));
    }

    #[test]
    fn test_fj011_exec_output_captures_nonzero_exit() {
        // The ExecOutput correctly represents non-zero exit codes
        let output = super::ExecOutput {
            exit_code: 127,
            stdout: String::new(),
            stderr: "command not found".to_string(),
        };
        assert!(!output.success());
        assert_eq!(output.exit_code, 127);
    }

    #[test]
    fn test_fj011_exec_output_captures_signal() {
        // Killed by signal (e.g., OOM kill)
        let output = super::ExecOutput {
            exit_code: -1,
            stdout: String::new(),
            stderr: "killed".to_string(),
        };
        assert!(!output.success());
        assert_eq!(output.exit_code, -1);
    }

    #[test]
    fn test_fj011_build_args_order() {
        // SSH options must come before user@host
        let m = make_machine("10.0.0.5", "root", Some("/root/.ssh/key"));
        let args = build_ssh_args(&m);
        let batch_idx = args.iter().position(|a| a == "BatchMode=yes").unwrap();
        let user_idx = args.iter().position(|a| a == "root@10.0.0.5").unwrap();
        let bash_idx = args.iter().position(|a| a == "bash").unwrap();
        assert!(batch_idx < user_idx, "options must come before user@host");
        assert!(user_idx < bash_idx, "user@host must come before bash");
    }

    #[test]
    fn test_fj011_stdin_piping_design() {
        // Verify that exec_ssh uses stdin piping (not -c argument)
        // This is a design invariant: stdin avoids argument length limits and injection
        let m = make_machine("10.0.0.1", "root", None);
        let args = build_ssh_args(&m);
        // The last arg should be "bash" (not a script), confirming stdin pipe design
        assert_eq!(args.last().unwrap(), "bash");
        // No -c flag present
        assert!(!args.contains(&"-c".to_string()));
    }

    #[test]
    fn test_fj011_build_args_count_without_key() {
        // Without key: -o BatchMode=yes -o ConnectTimeout=5 -o StrictHostKeyChecking=accept-new user@host bash
        let m = make_machine("10.0.0.1", "root", None);
        let args = build_ssh_args(&m);
        assert_eq!(args.len(), 8, "6 option args + user@host + bash");
    }

    #[test]
    fn test_fj011_build_args_count_with_key() {
        // With key adds -i <keypath> = 2 more args
        let m = make_machine("10.0.0.1", "root", Some("/root/.ssh/key"));
        let args = build_ssh_args(&m);
        assert_eq!(args.len(), 10, "6 option args + -i key + user@host + bash");
    }

    #[test]
    fn test_fj011_batch_mode_prevents_password_prompt() {
        // BatchMode=yes is critical: prevents SSH from prompting for passwords
        // which would hang the process in non-interactive mode
        let m = make_machine("10.0.0.1", "root", None);
        let args = build_ssh_args(&m);
        let batch_pos = args.iter().position(|a| a == "BatchMode=yes").unwrap();
        // Must be preceded by -o
        assert_eq!(args[batch_pos - 1], "-o");
    }

    #[test]
    fn test_fj011_connect_timeout_value() {
        // ConnectTimeout=5 prevents hangs on unreachable machines
        let m = make_machine("10.0.0.1", "root", None);
        let args = build_ssh_args(&m);
        assert!(
            args.contains(&"ConnectTimeout=5".to_string()),
            "must set 5s connect timeout"
        );
    }

    #[test]
    fn test_fj011_strict_host_key_accept_new() {
        // accept-new: accept first-time keys, reject changed keys
        let m = make_machine("10.0.0.1", "root", None);
        let args = build_ssh_args(&m);
        assert!(args.contains(&"StrictHostKeyChecking=accept-new".to_string()));
    }

    #[test]
    fn test_fj011_dns_hostname_addr() {
        let m = make_machine("db.prod.internal", "admin", None);
        let args = build_ssh_args(&m);
        assert!(args.contains(&"admin@db.prod.internal".to_string()));
    }

    #[test]
    fn test_fj011_nonstandard_user() {
        let m = make_machine("10.0.0.1", "deploy-bot", None);
        let args = build_ssh_args(&m);
        assert!(args.contains(&"deploy-bot@10.0.0.1".to_string()));
    }

    #[test]
    fn test_fj011_build_args_all_options_are_dash_o() {
        // Every SSH option must be preceded by -o
        let m = make_machine("10.0.0.1", "root", None);
        let args = build_ssh_args(&m);
        let option_values = ["BatchMode=yes", "ConnectTimeout=5", "StrictHostKeyChecking=accept-new"];
        for opt in &option_values {
            let pos = args.iter().position(|a| a == opt).unwrap();
            assert_eq!(args[pos - 1], "-o", "option '{}' must be preceded by -o", opt);
        }
    }

    #[test]
    fn test_fj011_build_args_key_before_user_host() {
        // -i key must come before user@host
        let m = make_machine("10.0.0.1", "root", Some("/root/.ssh/key"));
        let args = build_ssh_args(&m);
        let key_idx = args.iter().position(|a| a == "-i").unwrap();
        let user_idx = args.iter().position(|a| a == "root@10.0.0.1").unwrap();
        assert!(key_idx < user_idx, "-i must come before user@host");
    }

    #[test]
    fn test_fj011_build_args_bash_is_last() {
        // "bash" must always be the last argument
        let m = make_machine("10.0.0.1", "root", Some("/root/.ssh/key"));
        let args = build_ssh_args(&m);
        assert_eq!(args.last().unwrap(), "bash");
    }

    #[test]
    fn test_fj011_exec_output_success_zero() {
        let output = ExecOutput {
            exit_code: 0,
            stdout: "ok".to_string(),
            stderr: String::new(),
        };
        assert!(output.success());
    }

    #[test]
    fn test_fj011_key_expansion_relative_path() {
        // A relative path (not starting with ~ or /) should be unchanged
        let m = make_machine("10.0.0.1", "root", Some("keys/deploy.pem"));
        let args = build_ssh_args(&m);
        assert!(args.contains(&"keys/deploy.pem".to_string()));
    }
}
