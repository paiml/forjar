//! FJ-266: State locking — prevent concurrent applies.
//!
//! Acquisition is atomic via `link(2)` of a fully written staging file (as
//! exclusive as O_EXCL, and the lock never exists empty — FJ-LOCK-001 C1), so
//! two concurrent `forjar apply` processes can never both win. The loser gets
//! `AlreadyExists` and evaluates the stale-PID branch before retrying.
//!
//! #165 hardening:
//!   * the retry loop is BOUNDED (it previously looped unconditionally and could
//!     spin a core forever when a stale lock was observable but un-removable);
//!   * a confirmed-stale lock that cannot be removed propagates `Err` instead of
//!     being silently ignored;
//!   * the reap re-confirms the file still holds the same dead-PID content
//!     before unlinking (TOCTOU guard), so it can never delete a concurrent
//!     winner's freshly-acquired valid lock.

use super::lock_info::{is_pid_running, read_lock_info, Holder, LockInfo};
use super::lock_wait;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Path to the process lock file.
pub(crate) fn process_lock_path(state_dir: &Path) -> PathBuf {
    state_dir.join(".forjar.lock")
}

/// Acquire an exclusive process lock, failing fast if another apply holds it.
/// Stale locks (holder gone) are automatically removed.
///
/// FJ-266/#154: Acquisition is atomic (see the module doc). The lock is held
/// until [`release_process_lock`] or a [`ProcessLockGuard`] drops it.
pub fn acquire_process_lock(state_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(state_dir).map_err(|e| format!("cannot create state dir: {e}"))?;
    acquire_at(&process_lock_path(state_dir), Duration::ZERO).map(|_| ())
}

/// Acquire the lock file at `lock_path`, waiting up to `timeout` for a live
/// holder (FJ-LOCK-001 C5: `Duration::ZERO` fails fast, like Terraform's
/// `-lock-timeout=0s`). Waiters are served FIFO. Returns the identity written
/// into the lock, whose `id` is what release matches on.
pub(crate) fn acquire_at(lock_path: &Path, timeout: Duration) -> Result<LockInfo, String> {
    let info = LockInfo::current();
    let content = info.render();
    let deadline = Instant::now() + timeout;
    let ticket = (!timeout.is_zero()).then(|| lock_wait::enqueue(lock_path, &info.id));
    let outcome = acquire_loop(lock_path, &content, deadline, ticket.as_deref());
    if let Some(t) = ticket {
        lock_wait::dequeue(&t);
    }
    outcome.map(|()| info)
}

/// One bounded pass of the create / reap / wait loop.
fn acquire_loop(
    lock_path: &Path,
    content: &str,
    deadline: Instant,
    ticket: Option<&Path>,
) -> Result<(), String> {
    // #165: BOUND the reap retries. If a stale lock can be observed but never
    // removed (EROFS, cross-UID sticky dir) every iteration is identical and
    // the thread would spin a core forever; the cap makes that an error.
    let mut reap_attempts = 0u32;
    loop {
        // FIFO: only the head of the queue may attempt, so a late arrival
        // cannot take the lock from a waiter that has been queued longer.
        let my_turn = ticket.is_none_or(lock_wait::is_head);
        if my_turn {
            match try_create_lock(lock_path, content) {
                Ok(()) => return Ok(()),
                Err(LockAcquireError::Io(e)) => return Err(e),
                Err(LockAcquireError::AlreadyExists) => {
                    match reap_or_reject_stale_lock(lock_path)? {
                        ReapOutcome::Retry => {
                            reap_attempts += 1;
                            if reap_attempts >= MAX_LOCK_ACQUIRE_ATTEMPTS {
                                return Err(format!(
                                    "could not acquire state lock {} after {} attempts",
                                    lock_path.display(),
                                    MAX_LOCK_ACQUIRE_ATTEMPTS
                                ));
                            }
                            std::thread::sleep(LOCK_ACQUIRE_BACKOFF);
                            continue;
                        }
                        ReapOutcome::HeldByLivePid(msg) => {
                            if Instant::now() >= deadline {
                                return Err(timeout_message(msg, ticket.is_some()));
                            }
                        }
                    }
                }
            }
        } else if Instant::now() >= deadline {
            let holder = read_lock_info(lock_path).map(|i| i.describe());
            return Err(timeout_message(
                format!(
                    "state lock {} is queued behind earlier waiters (holder: {})",
                    lock_path.display(),
                    holder.unwrap_or_else(|| "none".into())
                ),
                true,
            ));
        }
        std::thread::sleep(LOCK_WAIT_POLL);
    }
}

fn timeout_message(msg: String, waited: bool) -> String {
    if waited {
        format!("{msg} [--lock-timeout expired while waiting]")
    } else {
        msg
    }
}

/// #165: Cap on stale-lock reap retries so acquisition can never livelock.
const MAX_LOCK_ACQUIRE_ATTEMPTS: u32 = 5;

