//! Child process-group tracking for the timeout path (#154, #165, infra#3590):
//! the PID slot a worker publishes into, and the group kill the timeout issues.

/// FJ-#154: Shared slot for publishing a spawned child's PID to the timeout
/// path. `exec_script_timeout` populates this from a worker thread so it can
/// kill the underlying ssh/bash/docker/nsenter process when the timeout fires,
/// instead of leaking a detached thread blocked in `wait_with_output` on a
/// still-alive child.
pub(crate) type ChildPidSlot = std::sync::Arc<std::sync::Mutex<Option<u32>>>;

/// Marker the timeout path leaves in an EMPTY slot: "timed out before the child was spawned".
/// PID 0 is never a real child, and group 0 (the caller's own) must never be signalled.
const CANCELLED_PID: u32 = 0;

/// Record a freshly-spawned child's PID into the (optional) tracking slot.
pub(crate) fn record_child_pid(slot: Option<&ChildPidSlot>, child: &std::process::Child) {
    if let Some(slot) = slot {
        if let Ok(mut guard) = slot.lock() {
            // The timeout fired before this child existed: nobody else will kill it, so kill its
            // group now or the timeout's join waits out the whole script (infra#3590).
            if *guard == Some(CANCELLED_PID) {
                kill_process_group(child.id());
            }
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

/// #165, #992: signal a child's whole process group with SIGKILL, by syscall.
///
/// The child was spawned in its own process group (see
/// [`configure_process_group`]) and is the group leader, so the group id equals
/// the recorded PID. Killing the group reaches descendants (e.g. the remote
/// shell ssh spawned), and a process-group target can never match a single
/// recycled PID: it only matches processes the child fathered.
///
/// #992: this used to spawn the external `kill` binary with its error
/// discarded. An image without procps (`rust:1.95-slim`) has no such binary,
/// so the spawn failed silently, nothing was killed, and a 2 s
/// `--resource-timeout` waited out a 30 s script. `killpg(2)` needs no binary.
///
/// Returns whether the signal was delivered. PID 0 (the caller's own group)
/// and values that do not fit a `pid_t` are refused, never signalled.
#[cfg(unix)]
pub(super) fn kill_process_group(pid: u32) -> bool {
    use nix::sys::signal::{killpg, Signal};
    use nix::unistd::Pid;
    match i32::try_from(pid) {
        Ok(raw) if raw > 0 => killpg(Pid::from_raw(raw), Signal::SIGKILL).is_ok(),
        _ => false,
    }
}

/// Non-Unix fallback: there are no process groups to signal.
#[cfg(not(unix))]
pub(super) fn kill_process_group(_pid: u32) -> bool {
    false
}

/// #992: SIGKILL one process by syscall (pepita namespace teardown). PIDs 0 and
/// 1 and values that do not fit a `pid_t` are refused. Returns whether the
/// signal was delivered.
#[cfg(unix)]
pub(crate) fn kill_pid(pid: u32) -> bool {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    match i32::try_from(pid) {
        Ok(raw) if raw > 1 => kill(Pid::from_raw(raw), Signal::SIGKILL).is_ok(),
        _ => false,
    }
}

/// Non-Unix fallback: no signal is sent.
#[cfg(not(unix))]
pub(crate) fn kill_pid(_pid: u32) -> bool {
    false
}

/// #165: Timeout-kill helper. Only signals the child's process group when the
/// worker has *not* yet finished (`worker_done == false`); otherwise the child
/// was already reaped and its PID may have been recycled, so signalling would
/// be unsafe. Returns whether a kill was actually issued (for tests).
pub(super) fn kill_worker_child_group(
    pid_slot: &ChildPidSlot,
    worker_done: &std::sync::atomic::AtomicBool,
) -> bool {
    if worker_done.load(std::sync::atomic::Ordering::SeqCst) {
        return false;
    }
    if let Ok(mut guard) = pid_slot.lock() {
        match *guard {
            Some(pid) if pid != CANCELLED_PID => {
                kill_process_group(pid);
                return true;
            }
            // Worker has not spawned yet: leave a marker so record_child_pid kills the child on arrival.
            None => *guard = Some(CANCELLED_PID),
            Some(_) => {}
        }
    }
    false
}
