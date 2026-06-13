use super::*;
use std::path::PathBuf;

/// Whether the test process is running as root (effective uid 0). Used to skip
/// permission-denial simulations that root bypasses. Dependency-free: parse the
/// effective uid from `/proc/self/status` (Linux). Falls back to "not root" if
/// it can't be read, which is the safe default for these simulations.
#[cfg(unix)]
fn is_root() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("Uid:"))
                .and_then(|rest| rest.split_whitespace().nth(1).map(str::to_string))
        })
        .map(|euid| euid == "0")
        .unwrap_or(false)
}

#[test]
fn test_fj266_acquire_and_release() {
    let dir = tempfile::tempdir().unwrap();
    acquire_process_lock(dir.path()).unwrap();
    let lock_path = process_lock_path(dir.path());
    assert!(lock_path.exists());
    let content = std::fs::read_to_string(&lock_path).unwrap();
    assert!(content.contains(&format!("pid: {}", std::process::id())));
    release_process_lock(dir.path());
    assert!(!lock_path.exists());
}

#[test]
fn test_fj266_concurrent_lock_blocked() {
    let dir = tempfile::tempdir().unwrap();
    // Write a lock with our own PID (still running)
    let lock_path = process_lock_path(dir.path());
    let content = format!(
        "pid: {}\nstarted_at: 2026-02-26T00:00:00Z\n",
        std::process::id()
    );
    std::fs::write(&lock_path, content).unwrap();

    let result = acquire_process_lock(dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("locked by PID"));
}

#[test]
fn test_fj266_stale_lock_cleaned() {
    let dir = tempfile::tempdir().unwrap();
    // PID 999999999 is almost certainly not running
    let lock_path = process_lock_path(dir.path());
    let content = "pid: 999999999\nstarted_at: 2026-02-26T00:00:00Z\n";
    std::fs::write(&lock_path, content).unwrap();

    // Should succeed — stale lock is cleaned up
    acquire_process_lock(dir.path()).unwrap();
    let new_content = std::fs::read_to_string(&lock_path).unwrap();
    assert!(new_content.contains(&format!("pid: {}", std::process::id())));
    release_process_lock(dir.path());
}

#[test]
fn test_fj266_force_unlock() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = process_lock_path(dir.path());
    std::fs::write(&lock_path, "pid: 12345\n").unwrap();
    force_unlock(dir.path()).unwrap();
    assert!(!lock_path.exists());
}

#[test]
fn test_fj266_force_unlock_no_lock() {
    let dir = tempfile::tempdir().unwrap();
    // No lock file — should be fine
    force_unlock(dir.path()).unwrap();
}

#[test]
fn test_fj266_parse_lock_pid() {
    assert_eq!(parse_lock_pid("pid: 12345\nstarted_at: x\n"), Some(12345));
    assert_eq!(parse_lock_pid("no pid here"), None);
    assert_eq!(parse_lock_pid("pid: abc"), None);
    assert_eq!(parse_lock_pid(""), None);
}

#[test]
fn test_fj266_lock_path() {
    let p = process_lock_path(std::path::Path::new("/state"));
    assert_eq!(p, PathBuf::from("/state/.forjar.lock"));
}

#[test]
fn test_fj266_lock_creates_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a").join("b").join("state");
    acquire_process_lock(&nested).unwrap();
    assert!(nested.exists());
    assert!(process_lock_path(&nested).exists());
    release_process_lock(&nested);
}

// #154: atomic acquisition — second acquire fails while held by a live PID.
#[test]
fn test_154_second_acquire_blocked_while_held() {
    let dir = tempfile::tempdir().unwrap();
    // First acquire writes our own (live) PID atomically.
    acquire_process_lock(dir.path()).unwrap();
    // A second acquire in the same (live) process must be rejected.
    let blocked = acquire_process_lock(dir.path());
    assert!(blocked.is_err());
    assert!(blocked.unwrap_err().contains("locked by PID"));
    // Releasing then re-acquiring must succeed.
    release_process_lock(dir.path());
    acquire_process_lock(dir.path()).unwrap();
    release_process_lock(dir.path());
    assert!(!process_lock_path(dir.path()).exists());
}

