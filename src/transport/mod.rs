//! FJ-010/011/021/230: Transport abstraction — local, SSH, container, and pepita execution.

pub mod container;
pub mod local;
pub mod pepita;
pub mod ssh;
pub mod stdin_isolation;

#[cfg(test)]
mod tests_container;
#[cfg(test)]
mod tests_container_b;
#[cfg(test)]
mod tests_container_c;
#[cfg(test)]
mod tests_container_d;
#[cfg(test)]
mod tests_dispatch;
#[cfg(test)]
mod tests_dispatch_b;
#[cfg(test)]
mod tests_i8_diagnosis;
#[cfg(test)]
mod tests_ssh;
#[cfg(test)]
mod tests_timeout;

use crate::core::types::Machine;

/// FJ-#154: Shared slot for publishing a spawned child's PID to the timeout
/// path. `exec_script_timeout` populates this from a worker thread so it can
/// kill the underlying ssh/bash/docker/nsenter process when the timeout fires,
/// instead of leaking a detached thread blocked in `wait_with_output` on a
/// still-alive child.
pub(crate) type ChildPidSlot = std::sync::Arc<std::sync::Mutex<Option<u32>>>;

/// Record a freshly-spawned child's PID into the (optional) tracking slot.
pub(crate) fn record_child_pid(slot: Option<&ChildPidSlot>, child: &std::process::Child) {
    if let Some(slot) = slot {
        if let Ok(mut guard) = slot.lock() {
            *guard = Some(child.id());
        }
    }
}

