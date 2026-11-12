//! FJ-LOCK-001 C3/C4/C5 falsifiers (#923, #924, #925). Each is written so the
//! pre-fix behaviour fails it: release deleted any file, force-unlock removed
//! a live holder, one lock covered every machine, and a live holder was never
//! waited for.

use super::lock_info::{Holder, LockInfo};
use super::lock_scope::{force_unlock_by_id, hold_global_lock, hold_machine_locks};
use super::process_lock::{force_unlock, hold_paths, process_lock_path};
use std::path::Path;
use std::time::{Duration, Instant};

/// A lock file as a live process that is not us would have written it (pid 1
/// is always alive on this host and its start time is not recorded).
fn write_foreign_live_lock(lock: &Path, id: &str) {
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    let info = LockInfo {
        id: id.to_string(),
        pid: 1,
        starttime: String::new(),
        command: "forjar apply -f other.yaml".to_string(),
        started_at: "2026-11-09T00:00:00Z".to_string(),
        ..LockInfo::current()
    };
    std::fs::write(lock, info.render()).unwrap();
}

fn m(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| s.to_string()).collect()
}

// ---- C3 ---------------------------------------------------------------

#[test]
fn c3_lock_carries_id_host_boot_id_pid_starttime() {
    let dir = tempfile::tempdir().unwrap();
    let _g = hold_machine_locks(dir.path(), &m(&["a"]), false).unwrap();
    let text = std::fs::read_to_string(dir.path().join("a/.forjar.lock")).unwrap();
    let info = LockInfo::parse(&text);
    assert_eq!(info.id.len(), 36, "uuid-shaped id: {}", info.id);
    assert_eq!(info.id.matches('-').count(), 4);
    assert!(!info.host.is_empty() && !info.boot_id.is_empty());
    assert_eq!(info.pid, std::process::id());
    assert!(
        !info.starttime.is_empty(),
        "starttime from /proc/<pid>/stat"
    );
    assert_eq!(info.holder(), Holder::LiveLocal);
}

/// Falsifier: A's release must not delete the lock B now holds.
#[test]
fn c3_release_by_non_owner_leaves_the_lock() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("a/.forjar.lock");
    let guard_a = hold_machine_locks(dir.path(), &m(&["a"]), false).unwrap();
    // A's lock is reaped or force-unlocked and B takes the machine.
    write_foreign_live_lock(&lock, "b-owns-this");
    drop(guard_a);
    let after = std::fs::read_to_string(&lock).expect("non-owner release deleted B's lock");
    assert_eq!(LockInfo::parse(&after).id, "b-owns-this");
}

#[test]
fn c3_owner_release_removes_the_lock() {
    let dir = tempfile::tempdir().unwrap();
    let guard = hold_machine_locks(dir.path(), &m(&["a"]), false).unwrap();
    drop(guard);
    assert!(!dir.path().join("a/.forjar.lock").exists());
}

/// Falsifier: force-unlock of a live local holder is refused, by id or bare.
#[test]
fn c3_force_unlock_live_holder_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("a/.forjar.lock");
    write_foreign_live_lock(&lock, "live-id");
    let by_id = force_unlock_by_id(dir.path(), "live-id").unwrap_err();
    assert!(by_id.contains("holder is alive"), "{by_id}");
    assert!(lock.exists(), "a live holder's lock was removed");
    let bare = hold_machine_locks(dir.path(), &m(&["a"]), true).unwrap_err();
    assert!(bare.contains("holder is alive"), "{bare}");
    assert!(lock.exists());
    write_foreign_live_lock(&process_lock_path(dir.path()), "root-id");
    assert!(force_unlock(dir.path()).unwrap_err().contains("alive"));
}