// #154: a lock owned by a dead PID is reaped, then we acquire atomically.
#[test]
fn test_154_stale_pid_reaped_then_acquired() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = process_lock_path(dir.path());
    // 999999999 is almost certainly not a running PID.
    std::fs::write(&lock_path, "pid: 999999999\nstarted_at: x\n").unwrap();
    acquire_process_lock(dir.path()).unwrap();
    let content = std::fs::read_to_string(&lock_path).unwrap();
    assert!(content.contains(&format!("pid: {}", std::process::id())));
    release_process_lock(dir.path());
}

// #154: a lock file whose content has no parseable PID is treated as stale.
#[test]
fn test_154_unparseable_lock_reaped() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = process_lock_path(dir.path());
    std::fs::write(&lock_path, "garbage with no pid line\n").unwrap();
    // No PID parsed → not "running" → reaped → acquire succeeds.
    acquire_process_lock(dir.path()).unwrap();
    release_process_lock(dir.path());
}

// #154: a lock that vanishes between create_new and read is retried, not failed.
#[test]
fn test_154_reap_missing_lock_is_ok() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = process_lock_path(dir.path());
    // No file present at all — reap helper must report "retry" (Ok).
    assert!(matches!(
        reap_or_reject_stale_lock(&lock_path).unwrap(),
        ReapOutcome::Retry
    ));
}

// =========================================================================
// #165 (#1): bounded acquire loop / error propagation on un-removable stale.
// =========================================================================

// The owner-classification decision is pure (modulo the liveness probe), so we
// exercise its branch logic directly without spawning processes.
#[test]
fn test_165_classify_owner_live_pid_when_running() {
    let owner = classify_lock_owner("pid: 7\nstarted_at: x\n", |_| true);
    assert!(matches!(owner, LockOwner::Live(7)));
}

#[test]
fn test_165_classify_owner_stale_when_dead() {
    let owner = classify_lock_owner("pid: 7\nstarted_at: x\n", |_| false);
    assert!(matches!(owner, LockOwner::Stale));
}

#[test]
fn test_165_classify_owner_stale_when_unparseable() {
    // No PID line → never "running" → reapable as stale.
    let owner = classify_lock_owner("garbage\n", |_| true);
    assert!(matches!(owner, LockOwner::Stale));
}

