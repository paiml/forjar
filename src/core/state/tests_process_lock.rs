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
