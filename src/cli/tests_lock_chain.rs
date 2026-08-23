//! Unit coverage for `lock_chain.rs` (`forjar lock-verify-chain`).
//!
//! The exit-code contract itself is pinned end-to-end through the binary in
//! `tests/falsification_lock_verify_chain_gate.rs`; these cover the function's
//! branches directly. Moved here from `tests_cov_lock.rs` when the chain check
//! got its own module.

#![allow(unused_imports)]
use super::lock_chain::*;
use super::lock_merge::cmd_lock_sign;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn state_lock_yaml() -> &'static str {
        "schema: \"1\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources: {}\n"
    }

    /// A state dir with one machine, `web`, holding an UNSIGNED lock.
    fn make_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("web")).unwrap();
        std::fs::write(dir.path().join("web/state.lock.yaml"), state_lock_yaml()).unwrap();
        dir
    }

    /// Sign through the real command, not a re-implementation of it — the
    /// signature format is exactly what these tests must not guess at.
    fn sign(dir: &Path, key: &str) {
        assert!(cmd_lock_sign(dir, key, false).is_ok());
    }

    // ── moved here from tests_cov_lock2.rs with the module ──

    #[test]
    fn test_sign_then_verify_chain() {
        let d = make_dir();
        sign(d.path(), "k");
        assert!(cmd_lock_verify_chain(d.path(), Some("k"), false, false).is_ok());
    }

    #[test]
    fn test_sign_then_verify_chain_json() {
        let d = make_dir();
        sign(d.path(), "k");
        assert!(cmd_lock_verify_chain(d.path(), Some("k"), false, true).is_ok());
    }

    /// The signature is well-formed hex either way; only the key tells the two
    /// cases apart, which is why verify-chain now takes one.
    #[test]
    fn test_sign_then_verify_chain_wrong_key() {
        let d = make_dir();
        sign(d.path(), "right");
        assert!(cmd_lock_verify_chain(d.path(), Some("wrong"), false, false).is_err());
    }

    /// An unsigned lock is a BROKEN chain. This asserted `is_ok` in
    /// tests_cov_lock2.rs only because verify-chain returned Ok whatever it
    /// found — see tests/falsification_lock_verify_chain_gate.rs.
    #[test]
    fn test_lock_verify_chain_unsigned_lock_fails() {
        let d = make_dir();
        assert!(cmd_lock_verify_chain(d.path(), None, true, false).is_err());
    }

    #[test]
    fn test_lock_verify_chain_unsigned_lock_fails_json() {
        let d = make_dir();
        assert!(cmd_lock_verify_chain(d.path(), None, true, true).is_err());
    }

    // `make_dir` writes a lock and no signature: an unsigned lock has no chain
    // of custody. These two discarded the result entirely (`let _ =`), so they
    // would have passed whatever the command returned.
    #[test]
    fn test_lock_verify_chain_plain() {
        let dir = make_dir();
        assert!(cmd_lock_verify_chain(dir.path(), None, true, false).is_err());
    }

    #[test]
    fn test_lock_verify_chain_json() {
        let dir = make_dir();
        assert!(cmd_lock_verify_chain(dir.path(), None, true, true).is_err());
    }

    /// A bare invocation verifies nothing and must say so rather than pass.
    #[test]
    fn test_lock_verify_chain_without_key_refuses() {
        let dir = make_dir();
        assert!(cmd_lock_verify_chain(dir.path(), None, false, false).is_err());
    }

    /// `--key` and `--presence-only` are contradictory: one verifies the
    /// signature against the lock, the other declines to.
    #[test]
    fn test_lock_verify_chain_key_and_presence_only_conflict() {
        let dir = make_dir();
        assert!(cmd_lock_verify_chain(dir.path(), Some("k"), true, false).is_err());
    }

    #[test]
    fn test_lock_verify_chain_signed_verifies() {
        let dir = make_dir();
        sign(dir.path(), "k");
        assert!(cmd_lock_verify_chain(dir.path(), Some("k"), false, false).is_ok());
        assert!(cmd_lock_verify_chain(dir.path(), Some("k"), false, true).is_ok());
    }

    #[test]
    fn test_lock_verify_chain_wrong_key_fails() {
        let dir = make_dir();
        sign(dir.path(), "right");
        assert!(cmd_lock_verify_chain(dir.path(), Some("wrong"), false, false).is_err());
    }

    /// Editing the lock after signing leaves a well-formed signature of the
    /// OLD content — the case well-formedness checking cannot see.
    #[test]
    fn test_lock_verify_chain_tampered_lock_fails() {
        let dir = make_dir();
        sign(dir.path(), "k");
        std::fs::write(
            dir.path().join("web/state.lock.yaml"),
            format!("{}# tampered\n", state_lock_yaml()),
        )
        .unwrap();
        assert!(cmd_lock_verify_chain(dir.path(), Some("k"), false, false).is_err());
    }

    /// A signature whose lock was deleted is a break in custody. The old
    /// discovery required `state.lock.yaml` to exist, so this machine was
    /// invisible and the "lock file missing" branch unreachable.
    #[test]
    fn test_lock_verify_chain_orphan_signature_fails() {
        let dir = make_dir();
        sign(dir.path(), "k");
        std::fs::remove_file(dir.path().join("web/state.lock.yaml")).unwrap();
        assert!(cmd_lock_verify_chain(dir.path(), None, true, false).is_err());
    }

    /// A signature file is untrusted input; truncating its preview on a byte
    /// boundary used to panic.
    #[test]
    fn test_lock_verify_chain_multibyte_signature_does_not_panic() {
        let dir = make_dir();
        std::fs::write(dir.path().join("web/lock.sig"), "☃".repeat(10)).unwrap();
        assert!(cmd_lock_verify_chain(dir.path(), None, true, false).is_err());
    }

    #[test]
    fn test_lock_verify_chain_absent_and_empty_state_dirs_fail() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("nope");
        assert!(cmd_lock_verify_chain(&absent, None, true, false).is_err());
        assert!(cmd_lock_verify_chain(dir.path(), None, true, false).is_err());
    }
}
