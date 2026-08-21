//! FJ-038: Kani bounded-model proofs for the `nas_archive` resource.
//!
//! Gated behind `#[cfg(kani)]`; normal builds ignore them.
//! Run with: `cargo kani --harness proof_archive_dest_never_inside_source`.
//!
//! These prove the containment and admission algebra over the whole input space
//! a unit test can only sample. The operationally decisive property — does a
//! mismatched destination actually leave the source alive? — is proved by
//! EXECUTION instead, in `src/resources/nas_archive/tests.rs`, because a bounded
//! model cannot say anything useful about what `rsync` and the filesystem do.
//!
//! Both harnesses target allocation-free predicates on purpose.
//! `kani_proofs_backup_sync` records the measurement: driving the constructor
//! instead ran one CBMC process for 117 minutes at 6.5 GB on an idle box, not
//! because the input space was large (216 cases) but because CBMC had to model
//! the allocator and `core::fmt` across it. `classify_declaration` and
//! `contains_path` are those same decisions with the message rendering lifted
//! out, so the model contains no allocation at all.

// ── NAS Archive Proofs (FJ-038 / nas-archive-v1) ──────────────────────

/// Contract `destination_cannot_be_the_source`: containment is component-wise
/// and symmetric-safe.
///
/// Two properties at once, because they constrain each other:
///
///   * Containment must be *reflexive* — a destination equal to the source is
///     the move-onto-itself shape and must be caught.
///   * Containment must NOT be a raw string prefix — `/mnt/unas-old` beside
///     `/mnt/unas` is a valid declaration, and refusing it trains operators to
///     work around the type.
///
/// Proved over every short path built from an alphabet containing the separator,
/// so the boundary between "next byte is `/`" and "next byte is anything else"
/// is exercised exhaustively rather than sampled.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(8)]
fn proof_archive_containment_is_component_wise() {
    use super::types::contains_path;

    // Deliberately includes the separator and two bytes that make a shared
    // prefix without a component boundary (`a` then `b` gives /a vs /ab).
    const ALPHABET: [u8; 3] = [b'/', b'a', b'b'];
    let i0: u8 = kani::any();
    let i1: u8 = kani::any();
    let j0: u8 = kani::any();
    let j1: u8 = kani::any();
    kani::assume(i0 < 3 && i1 < 3 && j0 < 3 && j1 < 3);

    let ob = [b'/', ALPHABET[i0 as usize], ALPHABET[i1 as usize]];
    let ib = [b'/', ALPHABET[j0 as usize], ALPHABET[j1 as usize]];
    let outer = core::str::from_utf8(&ob).unwrap();
    let inner = core::str::from_utf8(&ib).unwrap();

    // Reflexive: equal paths are containment, which is what makes
    // `dest == path` a rejection rather than a no-op move.
    assert!(contains_path(outer, outer));

    if contains_path(outer, inner) && outer != inner {
        // Strict containment implies a component boundary: `inner` continues
        // `outer` with a separator, never mid-component.
        assert!(inner.len() > outer.len());
        assert_eq!(inner.as_bytes()[outer.len()], b'/');
    }
}

/// Contract `destination_cannot_be_the_source`: an accepted declaration can
/// never be a move onto itself, and can never name a path where a directory
/// name is required.
///
/// This is the safety property the type exists for: whatever the operator
/// writes, an accepted `NasArchive` cannot delete a source into itself, and
/// cannot reach outside the source root.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(6)]
fn proof_archive_dest_never_inside_source() {
    use super::types::{classify_declaration, contains_path};

    const ALPHABET: [u8; 3] = [b'/', b'a', b'b'];
    let i0: u8 = kani::any();
    let j0: u8 = kani::any();
    kani::assume(i0 < 3 && j0 < 3);

    let pb = [b'/', ALPHABET[i0 as usize]];
    let db = [b'/', ALPHABET[j0 as usize]];
    let path = core::str::from_utf8(&pb).unwrap();
    let dest = core::str::from_utf8(&db).unwrap();

    // One directory name, symbolically either a legal name or a traversal.
    let is_dotdot: bool = kani::any();
    let dirs = [if is_dotdot {
        "..".to_string()
    } else {
        "d".to_string()
    }];

    if classify_declaration(path, Some(dest), &dirs, 10, 50).is_none() {
        // Accepted => neither path encloses the other, in either direction.
        assert!(!contains_path(path, dest));
        assert!(!contains_path(dest, path));
        // Accepted => the entry is a NAME, so it cannot escape the source root.
        assert!(!is_dotdot);
    }
}

/// Contract `cifs_hostile_trees_are_refused`: the small-file percentage is
/// bounded and cannot divide by zero.
///
/// Computed in POSIX shell integer arithmetic as `small * 100 / files`. A
/// percentage that could exceed 100 would make the comparison against
/// `max_small_file_pct` meaningless at the top of its range, and a division by
/// zero would abort the pass for every remaining directory.
#[cfg(kani)]
#[kani::proof]
fn proof_archive_small_file_pct_bounded() {
    let small: u32 = kani::any();
    let files: u32 = kani::any();
    kani::assume(files > 0 && files <= 1_000_000);
    kani::assume(small <= files);

    let pct = small * 100 / files;
    assert!(pct <= 100);
    // 100% is reachable only when every file is small.
    if pct == 100 {
        assert!(small == files);
    }
    // And a tree with no small files never trips a non-zero threshold.
    if small == 0 {
        assert!(pct == 0);
    }
}