#[test]
fn c3_force_unlock_by_id_clears_a_dead_holder_and_checks_the_id() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("a/.forjar.lock");
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    let dead = LockInfo {
        id: "dead-id".into(),
        pid: 4_294_967_290,
        ..LockInfo::current()
    };
    std::fs::write(&lock, dead.render()).unwrap();
    let err = force_unlock_by_id(dir.path(), "no-such-id").unwrap_err();
    assert!(err.contains("no lock"), "{err}");
    assert!(lock.exists());
    let removed = force_unlock_by_id(dir.path(), "dead-id").unwrap();
    assert_eq!(removed, lock);
    assert!(!lock.exists());
}

/// A recycled PID is not the holder: the recorded starttime must match.
#[test]
fn c3_pid_reuse_is_detected_by_starttime() {
    let reused = LockInfo {
        starttime: "1".into(),
        ..LockInfo::current()
    };
    assert_eq!(reused.holder(), Holder::Dead, "same pid, different start");
    let other_boot = LockInfo {
        boot_id: "00000000-0000-0000-0000-000000000000".into(),
        ..LockInfo::current()
    };
    assert_eq!(
        other_boot.holder(),
        Holder::Dead,
        "lock from before a reboot"
    );
    let remote = LockInfo {
        host: "some-other-host".into(),
        ..LockInfo::current()
    };
    assert_eq!(remote.holder(), Holder::Remote, "never reaped from here");
}

// ---- C4 ---------------------------------------------------------------

/// Falsifier: a hung apply on machine A must not block machine B, and two
/// applies that both declare A are still refused.
#[test]
fn c4_one_hung_machine_does_not_block_another() {
    let dir = tempfile::tempdir().unwrap();
    write_foreign_live_lock(&dir.path().join("intel/.forjar.lock"), "hung-intel");
    let lambda = hold_machine_locks(dir.path(), &m(&["lambda-labs"]), false);
    assert!(
        lambda.is_ok(),
        "lambda-labs blocked by intel: {:?}",
        lambda.err()
    );
    let intel = hold_machine_locks(dir.path(), &m(&["intel"]), false).unwrap_err();
    assert!(intel.contains("locked by PID 1"), "{intel}");
    assert!(
        intel.contains("id=hung-intel"),
        "holder identity in error: {intel}"
    );
}

#[test]
fn c4_same_machine_second_apply_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    let _first = hold_machine_locks(&path, &m(&["a"]), false).unwrap();
    let second =
        std::thread::spawn(move || hold_machine_locks(&path, &m(&["a"]), false).map(|_| ()))
            .join()
            .unwrap();
    assert!(second.unwrap_err().contains("locked by PID"));
}

/// A failure part-way through a multi-machine take leaves nothing behind.
#[test]
fn c4_partial_take_is_rolled_back() {
    let dir = tempfile::tempdir().unwrap();
    write_foreign_live_lock(&dir.path().join("b/.forjar.lock"), "held-b");
    let err = hold_machine_locks(dir.path(), &m(&["b", "a"]), false);
    assert!(err.is_err());
    assert!(
        !dir.path().join("a/.forjar.lock").exists(),
        "machine a's lock leaked after b was refused"
    );
}

#[test]
fn c4_machine_name_cannot_leave_the_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    for bad in ["", "..", "a/b", "../x"] {
        assert!(
            hold_machine_locks(dir.path(), &m(&[bad]), false).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn c4_global_lock_is_short_reentrant_and_released() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".forjar.global.lock");
    let outer = hold_global_lock(dir.path()).unwrap();
    let inner = hold_global_lock(dir.path()).unwrap();
    drop(inner);
    assert!(path.exists(), "inner guard released the outer hold");
    drop(outer);
    assert!(!path.exists());
}

/// Two writers of the shared global file take turns instead of interleaving.
#[test]
fn c4_global_lock_serialises_writers_across_threads() {
    let dir = tempfile::tempdir().unwrap();
    let d2 = dir.path().to_path_buf();
    let held = hold_global_lock(dir.path()).unwrap();
    let t = std::thread::spawn(move || {
        let start = Instant::now();
        let _g = hold_global_lock(&d2).unwrap();
        start.elapsed()
    });
    std::thread::sleep(Duration::from_millis(400));
    drop(held);
    assert!(
        t.join().unwrap() >= Duration::from_millis(300),
        "did not wait"
    );
}

