//! #165 (#3): process-group kill on timeout — PID-reuse safety tests.
//!
//! These are hermetic and deterministic: no real ssh, no long-running command,
//! and no test that could hang the suite. The dangerous "kill a recycled PID"
//! race is closed by (a) killing the child's process GROUP via the negative-PID
//! convention and (b) gating the kill on the worker not having finished. Both
//! are unit-tested here without spawning long-lived processes.

use super::*;

/// #3: the timeout kill must target the child's process GROUP, not a bare PID.
/// We assert the constructed `kill` command uses the negative-PID convention
/// (`kill -9 -- -<pid>`) so a recycled bare PID can never be signalled.
#[test]
fn test_165_kill_group_command_targets_group() {
    let cmd = kill_group_command(4242);
    let prog = cmd.get_program().to_string_lossy().to_string();
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    assert_eq!(prog, "kill");
    // -9 (SIGKILL), -- (end of options), -4242 (negative = the process group).
    assert_eq!(args, vec!["-9", "--", "-4242"]);
}

/// #3: when the worker has ALREADY finished (child reaped, PID possibly
/// recycled), the timeout path must NOT issue a kill.
#[test]
fn test_165_no_kill_when_worker_already_done() {
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    let slot: ChildPidSlot = Arc::new(Mutex::new(Some(4242)));
    // Worker finished -> the PID may already be recycled, so we must NOT kill.
    let done = AtomicBool::new(true);
    assert!(!kill_worker_child_group(&slot, &done));
}

/// #3: when the worker is still running and a PID was recorded, the timeout
/// path issues a kill against the group. We record a harmless high PID so the
/// real `kill` is a no-op (its group does not exist) but the gate still fires.
#[test]
fn test_165_kill_when_worker_still_running() {
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    let slot: ChildPidSlot = Arc::new(Mutex::new(Some(2_000_000_000)));
    let done = AtomicBool::new(false); // worker still blocked in wait_with_output
    assert!(kill_worker_child_group(&slot, &done));
}

/// #3: no recorded PID yet (child not spawned) -> nothing to kill.
#[test]
fn test_165_no_kill_when_no_pid_recorded() {
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    let slot: ChildPidSlot = Arc::new(Mutex::new(None));
    let done = AtomicBool::new(false);
    assert!(!kill_worker_child_group(&slot, &done));
}

/// #3: configuring a child's process group then spawning must still produce a
/// working child whose output we can collect (regression: the group setup must
/// not break normal local exec). On Unix the child becomes its own group leader.
#[cfg(unix)]
#[test]
fn test_165_configure_process_group_child_still_runs() {
    use std::process::{Command, Stdio};
    let mut cmd = Command::new("echo");
    cmd.arg("grouped")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    configure_process_group(&mut cmd);
    let out = cmd.output().expect("spawn echo");
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "grouped");
}

/// #3: the worker IS joined on timeout so the child is provably reaped (no
/// leaked thread). Exercise the real timeout path with a fast local command and
/// a generous timeout so it returns Ok — the symmetric "join after kill" path
/// is covered by the kill-gate unit tests above without risking a hang.
#[test]
fn test_165_timeout_path_fast_command_ok() {
    let m = Machine {
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
        allowed_operators: vec![],
    };
    let out = exec_script_timeout(&m, "echo quick", Some(30)).expect("fast command ok");
    assert!(out.success());
    assert_eq!(out.stdout.trim(), "quick");
}

/// #969: the timeout fires BEFORE the child spawned (slot still empty). The kill helper
/// must leave a cancel marker, and the spawner's `record_child_pid` must then kill the
/// child it just started, so the elapsed bound holds whatever the spawn latency.
/// RED on the unfixed code (the kill was a no-op and the child ran its full sleep).
#[cfg(unix)]
#[test]
fn test_969_timeout_before_spawn_kills_late_child() {
    use std::process::{Command, Stdio};
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    let slot: ChildPidSlot = Arc::new(Mutex::new(None));
    let done = AtomicBool::new(false);
    // Timeout fires with no PID published yet: nothing to signal, but a marker is left.
    assert!(!kill_worker_child_group(&slot, &done));
    assert_eq!(*slot.lock().unwrap(), Some(PID_CANCELLED));
    // The delayed spawn now happens (own process group, as in exec_local).
    let mut cmd = Command::new("sleep");
    cmd.arg("30").stdout(Stdio::null()).stderr(Stdio::null());
    configure_process_group(&mut cmd);
    let mut child = cmd.spawn().unwrap();
    let t = std::time::Instant::now();
    record_child_pid(Some(&slot), &child);
    let status = child.wait().unwrap();
    assert!(!status.success(), "late child must have been killed");
    assert!(
        t.elapsed().as_secs() < 5,
        "kill must be immediate, took {:?}",
        t.elapsed()
    );
}

/// #969: the cancel sentinel is never handed to `kill` (`kill -9 -- -0` would signal our own group).
#[test]
fn test_969_cancel_marker_is_not_killed_again() {
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};
    let slot: ChildPidSlot = Arc::new(Mutex::new(Some(PID_CANCELLED)));
    assert!(!kill_worker_child_group(&slot, &AtomicBool::new(false)));
}
