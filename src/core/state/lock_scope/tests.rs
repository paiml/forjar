use super::*;

/// FJ-LOCK-001 (#1307, slice of #920): locks are per machine. While one machine's
/// lock is held, another machine in the same state dir still locks, and the same
/// machine still refuses. The holder is the test thread; the checks run on a
/// second thread so the per-thread re-entrancy in `hold_paths` cannot hide a refusal.
#[test]
fn test_fj_lock_001_per_machine_locks_are_independent() {
    let dir = tempfile::tempdir().unwrap();
    let sd = dir.path().to_path_buf();
    let alpha = hold_machine_locks(&sd, &["alpha".to_string()], false).unwrap();
    let sd2 = sd.clone();
    let (beta_locked, alpha_refused) = std::thread::spawn(move || {
        let beta = hold_machine_locks(&sd2, &["beta".to_string()], false).is_ok();
        let alpha_again = hold_machine_locks(&sd2, &["alpha".to_string()], false).is_err();
        (beta, alpha_again)
    })
    .join()
    .unwrap();
    assert!(beta_locked, "beta must lock while alpha is held");
    assert!(alpha_refused, "alpha must refuse while it is held");
    drop(alpha);
}