/// #165: Put the spawned child in its own process group so a timeout kill can
/// target the *group* (`kill -9 -- -<pgid>`) rather than a bare PID that the OS
/// may have already recycled for an unrelated process after the child was
/// reaped. On Unix the child becomes the group leader, so the group id equals
/// the child PID we record. A no-op on non-Unix targets.
#[cfg(unix)]
pub(crate) fn configure_process_group(cmd: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

/// Non-Unix fallback: process groups are not available, so this is a no-op.
#[cfg(not(unix))]
pub(crate) fn configure_process_group(_cmd: &mut std::process::Command) {}

/// #165: Build the `kill` command used on the timeout path.
///
/// The child was spawned in its own process group (see
/// [`configure_process_group`]), so we signal the whole group with the
/// negative-PID convention (`kill -9 -- -<pgid>`). Because the child is the
/// group leader, the group id equals the recorded PID. Killing the group
/// reaches descendants (e.g. the remote shell ssh spawned), and — crucially —
/// the leading `-` means we can never accidentally signal a single recycled
/// PID: a process-group target only matches processes the child fathered.
/// Factored out so the argument construction is unit-testable without spawning.
fn kill_group_command(pid: u32) -> std::process::Command {
    let mut cmd = std::process::Command::new("kill");
    cmd.args(["-9", "--", &format!("-{pid}")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    cmd
}

/// Best-effort kill of a child's whole process group via the `kill` binary.
///
/// Used only on the timeout path, and only when the worker has *not* yet
/// reported a result (i.e. the child is provably still running). Targeting the
/// process group instead of a bare PID closes the PID-reuse race: a reaped
/// child's recycled PID can never belong to the dead group, so we cannot kill
/// an unrelated process.
fn kill_process_group(pid: u32) {
    let _ = kill_group_command(pid).status();
}

/// FJ-#154: Write a script to a child's stdin, reaping the child on failure.
///
/// If the child dies before consuming stdin (ssh auth fails instantly, remote
/// bash unavailable, broken pipe / EPIPE), `write_all` returns Err. The old
/// code `?`-returned here, dropping the `Child` WITHOUT `wait()`, which on Unix
/// leaves a zombie. This helper kills + reaps the child before returning the
/// error so no zombie survives. Passing the script through a function that owns
/// the `&mut Child` keeps the reap guaranteed on every early return.
pub(crate) fn write_stdin_or_reap(
    child: &mut std::process::Child,
    script: &str,
) -> Result<(), String> {
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(script.as_bytes()) {
            // Drop our stdin handle first (close the pipe), then kill + reap so
            // the child cannot become a zombie on this early return.
            drop(stdin);
            kill_and_reap(child);
            return Err(format!("stdin write error: {e}"));
        }
        // Explicitly close stdin so the child sees EOF before we wait.
        drop(stdin);
    }
    Ok(())
}

/// Kill a child and wait for it, so no zombie survives. Best-effort: both the
/// kill and the wait are allowed to fail (e.g. the child already exited).
pub(crate) fn kill_and_reap(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Output from executing a script on a target.
#[derive(Debug, Clone)]
pub struct ExecOutput {
    /// Process exit code.
    pub exit_code: i32,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

impl ExecOutput {
    /// Returns true if the process exited with code 0.
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// FJ-1357: Validate script via bashrs before execution (I8 enforcement gate).
///
/// FJ-29: Strip opaque data payloads before linting. Two patterns:
/// 1. Base64 blobs from `source:` file resources — binary data in single quotes
/// 2. Heredoc payloads from `content:` file resources — user content between delimiters
///
/// Both contain data that bashrs misinterprets as shell syntax. The data is never
/// executed as shell — it is written to files via pipe or heredoc redirection.
fn validate_before_exec(script: &str) -> Result<(), String> {
    let sanitised = strip_data_payloads(script);
    crate::core::purifier::validate_script(&sanitised).map_err(|e| {
        format!(
            "I8 violation — script failed bashrs validation: {e}\n\n{}",
            numbered_for_diagnosis(&sanitised)
        )
    })
}

/// Render the exact text bashrs judged, with line numbers.
///
/// forjar#281: the sanitised script was built, linted, and thrown away, so an
/// I8 rejection named a rule and a line number for text nobody could see. That
/// matters here more than in a normal linter, because the rejected script is
/// GENERATED — `strip_data_payloads` rewrites it through four regex passes, and
/// a `completion_check` written as a YAML folded scalar arrives already
/// collapsed onto one line. The offending text existed in neither the YAML the
/// author wrote nor in bashrs; forjar manufactured it, and then hid it.
///
/// Line numbers are what make the diagnostic actionable: bashrs reports the
/// line it rejected, and this is the only rendering of the file that has those
/// lines.
fn numbered_for_diagnosis(sanitised: &str) -> String {
    let mut out = String::from(
        "--- the script bashrs judged (after forjar's own data-payload stripping) ---\n",
    );
    for (i, line) in sanitised.lines().enumerate() {
        out.push_str(&format!("{:>4} | {}\n", i + 1, line));
    }
    out.push_str("--- end ---");
    out
}

/// Strip opaque data payloads that bashrs should not lint.
///
/// Handles four forjar codegen patterns:
/// 1. `echo '<base64>' | base64 -d > '<path>'` — binary file deployment
/// 2. `cat > '<path>' <<'FORJAR_EOF'\n...\nFORJAR_EOF` — text file deployment
/// 3. Copia delta patch temp file operations (`rm -f "$TMPFILE"`, `mv "$TMPFILE" ...`)
///    which use absolute paths that trigger bashrs SEC010 false positives
/// 4. Cargo cache staging operations (`cp`, `mkdir -p`, `rm -rf` with `_STAGING`/`_CACHE_DIR`)
///    which are safe by construction but trigger SEC010/SEC011 false positives
fn strip_data_payloads(script: &str) -> String {
    // Phase 1: strip base64 blobs
    let re_b64 = regex::Regex::new(r"echo '([A-Za-z0-9+/=\n]+)' \| base64 -d > '([^']+)'")
        .expect("base64 regex is valid");
    let pass1 = re_b64
        .replace_all(script, "echo 'FORJAR_BASE64_STRIPPED' > '$2'")
        .into_owned();

    // Phase 2: strip heredoc payloads (FORJAR_EOF delimiters)
    let re_heredoc =
        regex::Regex::new(r"(?s)<<'FORJAR_EOF'\n.*?\nFORJAR_EOF").expect("heredoc regex is valid");
    let pass2 = re_heredoc
        .replace_all(
            &pass1,
            "<<'FORJAR_EOF'\n# payload stripped for lint\nFORJAR_EOF",
        )
        .into_owned();

    // Phase 3: strip forjar-generated copia delta temp file operations
    // These use TMPFILE variable for atomic file replacement and always
    // reference absolute paths, which triggers bashrs SEC010 false positives.
    let re_copia_rm = regex::Regex::new(r#"rm -f "\$TMPFILE""#).expect("copia rm regex is valid");
    let pass3 = re_copia_rm
        .replace_all(&pass2, "# forjar-copia: tmpfile cleanup stripped")
        .into_owned();
    let re_copia_mv =
        regex::Regex::new(r#"mv "\$TMPFILE" '[^']+'"#).expect("copia mv regex is valid");
    let pass4 = re_copia_mv
        .replace_all(&pass3, "# forjar-copia: atomic replace stripped")
        .into_owned();

    // Phase 4: strip forjar-generated cargo cache staging operations
    // These use _STAGING, _CACHE_DIR, _CARGO_BIN variables that are safe by
    // construction (mktemp -d, derived from $HOME) but trigger SEC010/SEC011.
    // Match any line containing cp/mkdir/rm with these internal variables.
    let re_cargo_ops = regex::Regex::new(
        r#"(?m)^\s*(?:cp|mkdir -p|rm -rf?)\s+.*\$_(?:STAGING|CACHE_DIR|CARGO_BIN).*$"#,
    )
    .expect("cargo ops regex is valid");
    re_cargo_ops
        .replace_all(&pass4, "# forjar-cargo: cache op stripped")
        .into_owned()
}

/// Execute a purified shell script on a machine.
/// Dispatches to pepita, container, local, or SSH based on transport/address.
/// Priority: pepita > container > local > SSH.
///
/// I8 invariant: script is validated via bashrs before any execution.
pub fn exec_script(machine: &Machine, script: &str) -> Result<ExecOutput, String> {
    exec_script_tracked(machine, script, None)
}

/// Dispatch core for [`exec_script`] with an optional child-PID tracking slot.
///
/// `pid_slot`, when present, receives the spawned transport child's PID so the
/// timeout path can kill it (FJ-#154). Public `exec_script` passes `None`.
fn exec_script_tracked(
    machine: &Machine,
    script: &str,
    pid_slot: Option<&ChildPidSlot>,
) -> Result<ExecOutput, String> {
    validate_before_exec(script)?;

    // FJ-2732: the script travels on the target shell'"'"'s STDIN, so a command
    // that reads stdin would consume the rest of its own script. Wrap once, at
    // the single funnel every transport passes through.
    let script = &stdin_isolation::wrap_script_stdin_isolated(script);

    // Pepita (kernel namespace) transport takes highest priority
    if machine.is_pepita_transport() {
        return pepita::exec_pepita(machine, script, pid_slot);
    }

    // Container transport takes priority over local/SSH
    if machine.is_container_transport() {
        return container::exec_container(machine, script, pid_slot);
    }

    let is_local = machine_is_local(machine);

    if is_local {
        local::exec_local(script, pid_slot)
    } else {
        ssh::exec_ssh(machine, script, pid_slot)
    }
}

/// Execute a script with an optional timeout (in seconds).
/// Returns an error if the script exceeds the timeout.
///
/// FJ-#154 / #165: On timeout the underlying transport child's *process group*
/// is killed (the group leader's PID is published into a shared slot by the
/// worker). Killing the group unblocks the worker's `wait_with_output`, so the
/// child is reaped and neither the child process nor the worker thread leaks.
///
/// #165: The kill is gated on `worker_done` — an `AtomicBool` the worker raises
/// the instant before it returns its result. If the worker already finished
/// (child reaped, PID potentially recycled by the OS), we do NOT signal, so a
/// recycled PID can never be killed. After signalling we `join` the worker so
/// the child is provably reaped before we return.
pub fn exec_script_timeout(
    machine: &Machine,
    script: &str,
    timeout_secs: Option<u64>,
) -> Result<ExecOutput, String> {
    let Some(secs) = timeout_secs else {
        return exec_script(machine, script);
    };

    let hostname = machine.hostname.clone();
    let machine = machine.clone();
    let script = script.to_string();
    let pid_slot: ChildPidSlot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let worker_slot = pid_slot.clone();
    let worker_done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_done_w = worker_done.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    let handle = std::thread::spawn(move || {
        let result = exec_script_tracked(&machine, &script, Some(&worker_slot));
        // Mark done BEFORE sending so the timeout path observes completion as
        // soon as a result is available — the child is already reaped here.
        worker_done_w.store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(std::time::Duration::from_secs(secs)) {
        Ok(result) => result,
        Err(_) => {
            kill_worker_child_group(&pid_slot, &worker_done);
            // Join the worker so its `wait_with_output` finishes reaping the
            // child before we return (no leaked thread, no zombie).
            let _ = handle.join();
            Err(format!(
                "transport timeout: script on '{hostname}' exceeded {secs}s limit"
            ))
        }
    }
}

/// #165: Timeout-kill helper. Only signals the child's process group when the
/// worker has *not* yet finished (`worker_done == false`); otherwise the child
/// was already reaped and its PID may have been recycled, so signalling would
/// be unsafe. Returns whether a kill was actually issued (for tests).
fn kill_worker_child_group(
    pid_slot: &ChildPidSlot,
    worker_done: &std::sync::atomic::AtomicBool,
) -> bool {
    if worker_done.load(std::sync::atomic::Ordering::SeqCst) {
        return false;
    }
    if let Some(pid) = pid_slot.lock().ok().and_then(|g| *g) {
        kill_process_group(pid);
        return true;
    }
    false
}

/// Check if a machine uses SSH transport (not pepita, container, or local).
pub fn is_ssh_transport(machine: &Machine) -> bool {
    !machine.is_pepita_transport()
        && !machine.is_container_transport()
        && machine.addr != "127.0.0.1"
        && machine.addr != "localhost"
        && !is_local_addr(&machine.addr)
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
    let is_ssh = is_ssh_transport(machine);
    let max_attempts = if is_ssh { ssh_retries.clamp(1, 4) } else { 1 };

    let mut last_err = String::new();
    for attempt in 0..max_attempts {
        if attempt > 0 {
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
///
/// I8 invariant: query command is validated via bashrs before execution.
pub fn query(machine: &Machine, cmd: &str) -> Result<ExecOutput, String> {
    // exec_script already validates, but we gate here explicitly for
    // defense-in-depth in case query ever takes a different path.
    validate_before_exec(cmd)?;
    exec_script(machine, cmd)
}

/// FJ-2710 (PMAT-197): does this machine refer to the host forjar runs on?
///
/// Exposed so the build-I/O probe can refuse to hash the controller's
/// filesystem on behalf of a remote target. One definition, one place — the
/// exec path and the probe path must never disagree about what "local" means.
pub fn machine_is_local(machine: &Machine) -> bool {
    !machine.is_container_transport()
        && (machine.addr == "127.0.0.1"
            || machine.addr == "localhost"
            || is_local_addr(&machine.addr))
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
