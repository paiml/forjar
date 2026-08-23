//! A state directory that is not there is not evidence of anything.
//!
//! `lock-verify` and `lock-info` have always exited 1 on a missing state dir.
//! `lock-validate`, `lock-stats` and `lock-audit` exited 0 — `lock-validate`
//! going as far as printing "All 0 lock files are valid". A wrong `--state-dir`,
//! a wiped state, a gate that runs before the first apply: all green.
//!
//! This file pins the aligned behaviour. It does NOT claim the three commands
//! now fail on an existing-but-empty directory — only `lock-verify-chain` was
//! hardened that far; see `falsification_lock_verify_chain_gate.rs`.

use std::path::PathBuf;
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// A path that does not exist, inside a temp dir that does.
fn missing() -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("no-such-state");
    (d, p)
}

fn assert_fails(cmd: &str) {
    let (_guard, path) = missing();
    let out = forjar()
        .args([cmd, "--state-dir", path.to_str().unwrap()])
        .output()
        .expect("spawn forjar");
    assert_eq!(
        out.status.code().unwrap_or(-1),
        1,
        "`forjar {cmd}` reported success against a state dir that does not exist\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn lock_validate_must_not_certify_an_absent_state_dir() {
    assert_fails("lock-validate");
}

#[test]
fn lock_stats_must_not_report_on_an_absent_state_dir() {
    assert_fails("lock-stats");
}

#[test]
fn lock_audit_must_not_certify_an_absent_state_dir() {
    assert_fails("lock-audit");
}

/// The two that already behaved, kept here so a regression in either shows up
/// next to the commands that were aligned to them.
#[test]
fn lock_verify_and_lock_info_still_fail_on_an_absent_state_dir() {
    assert_fails("lock-verify");
    assert_fails("lock-info");
}
