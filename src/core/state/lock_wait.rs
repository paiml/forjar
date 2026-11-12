//! FJ-LOCK-001 C5 (#925): the FIFO queue behind a held lock.
//!
//! A waiter drops a ticket file `<lock>.q.<unix-nanos>.<pid>.<id>` beside the
//! lock and only the lexicographically first ticket whose owner is alive may
//! try to take the lock, so waiters are served in arrival order and a late
//! waiter cannot overtake an earlier one. The order holds among queued callers
//! only: a caller with a zero timeout takes no ticket and tries once, so it can
//! win a just-released lock ahead of the queue. Tickets of dead processes are
//! swept as they are met, so a killed waiter never blocks the queue.

use std::path::{Path, PathBuf};

fn prefix(lock_path: &Path) -> String {
    let name = lock_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!("{name}.q.")
}

/// Join the queue for `lock_path`; the returned ticket is removed by [`dequeue`].
pub(crate) fn enqueue(lock_path: &Path, id: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let ticket = lock_path.with_file_name(format!(
        "{}{nanos:030}.{}.{id}",
        prefix(lock_path),
        std::process::id()
    ));
    // A ticket we cannot write leaves us unqueued, which only costs fairness.
    let _ = std::fs::write(&ticket, std::process::id().to_string());
    ticket
}

/// Leave the queue.
pub(crate) fn dequeue(ticket: &Path) {
    let _ = std::fs::remove_file(ticket);
}

/// Is `ticket` the first live ticket for its lock?
pub(crate) fn is_head(ticket: &Path) -> bool {
    let (Some(dir), Some(name)) = (ticket.parent(), ticket.file_name()) else {
        return true;
    };
    let name = name.to_string_lossy();
    let Some(pfx) = name.split(".q.").next().map(|p| format!("{p}.q.")) else {
        return true;
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return true;
    };
    let mut live: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let n = entry.file_name().to_string_lossy().into_owned();
        if !n.starts_with(&pfx) {
            continue;
        }
        if ticket_pid(&n).is_some_and(|pid| !super::lock_info::is_pid_running(pid)) {
            let _ = std::fs::remove_file(entry.path());
            continue;
        }
        live.push(n);
    }
    live.sort();
    live.first().is_none_or(|first| *first == *name)
}

/// The pid segment of `<lock>.q.<nanos>.<pid>.<id>`.
fn ticket_pid(name: &str) -> Option<u32> {
    name.split(".q.").nth(1)?.split('.').nth(1)?.parse().ok()
}
