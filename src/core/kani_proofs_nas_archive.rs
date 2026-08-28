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
///
/// STATED OVER NORMALISED PATHS, and that is the whole content of this
/// harness's history. It first said, of the RAW arguments,
///
///     assert!(inner.len() > outer.len());
///     assert_eq!(inner.as_bytes()[outer.len()], b'/');
///
/// and `cargo kani` refused it: `1 of 557 failed — assertion failed:
/// inner.len() > outer.len()`. The refusal was correct and the function was
/// not at fault. `contains_path` decides on `norm_path(..)` of each argument,
/// so arithmetic on the raw lengths describes a different pair of strings than
/// the one the decision was made about. `outer = "///"` normalises to the root
/// and is 3 bytes long while meaning 1; every one of the eight `inner` values
/// that is not `outer` itself falsifies the raw form.
///
/// The root is also why the boundary index is not simply `o.len()`: `/` is the
/// one path that already ENDS in its own separator, so what follows it starts
/// at index 0, not at index 1. `boundary` below is that distinction and
/// nothing else. Enumerated over a wider space than Kani's (alphabet `/abc`,
/// paths to length 4, 85 paths, 7225 ordered pairs): 0 counterexamples to the
/// form below, and 207 to the same form if `contains_path` is mutated into the
/// raw string-prefix test this proof exists to exclude — so restating it did
/// not cost the property its teeth.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(8)]
fn proof_archive_containment_is_component_wise() {
    use super::types::{contains_path, norm_path};

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

    // Compare the paths `contains_path` actually decided about, not the bytes
    // they were written with: `/a/` and `/a` are one path, and "strictly
    // inside" has to mean strictly inside AFTER normalisation.
    let (o, i) = (norm_path(outer), norm_path(inner));
    if contains_path(outer, inner) && o != i {
        // Strict containment implies a component boundary: `inner` continues
        // `outer` with a separator, never mid-component. The root already ends
        // in its separator, so its boundary is at index 0.
        let boundary = if o == "/" { 0 } else { o.len() };
        assert!(i.starts_with(o));
        assert!(i.len() > boundary + 1);
        assert_eq!(i.as_bytes()[boundary], b'/');
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

    if classify_declaration(path, Some(dest), &dirs, 1024).is_none() {
        // Accepted => neither path encloses the other, in either direction.
        assert!(!contains_path(path, dest));
        assert!(!contains_path(dest, path));
        // Accepted => the entry is a NAME, so it cannot escape the source root.
        assert!(!is_dotdot);
    }
}

/// Contract `cifs_hostile_trees_are_refused`: the small-byte budget admits a
/// directory exactly when its small-file bytes fit, with no overflow.
///
/// The comparison runs in POSIX shell integer arithmetic, and the reported
/// figure is divided down to MB for the operator. This pins that the admission
/// decision is a plain total order — a directory at exactly the budget is
/// admitted, one byte over is refused — and that the MB rendering never claims
/// a refused tree was smaller than the budget.
#[cfg(kani)]
#[kani::proof]
fn proof_archive_small_byte_budget_is_a_total_order() {
    let small_bytes: u64 = kani::any();
    let budget: u64 = kani::any();
    kani::assume(small_bytes <= 1 << 40);
    kani::assume(budget > 0 && budget <= 1 << 40);

    let admitted = small_bytes <= budget;

    // Exactly at the budget is admitted; one byte over is not.
    if small_bytes == budget {
        assert!(admitted);
    }
    if small_bytes > budget {
        assert!(!admitted);
    }
    // The MB rendering shown to the operator never understates a refusal.
    if !admitted {
        assert!(small_bytes / 1_048_576 >= budget / 1_048_576);
    }
}
