//! FJ-49: Bootstrap a new machine for forjar management.
//!
//! Handles the zero-to-managed onboarding:
//! 1. Copy SSH public key (via sshpass + ssh-copy-id if password provided)
//! 2. Configure passwordless sudo for the user
//! 3. Verify: ssh key auth + sudo -n works

use std::process::Command;

/// Run the bootstrap sequence for a new machine.
pub(crate) fn cmd_bootstrap(
    addr: &str,
    user: &str,
    password_stdin: bool,
    ssh_key_path: Option<&str>,
    hostname: Option<&str>,
    skip_key_if_working: bool,
) -> Result<(), String> {
    // Step 0: Check if key auth already works
    if skip_key_if_working && key_auth_works(addr, user) {
        eprintln!("SSH key auth already works for {user}@{addr}, skipping key copy");
    } else {
        // Step 1: Copy SSH public key
        copy_ssh_key(addr, user, password_stdin, ssh_key_path)?;
    }

    // Step 2: Verify key auth works
    if !key_auth_works(addr, user) {
        return Err(format!(
            "SSH key auth failed for {user}@{addr} — check your key and try again"
        ));
    }
    eprintln!("SSH key auth: OK");

    // Step 3: Configure passwordless sudo (if not root)
    if user != "root" {
        configure_sudo(addr, user)?;
    }

    // Step 4: Verify sudo
    if user != "root" && !sudo_works(addr, user) {
        return Err(format!(
            "passwordless sudo verification failed for {user}@{addr}"
        ));
    }
    eprintln!("Passwordless sudo: OK");

    // Step 5: Set hostname (optional)
    if let Some(name) = hostname {
        set_hostname(addr, user, name)?;
    }

    println!("Machine {user}@{addr} bootstrapped. Run: forjar apply -f <config.yaml>");
    Ok(())
}

/// Check if SSH key auth works (no password needed).
fn key_auth_works(addr: &str, user: &str) -> bool {
    Command::new("ssh")
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "StrictHostKeyChecking=accept-new",
            &format!("{user}@{addr}"),
            "true",
        ])
        .output()
        .is_ok_and(|out| out.status.success())
}

/// Copy SSH public key to the remote machine.
fn copy_ssh_key(
    addr: &str,
    user: &str,
    password_stdin: bool,
    ssh_key_path: Option<&str>,
) -> Result<(), String> {
    let pub_key = resolve_pub_key(ssh_key_path)?;

    if password_stdin {
        // Read password from stdin
        let password = read_password_stdin()?;
        // Use sshpass + ssh-copy-id
        let status = Command::new("sshpass")
            .args([
                "-p",
                &password,
                "ssh-copy-id",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-i",
                &pub_key,
                &format!("{user}@{addr}"),
            ])
            .status()
            .map_err(|e| format!("sshpass not found (install: apt install sshpass): {e}"))?;
        if !status.success() {
            return Err("ssh-copy-id failed — check password and connectivity".to_string());
        }
    } else {
        // Try ssh-copy-id without password (user types it interactively)
        let status = Command::new("ssh-copy-id")
            .args([
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-i",
                &pub_key,
                &format!("{user}@{addr}"),
            ])
            .status()
            .map_err(|e| format!("ssh-copy-id failed: {e}"))?;
        if !status.success() {
            return Err("ssh-copy-id failed".to_string());
        }
    }
    eprintln!("SSH key copied to {user}@{addr}");
    Ok(())
}

/// Resolve the path to the SSH public key.
fn resolve_pub_key(ssh_key_path: Option<&str>) -> Result<String, String> {
    if let Some(path) = ssh_key_path {
        return Ok(path.to_string());
    }
    // Try ed25519 first, then rsa
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let ed25519 = format!("{home}/.ssh/id_ed25519.pub");
    if std::path::Path::new(&ed25519).exists() {
        return Ok(ed25519);
    }
    let rsa = format!("{home}/.ssh/id_rsa.pub");
    if std::path::Path::new(&rsa).exists() {
        return Ok(rsa);
    }
    Err("no SSH public key found (~/.ssh/id_ed25519.pub or ~/.ssh/id_rsa.pub) — generate one with: ssh-keygen -t ed25519".to_string())
}

/// Read password from stdin (single line, no echo).
fn read_password_stdin() -> Result<String, String> {
    let mut password = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut password)
        .map_err(|e| format!("failed to read password from stdin: {e}"))?;
    Ok(password.trim().to_string())
}

/// Configure passwordless sudo for a user via SSH.
fn configure_sudo(addr: &str, user: &str) -> Result<(), String> {
    let sudoers_line = format!("{user} ALL=(ALL) NOPASSWD:ALL");
    let sudoers_file = format!("/etc/sudoers.d/{user}-nopasswd");
    // Use tee with sudo (user may have partial sudo with password)
    let script = format!(
        "echo '{sudoers_line}' | sudo tee '{sudoers_file}' > /dev/null && sudo chmod 0440 '{sudoers_file}'"
    );
    let output = Command::new("ssh")
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            &format!("{user}@{addr}"),
            &script,
        ])
        .output()
        .map_err(|e| format!("SSH failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("sudo configuration failed: {stderr}"));
    }
    eprintln!("Passwordless sudo configured for {user}");
    Ok(())
}

/// Verify passwordless sudo works.
fn sudo_works(addr: &str, user: &str) -> bool {
    Command::new("ssh")
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            &format!("{user}@{addr}"),
            "sudo -n true",
        ])
        .output()
        .is_ok_and(|out| out.status.success())
}

/// Set hostname on the remote machine.
fn set_hostname(addr: &str, user: &str, hostname: &str) -> Result<(), String> {
    let script = format!(
        "sudo hostnamectl set-hostname '{hostname}' 2>/dev/null || echo '{hostname}' | sudo tee /etc/hostname > /dev/null"
    );
    let output = Command::new("ssh")
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            &format!("{user}@{addr}"),
            &script,
        ])
        .output()
        .map_err(|e| format!("set hostname failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("set hostname failed: {stderr}"));
    }
    eprintln!("Hostname set to {hostname}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_pub_key_explicit() {
        let result = resolve_pub_key(Some("/tmp/test.pub"));
        assert_eq!(result.unwrap(), "/tmp/test.pub");
    }

    #[test]
    fn test_resolve_pub_key_default() {
        // Should find a key or return an error (never panic)
        let _result = resolve_pub_key(None);
    }

    #[test]
    fn test_key_auth_nonexistent_host() {
        // Should return false, not panic
        assert!(!key_auth_works("192.168.255.254", "nobody"));
    }
}
