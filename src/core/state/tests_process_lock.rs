use super::*;
use std::path::PathBuf;

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
    reap_or_reject_stale_lock(&lock_path).unwrap();
}
