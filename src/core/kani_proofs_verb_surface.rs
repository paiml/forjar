//! FVS: Kani bounded-model proofs for the unified verb surface (paiml/forjar#288).
//!
//! Gated behind `#[cfg(kani)]`; normal builds ignore them.
//! Run with: `cargo kani --harness proof_fvs_partition_is_total_and_disjoint`.
//!
//! ALLOCATION-FREE ON PURPOSE, like the nas_archive and backup_sync harnesses
//! beside it. That file records the measurement this one is shaped by: driving a
//! constructor instead of a predicate ran one CBMC process for 117 minutes at
//! 6.5 GB on an idle box — not because the input space was large, but because
//! CBMC had to model the allocator and `core::fmt` across it. These harnesses
//! never dereference a `&'static str` in the partition table; they read only the
//! bucket discriminant, so the model contains no string data and no allocation.
//!
//! WHAT THIS PROVES, AND WHAT IT DOES NOT. Kani proves the classification is
//! total and disjoint — every leaf carries exactly one bucket, over the whole
//! table rather than the entries a test happened to sample. It cannot prove the
//! table COVERS the shipped CLI, because that requires walking a `clap::Command`
//! tree built at runtime. That half is
//! `verb::partition::tests::the_partition_is_total`, which walks the live tree
//! and fails on an unbucketed leaf — verified by injection, and it caught
//! `rules serve` arriving from an unrelated merge. The two halves are
//! complementary and neither is sufficient alone.

// Gated with the harnesses: a bare `use` here is an unused import in every
// normal build, and this crate is clippy -D warnings.
#[cfg(kani)]
use crate::verb::Bucket;

/// Contract `KANI-FVS-003`: the CLI-leaf partition is total and disjoint —
/// every leaf is in exactly one of `{Unified, CliOnly, Pending}`.
///
/// Disjointness is the property worth proving mechanically. Totality at the
/// *type* level is enforced by the enum, but a future refactor that stores the
/// bucket as flags or a bitmask would lose it silently, and this harness fails
/// the moment two classifications can hold at once.
#[cfg(kani)]
#[kani::proof]
fn proof_fvs_partition_is_total_and_disjoint() {
    let table = crate::verb::partition();
    let i: usize = kani::any();
    kani::assume(i < table.len());

    // Only the discriminant is read. `CliOnly(_)` and `Pending(_)` deliberately
    // bind nothing: touching the &'static str would pull its bytes into the
    // model for no proof value.
    let b = &table[i].bucket;
    let unified = matches!(b, Bucket::Unified) as u8;
    let clionly = matches!(b, Bucket::CliOnly(_)) as u8;
    let pending = matches!(b, Bucket::Pending(_)) as u8;

    assert!(
        unified + clionly + pending == 1,
        "a CLI leaf is in zero or several buckets — the partition is not a partition"
    );
}

/// The table is not empty.
///
/// Stated separately because `proof_fvs_partition_is_total_and_disjoint`
/// assumes `i < len`, and on an empty table that assumption is unsatisfiable:
/// the harness would pass vacuously while classifying nothing. A proof that
/// holds because its premise cannot be met is the formal-methods form of a
/// green test over an empty set.
#[cfg(kani)]
#[kani::proof]
fn proof_fvs_partition_is_not_empty() {
    assert!(
        !crate::verb::partition().is_empty(),
        "the partition is empty — every totality proof over it is vacuous"
    );
}