/// #165: Short backoff between reap retries (yields the CPU instead of spinning).
const LOCK_ACQUIRE_BACKOFF: Duration = Duration::from_millis(50);

/// How often a waiting acquirer re-checks a live holder.
const LOCK_WAIT_POLL: Duration = Duration::from_millis(50);

/// Outcome of an atomic lock-file creation attempt.
enum LockAcquireError {
    /// The lock file already exists (lost the race or a held/stale lock).
    AlreadyExists,
    /// A non-recoverable I/O error (already formatted for the caller).
    Io(String),
}

/// Atomically create the lock file with O_EXCL semantics.
fn try_create_lock(lock_path: &Path, content: &str) -> Result<(), LockAcquireError> {
    try_create_lock_observed(lock_path, content, || {})
}

/// [`try_create_lock`] with `on_visible` run the moment the lock path exists.
///
/// FJ-LOCK-001 C1 (#921): the lock is never visible empty. `create_new` then
/// `write_all` left a window in which a racer read "", parsed no PID, judged
/// the lock stale and unlinked the LIVE holder's lock. Now the full content is
/// written to a private staging file and published with `link(2)`, which is as
/// exclusive as O_EXCL (EEXIST if the path exists) and makes the path appear
/// with its content already in place.
fn try_create_lock_observed(
    lock_path: &Path,
    content: &str,
    on_visible: impl FnOnce(),
) -> Result<(), LockAcquireError> {
    let staging = staging_path(lock_path);
    std::fs::write(&staging, content)
        .map_err(|e| LockAcquireError::Io(format!("cannot write lock file: {e}")))?;
    let linked = std::fs::hard_link(&staging, lock_path);
    let _ = std::fs::remove_file(&staging);
    match linked {
        Ok(()) => {
            on_visible();
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(LockAcquireError::AlreadyExists)
        }
        Err(e) => Err(LockAcquireError::Io(format!(
            "cannot create lock file: {e}"
        ))),
    }
}

