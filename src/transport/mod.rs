//! FJ-010/011/021/230: Transport abstraction — local, SSH, container, and pepita execution.

pub mod container;
pub mod local;
pub mod pepita;
pub mod ssh;

#[cfg(test)]
mod tests_dispatch;
#[cfg(test)]
mod tests_dispatch_b;
#[cfg(test)]
mod tests_ssh;
#[cfg(test)]
mod tests_container;
#[cfg(test)]
mod tests_container_b;
#[cfg(test)]
mod tests_container_c;

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