// #1: A stale lock that CANNOT be removed must return Err — never loop forever.
// We simulate an un-removable stale lock by making the lock path a non-empty
// DIRECTORY: it is "stale" (no parseable pid → not running), `read_to_string`
// of a directory... actually fails, so instead we use a read-only parent dir so
// `remove_file` returns a permission/EISDIR-style error. The key assertion is
// that the helper returns Err rather than retrying indefinitely.
#[cfg(unix)]
#[test]
fn test_165_unremovable_stale_lock_propagates_err() {
    use std::os::unix::fs::PermissionsExt;
    if is_root() {
        // root bypasses directory write perms, so EACCES can't be simulated;
        // the pure branch logic is covered by classify_lock_owner tests.
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let subdir = dir.path().join("ro");
    std::fs::create_dir(&subdir).unwrap();
    let lock_path = subdir.join(".forjar.lock");
    // Stale (dead PID) lock content.
    std::fs::write(&lock_path, "pid: 999999999\nstarted_at: x\n").unwrap();
    // Make the PARENT dir read-only so unlink of the entry is denied (EACCES).
    let mut perms = std::fs::metadata(&subdir).unwrap().permissions();
    perms.set_mode(0o500); // r-x------ : can read/traverse, cannot remove entries
    std::fs::set_permissions(&subdir, perms).unwrap();

    let result = reap_or_reject_stale_lock(&lock_path);

    // Restore perms so tempdir cleanup succeeds regardless of assertion outcome.
    let mut restore = std::fs::metadata(&subdir).unwrap().permissions();
    restore.set_mode(0o700);
    std::fs::set_permissions(&subdir, restore).unwrap();

    // MUST be Err (un-removable confirmed-stale lock), NOT a silent retry.
    assert!(result.is_err(), "expected Err on un-removable stale lock");
    assert!(result.unwrap_err().contains("cannot remove stale lock"));
}

// #1: the full acquire path over an un-removable stale lock must TERMINATE with
// Err (bounded loop), never hang. Running it inside the test asserts no spin.
#[cfg(unix)]
#[test]
fn test_165_acquire_terminates_on_unremovable_stale() {
    use std::os::unix::fs::PermissionsExt;
    if is_root() {
        // root bypasses directory perms: acquire would succeed by removing the
        // stale lock. The bound itself is still exercised — it terminates here
        // either way (Ok via reap, or Err via bound), never spinning.
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("st");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(
            process_lock_path(&subdir),
            "pid: 999999999\nstarted_at: x\n",
        )
        .unwrap();
        // Must return (not hang); under root the stale lock is reaped → Ok.
        acquire_process_lock(&subdir).unwrap();
        release_process_lock(&subdir);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let subdir = dir.path().join("st");
    std::fs::create_dir(&subdir).unwrap();
    let lock_path = process_lock_path(&subdir);
    std::fs::write(&lock_path, "pid: 999999999\nstarted_at: x\n").unwrap();
    let mut perms = std::fs::metadata(&subdir).unwrap().permissions();
    perms.set_mode(0o500);
    std::fs::set_permissions(&subdir, perms).unwrap();

    let result = acquire_process_lock(&subdir);

    let mut restore = std::fs::metadata(&subdir).unwrap().permissions();
    restore.set_mode(0o700);
    std::fs::set_permissions(&subdir, restore).unwrap();

    assert!(result.is_err(), "acquire must give up, not spin forever");
}

// =========================================================================
// #165 (#2): stale-reap TOCTOU guard — only delete when content still matches.
// =========================================================================

// If the lock file's content has CHANGED since we observed it stale (a
// concurrent acquirer wrote a fresh valid lock), the reap must NOT remove it.
#[test]
fn test_165_reap_does_not_delete_replaced_lock() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = process_lock_path(dir.path());
    // On disk: a FRESH valid lock (a concurrent winner's, with OUR live PID).
    let fresh = format!("pid: {}\nstarted_at: fresh\n", std::process::id());
    std::fs::write(&lock_path, &fresh).unwrap();

    // We "observed" the OLD stale content; the file now holds different content.
    let observed_stale = "pid: 999999999\nstarted_at: old\n";
    let outcome = reap_stale_if_unchanged(&lock_path, observed_stale).unwrap();

    // Must be Retry, and CRUCIALLY the winner's lock must still be on disk.
    assert!(matches!(outcome, ReapOutcome::Retry));
    assert!(
        lock_path.exists(),
        "must not delete a replaced (fresh) lock"
    );
    assert_eq!(std::fs::read_to_string(&lock_path).unwrap(), fresh);
}

// When content STILL matches the observed dead-PID, the reap removes it.
#[test]
fn test_165_reap_removes_when_content_matches() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = process_lock_path(dir.path());
    let stale = "pid: 999999999\nstarted_at: old\n";
    std::fs::write(&lock_path, stale).unwrap();

    let outcome = reap_stale_if_unchanged(&lock_path, stale).unwrap();
    assert!(matches!(outcome, ReapOutcome::Retry));
    assert!(!lock_path.exists(), "matching stale lock must be removed");
}

// A live-PID lock is reported as HeldByLivePid (reject), not reaped.
#[test]
fn test_165_reap_reports_live_pid_held() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = process_lock_path(dir.path());
    std::fs::write(
        &lock_path,
        format!("pid: {}\nstarted_at: x\n", std::process::id()),
    )
    .unwrap();
    let outcome = reap_or_reject_stale_lock(&lock_path).unwrap();
    match outcome {
        ReapOutcome::HeldByLivePid(msg) => assert!(msg.contains("locked by PID")),
        ReapOutcome::Retry => panic!("live PID lock must be rejected, not reaped"),
    }
    // The live lock must remain untouched.
    assert!(lock_path.exists());
}
