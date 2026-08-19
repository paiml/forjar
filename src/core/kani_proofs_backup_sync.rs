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
    // MEASURED 2026-08-17: the previous form of this harness ran a single cbmc
    // process for 117 minutes at 6.5 GB RSS and was killed by the job timeout,
    // on an otherwise IDLE box (1 core of 18 in use). It was not slow because
    // the machine was busy — it was intractable, which is the exact shape this
    // workflow's header warns about and the second harness in this crate to
    // take it.
    //
    // The input space was never the problem: 6 characters, length 3, is 216
    // cases. Two other things were.
    //
    //   1. It drove `BackupSync::new`, which allocates a `Vec<String>` of
    //      sources plus several `String`s, so CBMC had to model the allocator
    //      across the whole symbolic space to prove a property about a STRING.
    //   2. The indices were `usize` — 64-bit symbolic values, pruned by an
    //      assume to 6 possibilities but still reasoned about as bitvectors.
    //
    // Retargeting to `validate_remote` alone was not enough — it still runs
    // `format!` on its error paths, and CBMC models every path regardless of
    // which one the property asserts on. `classify_remote` is that same
    // decision with the message rendering lifted out, so the model contains no
    // `core::fmt` and no allocation at all. The property proved is identical in
    // force: `BackupSync::new` accepts a remote exactly when this returns None.
    use super::types::classify_remote;

    // A small alphabet that deliberately includes every rejected prefix.
    const ALPHABET: [u8; 6] = [b'/', b'.', b'~', b':', b'a', b'1'];
    let i0: u8 = kani::any();
    let i1: u8 = kani::any();
    let i2: u8 = kani::any();
    kani::assume(i0 < 6 && i1 < 6 && i2 < 6);

    let bytes = [
        ALPHABET[i0 as usize],
        ALPHABET[i1 as usize],
        ALPHABET[i2 as usize],
    ];
    let remote = core::str::from_utf8(&bytes).unwrap();

    if classify_remote(remote).is_none() {
        // Accepted => provably not a local path.
        assert!(remote.contains(':'));
        assert!(!remote.starts_with('/'));
        assert!(!remote.starts_with('.'));
        assert!(!remote.starts_with('~'));
        // ...and the remote name is non-empty, so `name:` is well formed.
        assert!(!remote.split(':').next().unwrap_or("").is_empty());
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
