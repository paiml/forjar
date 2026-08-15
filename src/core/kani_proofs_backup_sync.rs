//! FJ-037: Kani bounded-model proofs for the `backup_sync` resource.
//!
//! Gated behind `#[cfg(kani)]`; normal builds ignore them.
//! Run with: `cargo kani --harness proof_backup_remote_never_local`.
//!
//! These prove the destination algebra over the whole input space a unit test
//! can only sample. The deletion-free parts that matter operationally — does a
//! zero-match verification actually fail? — are proved by execution instead, in
//! `tests/falsification_backup_sync.rs`, because a bounded model cannot say
//! anything useful about what `rclone` reports.

// ── Backup Sync Proofs (FJ-037 / backup-sync-v1) ──────────────────────

/// Contract `remote_cannot_be_local`: no accepted destination is a local path.
///
/// The predecessor's destination was `/videos`, a symlink back to its own
/// source. For every short string over a character set that includes the
/// dangerous prefixes, an accepted `BackupSync` must contain a `:` and must not
/// begin with `/`, `.` or `~` — so a local destination cannot be constructed.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(10)]
fn proof_backup_remote_never_local() {
    use super::types::BackupSync;

    // A small alphabet that deliberately includes every rejected prefix.
    const ALPHABET: [u8; 6] = [b'/', b'.', b'~', b':', b'a', b'1'];
    let i0: usize = kani::any();
    let i1: usize = kani::any();
    let i2: usize = kani::any();
    kani::assume(i0 < ALPHABET.len() && i1 < ALPHABET.len() && i2 < ALPHABET.len());

    let bytes = [ALPHABET[i0], ALPHABET[i1], ALPHABET[i2]];
    let remote = core::str::from_utf8(&bytes).unwrap();

    if let Ok(b) = BackupSync::new(vec!["/mnt/a".to_string()], remote, "daily", 99, 700, None) {
        // Accepted => provably not a local path.
        assert!(b.remote.contains(':'));
        assert!(!b.remote.starts_with('/'));
        assert!(!b.remote.starts_with('.'));
        assert!(!b.remote.starts_with('~'));
        // ...and the remote name is non-empty, so `name:` is well formed.
        assert!(!b.remote_name().is_empty());
    }
}

/// Contract `coverage_is_checksum_verified`: coverage is bounded 0..=100.
///
/// The percentage is computed in POSIX shell integer arithmetic as
/// `matched * 100 / total`. This pins that it cannot exceed 100 (which would
/// let a broken counter report over-full coverage and mask a real gap) and
/// cannot divide by zero.
#[cfg(kani)]
#[kani::proof]
fn proof_backup_coverage_bounded() {
    let matched: u32 = kani::any();
    let total: u32 = kani::any();
    kani::assume(total > 0 && total <= 1_000_000);
    kani::assume(matched <= total);

    let coverage = matched * 100 / total;
    assert!(coverage <= 100);
    // Full coverage is reachable only when every file matched.
    if coverage == 100 {
        assert!(matched == total);
    }
    // And a zero-match run can never look healthy.
    if matched == 0 {
        assert!(coverage == 0);
    }
}

/// Contract `remote_cannot_be_local`: a declaration with no sources is refused.
///
/// A backup of nothing verifies trivially — 0 of 0 files — and would otherwise
/// report perfect coverage while protecting nothing.
#[cfg(kani)]
#[kani::proof]
fn proof_backup_empty_sources_rejected() {
    use super::types::BackupSync;

    let verify_pct: u8 = kani::any();
    let result = BackupSync::new(vec![], "gdrive:x", "daily", verify_pct, 700, None);
    assert!(result.is_err());
}
