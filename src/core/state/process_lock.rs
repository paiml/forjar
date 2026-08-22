//! FJ-266: State locking — prevent concurrent applies.
//!
//! Acquisition is atomic via `OpenOptions::create_new` (a single O_EXCL
//! syscall), so two concurrent `forjar apply` processes can never both observe
//! the lock absent and both win. The loser gets `AlreadyExists` and evaluates
//! the stale-PID branch before retrying.
//!
//! #165 hardening:
//!   * the retry loop is BOUNDED (it previously looped unconditionally and could
//!     spin a core forever when a stale lock was observable but un-removable);
//!   * a confirmed-stale lock that cannot be removed propagates `Err` instead of
//!     being silently ignored;
//!   * the reap re-confirms the file still holds the same dead-PID content
//!     before unlinking (TOCTOU guard), so it can never delete a concurrent
//!     winner's freshly-acquired valid lock.

use std::path::{Path, PathBuf};

/// Path to the process lock file.
pub(crate) fn process_lock_path(state_dir: &Path) -> PathBuf {
    state_dir.join(".forjar.lock")
}

/// Acquire an exclusive process lock. Returns an error if another apply is running.
/// Stale locks (PID no longer running) are automatically removed.
///
/// FJ-266/#154: Acquisition is atomic. `OpenOptions::create_new` is a single
/// O_EXCL syscall, so two concurrent `forjar apply` processes can never both
/// observe the file absent and both win — the loser gets `AlreadyExists` and
/// then evaluates the stale-PID branch before retrying. The previous
/// exists→read→remove→write sequence had a TOCTOU window in which both
/// processes wrote the lock and ran concurrently, corrupting state.
pub fn acquire_process_lock(state_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(state_dir).map_err(|e| format!("cannot create state dir: {e}"))?;

    let lock_path = process_lock_path(state_dir);
    let content = process_lock_content();

    // #165: BOUND the loop. The previous code looped unconditionally; if a
    // stale lock could be observed (dead PID) but never removed (read-only
    // mount → EROFS, sticky-bit dir owned by another UID → EPERM), every
    // iteration was identical and the thread span a core forever. We cap the
    // retries and back off briefly between attempts, so acquisition can never
    // livelock — on exhaustion we surface the locked-by-PID-style error.
    let exhausted = || {
        format!(
            "could not acquire state lock {} after {} attempts",
            lock_path.display(),
            MAX_LOCK_ACQUIRE_ATTEMPTS
        )
    };
    for attempt in 0..MAX_LOCK_ACQUIRE_ATTEMPTS {
        match try_create_lock(&lock_path, &content) {
            Ok(()) => return Ok(()),
            Err(LockAcquireError::Io(e)) => return Err(e),
            // Lost the race / pre-existing lock: evaluate staleness atomically.
            Err(LockAcquireError::AlreadyExists) => {
                match reap_or_reject_stale_lock(&lock_path)? {
                    // Stale lock removed (or vanished, or replaced by a fresh
                    // one) — retry the create_new, after a short backoff.
                    ReapOutcome::Retry => {}
                    // Held by a live PID — reject immediately, no point retrying.
                    ReapOutcome::HeldByLivePid(msg) => return Err(msg),
                }
            }
        }
        if attempt + 1 < MAX_LOCK_ACQUIRE_ATTEMPTS {
            std::thread::sleep(LOCK_ACQUIRE_BACKOFF);
        }
    }
    Err(exhausted())
}

/// #165: Cap on `acquire_process_lock` retries so it can never livelock.
const MAX_LOCK_ACQUIRE_ATTEMPTS: u32 = 5;

/// #165: Short backoff between lock-acquire attempts (also yields the CPU so a
/// transient contender / reap window can clear instead of busy-spinning).
const LOCK_ACQUIRE_BACKOFF: std::time::Duration = std::time::Duration::from_millis(50);

/// Build the lock-file content (our PID + start timestamp).
fn process_lock_content() -> String {
    format!(
        "pid: {}\nstarted_at: {}\n",
        std::process::id(),
        crate::tripwire::eventlog::now_iso8601()
    )
}

/// Outcome of an atomic lock-file creation attempt.
enum LockAcquireError {
    /// The lock file already exists (lost the race or a held/stale lock).
    AlreadyExists,
    /// A non-recoverable I/O error (already formatted for the caller).
    Io(String),
}

/// Atomically create the lock file with O_EXCL semantics.
fn try_create_lock(lock_path: &Path, content: &str) -> Result<(), LockAcquireError> {
    use std::io::Write;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)
    {
        Ok(mut f) => f
            .write_all(content.as_bytes())
            .map_err(|e| LockAcquireError::Io(format!("cannot write lock file: {e}"))),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(LockAcquireError::AlreadyExists)
        }
        Err(e) => Err(LockAcquireError::Io(format!(
            "cannot create lock file: {e}"
        ))),
    }
}

/// Outcome of evaluating a pre-existing lock file.
#[derive(Debug)]
enum ReapOutcome {
    /// The lock was stale (or vanished, or replaced) — the caller may retry the
    /// atomic create.
    Retry,
    /// The lock is held by a still-running PID — reject with this message.
    HeldByLivePid(String),
}

