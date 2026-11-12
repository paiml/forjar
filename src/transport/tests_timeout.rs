//! #165 (#3): process-group kill on timeout — PID-reuse safety tests.
//!
//! These are hermetic and deterministic: no real ssh, no long-running command,
//! and no test that could hang the suite. The dangerous "kill a recycled PID"
//! race is closed by (a) killing the child's process GROUP via the negative-PID
//! convention and (b) gating the kill on the worker not having finished. Both
//! are unit-tested here without spawning long-lived processes.

use super::*;

/// #3, #992: the timeout kill signals the child's process GROUP, by syscall. A
/// real `sleep 30` in its own group dies of SIGKILL at once, with no external
/// `kill` binary involved (`kill_process_group` spawns nothing).
#[cfg(unix)]
#[test]
fn test_992_kill_process_group_kills_a_live_group_by_syscall() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};
    let mut cmd = Command::new("sleep");
    cmd.arg("30").stdout(Stdio::null()).stderr(Stdio::null());
    configure_process_group(&mut cmd);
    let mut child = cmd.spawn().expect("spawn sleep");
    let start = std::time::Instant::now();
    assert!(kill_process_group(child.id()), "signal was not delivered");
    let status = child.wait().expect("wait");
    assert_eq!(status.signal(), Some(9), "child must die of SIGKILL");
    assert!(start.elapsed().as_secs() < 10, "kill did not end the child");
}

/// #992: group 0 is the caller's own group and must never be signalled; an
/// absent group reports "not delivered" instead of failing silently.
#[test]
fn test_992_kill_process_group_refuses_zero_and_reports_absent() {
    assert!(!kill_process_group(0));
    assert!(!kill_process_group(u32::MAX));
    assert!(!kill_process_group(2_000_000_000));
    assert!(!kill_pid(0));
    assert!(!kill_pid(1));
    assert!(!kill_pid(u32::MAX));
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

/// Timeout that fires BEFORE the worker has spawned its child (heavily loaded host: observed
/// infra#3590, forjar apply ran 30s under `--resource-timeout 2`). The marker left in the empty
/// slot must make the late child kill itself, so the join is bounded by the limit, not the script.
#[test]
fn a_timeout_before_the_child_spawns_still_kills_it() {
    let slot: ChildPidSlot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let done = std::sync::atomic::AtomicBool::new(false);
    assert!(
        !kill_worker_child_group(&slot, &done),
        "nothing to kill yet"
    );
    let start = std::time::Instant::now();
    let out = local::exec_local("sleep 30", Some(&slot));
    assert!(start.elapsed().as_secs() < 10, "late child was not killed");
    // Killed before or after the stdin write: an Err or a non-success exit, never a clean 30s run.
    assert!(out.map(|o| !o.success()).unwrap_or(true));
}