/// A staging path beside the lock, unique per process and per attempt, so two
/// threads or processes never share one.
fn staging_path(lock_path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    lock_path.with_file_name(format!(".forjar.lock.{}.{n}.tmp", std::process::id()))
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
    match LockInfo::parse(content).holder_with(is_running) {
        Holder::Dead => LockOwner::Stale,
        Holder::LiveLocal | Holder::Remote => LockOwner::Live(LockInfo::parse(content).pid),
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
        return Ok(ReapOutcome::HeldByLivePid(held_message(
            pid,
            lock_path,
            &LockInfo::parse(&content),
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
/// READ-ONLY: is this state dir held by a live process OTHER than ours?
///
/// forjar#310. The apply pipeline validates BEFORE it acquires: `apply_pre_validate`
/// (and therefore `check_pre_apply_drift`, which persists `ResourceStatus::Drifted`)
/// runs at cli/apply.rs, while `acquire_process_lock` is not reached until
/// executor/mod.rs. So a second concurrent apply would run the drift check,
/// WRITE state.lock.yaml and re-seal its `.b3` — over the state belonging to the
/// run that holds the lock — and only THEN be refused:
///
/// ```text
/// error: state directory is locked by PID 1034915
/// state.lock.yaml: MUTATED by the REFUSED apply
/// ```
///
/// Measured, not inferred. (An indented block here is a DOCTEST — rustdoc tried
/// to compile that transcript as Rust, and the clean-room gate caught it where
/// the repo's own CI did not: GATE B3 runs doctests, `ci / test` does not.
/// paiml/forjar#318.)
///
/// This is deliberately a READ-ONLY PROBE and not an early acquire.
/// `acquire_process_lock` uses `create_new` and is not reentrant, so acquiring
/// here would make the executor's own later acquire fail against our own PID.
/// Changing that lock's lifetime is a real concurrency change; refusing early on
/// an observation is not. It also cannot reap a stale lock — that stays the
/// acquirer's job, so a dead PID does not block the pre-check.
///
/// Returns `None` when free, stale, unreadable, or held by US. Not a substitute
/// for acquiring: between this probe and the acquire another process may win.
/// It only ensures the LOSER has not written anything first.
pub fn locked_by_other_live_pid(state_dir: &Path) -> Option<String> {
    let lock_path = process_lock_path(state_dir);
    let content = std::fs::read_to_string(&lock_path).ok()?;
    let info = LockInfo::parse(&content);
    match classify_lock_owner(&content, is_pid_running) {
        LockOwner::Live(pid) if !info.owned_by_this_process() => {
            Some(held_message(pid, &lock_path, &info))
        }
        _ => None,
    }
}

/// The refusal for a live holder: PID first (older readers match on it), then
/// the whole [`LockInfo`] so the operator can decide without opening the file.
fn held_message(pid: u32, lock_path: &Path, info: &LockInfo) -> String {
    format!(
        "state directory is locked by PID {pid} ({}): {}. Wait, or pass --lock-timeout <dur>; \
         if the holder is gone: forjar apply --force-unlock <ID> (refused while it is alive)",
        lock_path.display(),
        info.describe()
    )
}

/// Release the lock at `<state_dir>/.forjar.lock` if THIS process owns it.
pub fn release_process_lock(state_dir: &Path) {
    let lock_path = process_lock_path(state_dir);
    let ours = read_lock_info(&lock_path).is_some_and(|i| i.owned_by_this_process());
    if ours {
        let _ = std::fs::remove_file(&lock_path);
    }
}

/// FJ-LOCK-001 C3 (#923): remove the lock at `lock_path` only if it still
/// carries `id`. A's late release can no longer delete the lock B acquired
/// after A's was reaped or force-unlocked, which used to admit a third apply.
pub(crate) fn release_owned(lock_path: &Path, id: &str) {
    if read_lock_info(lock_path).is_some_and(|i| i.id == id) {
        let _ = std::fs::remove_file(lock_path);
    }
}

/// FJ-LOCK-001 C2 (#922): the apply's hold on a set of locks, released on Drop.
///
/// The lock used to be taken in the executor, after the drift gate had already
/// written state, released before the generation and outputs were recorded,
/// and leaked on every `?` and panic between acquire and release. A guard ties
/// release to scope: the CLI takes it before `apply_pre_validate` and holds it
/// through `apply_post_actions`; `Drop` runs on return, `?` and unwind alike.
///
/// Re-entrant on the SAME thread only: the executor's own take under a CLI
/// hold joins it. Another thread or process still meets the lock file and is
/// refused, exactly as before. Release is owner-checked (C3): each lock is
/// removed only while it still carries the id this guard wrote.
#[derive(Debug)]
#[must_use = "the lock is released when the guard drops"]
pub struct ProcessLockGuard {
    held: Vec<(PathBuf, String)>,
}

thread_local! {
    static HELD: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Take (or, when this thread already holds it, join) the apply lock for
/// `state_dir`. `force_unlock` clears a dead holder first and refuses a live one.
pub fn hold_process_lock(state_dir: &Path, force_unlock: bool) -> Result<ProcessLockGuard, String> {
    std::fs::create_dir_all(state_dir).map_err(|e| format!("cannot create state dir: {e}"))?;
    hold_paths(
        &[process_lock_path(state_dir)],
        force_unlock,
        Duration::ZERO,
    )
}

/// Acquire every lock file in `paths` in the order given (callers sort, so two
/// multi-lock applies cannot deadlock), sharing ONE deadline. On any failure
/// the locks already taken are released by the guard's drop.
pub(crate) fn hold_paths(
    paths: &[PathBuf],
    force_unlock: bool,
    timeout: Duration,
) -> Result<ProcessLockGuard, String> {
    let deadline = Instant::now() + timeout;
    let mut guard = ProcessLockGuard { held: Vec::new() };
    for path in paths {
        if HELD.with(|h| h.borrow().iter().any(|p| p == path)) {
            continue;
        }
        if force_unlock {
            force_unlock_path(path, None)?;
        }
        let left = deadline.saturating_duration_since(Instant::now());
        let info = acquire_at(path, left)?;
        HELD.with(|h| h.borrow_mut().push(path.clone()));
        guard.held.push((path.clone(), info.id));
    }
    Ok(guard)
}

impl Drop for ProcessLockGuard {
    fn drop(&mut self) {
        for (path, id) in self.held.drain(..).rev() {
            HELD.with(|h| h.borrow_mut().retain(|p| p != &path));
            release_owned(&path, &id);
        }
    }
}

/// Force-remove the process lock (for --force-unlock), refusing a live holder.
pub fn force_unlock(state_dir: &Path) -> Result<(), String> {
    force_unlock_path(&process_lock_path(state_dir), None)
}

/// FJ-LOCK-001 C3 (#923): the Terraform `force-unlock <ID>` model. Removes the
/// lock at `lock_path` unless its holder is a live process on THIS host, and,
/// when `expected_id` is given, unless the lock's id differs (the operator
/// named a lock that has since been replaced). A remote holder cannot be
/// verified from here, so the named id is the operator's attestation.
pub(crate) fn force_unlock_path(lock_path: &Path, expected_id: Option<&str>) -> Result<(), String> {
    let Some(info) = read_lock_info(lock_path) else {
        return Ok(());
    };
    if let Some(id) = expected_id {
        if info.id != id {
            return Err(format!(
                "refusing --force-unlock {id}: {} now holds a different lock ({})",
                lock_path.display(),
                info.describe()
            ));
        }
    }
    if info.holder() == Holder::LiveLocal {
        return Err(format!(
            "refusing to force-unlock {}: its holder is alive on this host ({}); \
             wait for it, or stop that process",
            lock_path.display(),
            info.describe()
        ));
    }
    match std::fs::remove_file(lock_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("cannot remove lock file: {e}")),
    }
}

/// Parse PID from lock file content (kept for the legacy-format tests).
#[cfg(test)]
pub(crate) fn parse_lock_pid(content: &str) -> Option<u32> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("pid:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests;