/// Whether the owning PID of a lock is live, dead (stale), or absent.
enum LockOwner {
    /// PID parsed and `/proc/<pid>` exists.
    Live(u32),
    /// PID parsed but not running, OR no PID parseable — safe to reap.
    Stale,
}

/// Classify a lock file's content into a [`LockOwner`]. Pure (modulo the
/// `is_running` probe) so the decision is unit-testable without real processes.
fn classify_lock_owner(content: &str, is_running: impl Fn(u32) -> bool) -> LockOwner {
    match parse_lock_pid(content) {
        Some(pid) if is_running(pid) => LockOwner::Live(pid),
        _ => LockOwner::Stale,
    }
}

/// Given an existing lock file, reject it if the owning PID is still running,
/// otherwise reap it so the caller can retry the atomic create.
///
/// #165 (#1): If the lock is stale but `remove_file` FAILS (read-only mount →
/// EROFS, cross-UID sticky-bit dir → EPERM), we now propagate `Err` instead of
/// ignoring the error and letting the acquire loop spin forever.
///
/// #165 (#2): TOCTOU guard — we re-read the file and only remove it if it still
/// holds the SAME dead-PID content we just classified as stale. If a concurrent
/// acquirer replaced it with a fresh (valid) lock in the meantime, the content
/// won't match and we leave it alone (returning `Retry`), so we can never
/// unlink a winner's freshly-acquired lock and let two applies run.
fn reap_or_reject_stale_lock(lock_path: &Path) -> Result<ReapOutcome, String> {
    // The file may have vanished between create_new and read (the holder
    // released it) — treat a missing file as "retry the create".
    let content = match std::fs::read_to_string(lock_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ReapOutcome::Retry),
        Err(e) => return Err(format!("cannot read lock file: {e}")),
    };

    if let LockOwner::Live(pid) = classify_lock_owner(&content, is_pid_running) {
        return Ok(ReapOutcome::HeldByLivePid(format!(
            "state directory is locked by PID {} ({}). \
             If this is stale, run: forjar apply --force-unlock",
            pid,
            lock_path.display()
        )));
    }

    // Stale. Re-confirm identity before unlinking (TOCTOU guard, #2).
    reap_stale_if_unchanged(lock_path, &content)
}

/// #165 (#2): Remove a stale lock only if it still holds `observed` content.
///
/// Re-reads the file: if it vanished, a concurrent acquirer already handled it
/// (`Retry`); if it now holds DIFFERENT content, a fresh valid lock was placed
/// there and we must NOT delete it (`Retry`, leaving the winner's lock intact);
/// only when the content is byte-identical to what we classified as stale do we
/// `remove_file`. A failed removal of a confirmed-stale lock is a hard error
/// (#1) rather than a silently-ignored result that would loop forever.
fn reap_stale_if_unchanged(lock_path: &Path, observed: &str) -> Result<ReapOutcome, String> {
    match std::fs::read_to_string(lock_path) {
        // Byte-identical to what we classified as stale — safe to unlink.
        Ok(current) if current == observed => unlink_confirmed_stale_lock(lock_path),
        // Content changed (fresh valid lock) — leave it; re-evaluate next loop.
        Ok(_) => Ok(ReapOutcome::Retry),
        // Vanished — already handled by someone else.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ReapOutcome::Retry),
        Err(e) => Err(format!("cannot re-read lock file: {e}")),
    }
}

/// Unlink a lock whose staleness the caller has just re-confirmed, and decide
/// what a failed removal means.
///
/// A concurrent reaper getting there first (`NotFound`) is success — the lock is
/// gone either way, so the caller may `Retry`. Any other failure is the #165
/// (#1) case: a confirmed-stale lock we cannot remove (EROFS, cross-UID EPERM)
/// would make the acquire loop spin, so it is a hard error carrying the
/// `--force-unlock` escape hatch. Exists to keep that removal policy in one
/// place, separate from the TOCTOU re-read that authorises it.
fn unlink_confirmed_stale_lock(lock_path: &Path) -> Result<ReapOutcome, String> {
    match std::fs::remove_file(lock_path) {
        Ok(()) => Ok(ReapOutcome::Retry),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ReapOutcome::Retry),
        Err(e) => Err(format!(
            "cannot remove stale lock {}: {} \
             (run: forjar apply --force-unlock)",
            lock_path.display(),
            e
        )),
    }
}

/// Release the process lock.
pub fn release_process_lock(state_dir: &Path) {
    let lock_path = process_lock_path(state_dir);
    let _ = std::fs::remove_file(&lock_path);
}

/// Force-remove the process lock (for --force-unlock).
pub fn force_unlock(state_dir: &Path) -> Result<(), String> {
    let lock_path = process_lock_path(state_dir);
    if !lock_path.exists() {
        return Ok(());
    }
    std::fs::remove_file(&lock_path).map_err(|e| format!("cannot remove lock file: {e}"))
}

/// Parse PID from lock file content.
pub(crate) fn parse_lock_pid(content: &str) -> Option<u32> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("pid:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

/// Check if a PID is still running (Linux-specific: /proc/<pid> exists).
fn is_pid_running(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(test)]
mod tests;