// ---- C5 ---------------------------------------------------------------

#[test]
fn c5_parse_lock_timeout() {
    use super::lock_scope::parse_lock_timeout as p;
    assert_eq!(p("0").unwrap(), Duration::ZERO);
    assert_eq!(p("5s").unwrap(), Duration::from_secs(5));
    assert_eq!(p("250ms").unwrap(), Duration::from_millis(250));
    assert_eq!(p("2m").unwrap(), Duration::from_secs(120));
    assert_eq!(p("30").unwrap(), Duration::from_secs(30));
    assert!(p("soon").is_err() && p("5x").is_err() && p("").is_err());
}

/// Falsifier: timeout 0 fails at once and names the holder in full.
#[test]
fn c5_timeout_zero_fails_fast_naming_the_holder() {
    let dir = tempfile::tempdir().unwrap();
    write_foreign_live_lock(&dir.path().join("a/.forjar.lock"), "holder-77");
    let start = Instant::now();
    let err = hold_paths(&[dir.path().join("a/.forjar.lock")], false, Duration::ZERO)
        .map(|_| ())
        .unwrap_err();
    assert!(start.elapsed() < Duration::from_millis(500), "waited");
    for want in [
        "id=holder-77",
        "host=",
        "command=[forjar apply -f other.yaml]",
        "started_at=2026-11-09T00:00:00Z",
    ] {
        assert!(err.contains(want), "missing {want} in {err}");
    }
}

/// Falsifier: a bounded wait that expires carries the LockInfo in its error.
#[test]
fn c5_timeout_error_carries_lockinfo() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("a/.forjar.lock");
    write_foreign_live_lock(&lock, "holder-88");
    let start = Instant::now();
    let err = hold_paths(&[lock], false, Duration::from_millis(400))
        .map(|_| ())
        .unwrap_err();
    assert!(
        start.elapsed() >= Duration::from_millis(350),
        "gave up early"
    );
    assert!(
        err.contains("id=holder-88") && err.contains("--lock-timeout expired"),
        "{err}"
    );
    assert!(
        std::fs::read_dir(dir.path().join("a")).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".q.")),
        "queue ticket leaked after the wait"
    );
}

/// Falsifier: the holder releases after a while, the waiter gets in.
#[test]
fn c5_waiter_acquires_after_holder_releases() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("a/.forjar.lock");
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    let (held_tx, held_rx) = std::sync::mpsc::channel();
    let hold_path = lock.clone();
    let releaser = std::thread::spawn(move || {
        let holder = hold_paths(&[hold_path], false, Duration::ZERO).unwrap();
        held_tx.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(800));
        drop(holder);
    });
    held_rx.recv().unwrap();
    let start = Instant::now();
    let got = hold_paths(&[lock], false, Duration::from_secs(5));
    assert!(got.is_ok(), "waiter timed out: {:?}", got.err());
    assert!(start.elapsed() >= Duration::from_millis(700));
    releaser.join().unwrap();
}

/// Waiters are served in arrival order.
#[test]
fn c5_waiters_are_served_fifo() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("a/.forjar.lock");
    std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
    let holder = hold_paths(std::slice::from_ref(&lock), false, Duration::ZERO).unwrap();
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut joins = Vec::new();
    for n in 0..3u32 {
        let (lock, order) = (lock.clone(), order.clone());
        joins.push(std::thread::spawn(move || {
            let g = hold_paths(&[lock], false, Duration::from_secs(20)).unwrap();
            order.lock().unwrap().push(n);
            std::thread::sleep(Duration::from_millis(60));
            drop(g);
        }));
        std::thread::sleep(Duration::from_millis(200));
    }
    drop(holder);
    for j in joins {
        j.join().unwrap();
    }
    assert_eq!(*order.lock().unwrap(), vec![0, 1, 2]);
}
