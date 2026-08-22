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

// ── FVS-4: the error taxonomy ───────────────────────────────────────────
//
// Both harnesses below target `ErrorClass`, never `ForjarError`. That is the
// allocation-free boundary: `ForjarError` carries a `String`, so a harness that
// CONSTRUCTS one drags the allocator and `core::fmt` into the model — the
// measurement recorded in kani_proofs_backup_sync is 117 minutes at 6.5 GB for
// exactly that mistake, on an input space of 216 cases.
//
// The split is the same one nas_archive documents. Kani proves the decision
// algebra; EXECUTION proves the delegation. `ForjarError::exit_code` is one line
// — `self.class.exit_code()` — and that it ignores `message` is proved by
// `error::tests`, which builds real errors with different messages and compares
// codes. Modelling a String to prove a one-line delegation would buy nothing.

/// Every `ErrorClass` variant, selected symbolically.
///
/// The `match` is EXHAUSTIVE over the enum rather than a `%`-wrapped index, so
/// adding a sixth variant fails to compile here. A harness that silently stops
/// covering a new variant is worse than no harness: it reports totality over a
/// set that has grown behind it.
#[cfg(kani)]
fn any_class() -> crate::core::error::ErrorClass {
    use crate::core::error::ErrorClass as C;
    let i: u8 = kani::any();
    kani::assume(i < 5);
    match i {
        0 => C::Other,
        1 => C::Partial,
        2 => C::Validation,
        3 => C::Connection,
        _ => C::Drift,
    }
}

/// Contract `KANI-FVS-001`: classification is TOTAL — every class maps into the
/// published exit-code set `{1, 2, 3, 4, 10}`.
///
/// These values are a public contract: CI scripts branch on them. A class that
/// mapped to some sixth code would be a silent change to that contract, and
/// nothing else in the build would notice.
#[cfg(kani)]
#[kani::proof]
fn proof_fvs_classification_is_total() {
    let code = any_class().exit_code();
    assert!(
        matches!(code, 1 | 2 | 3 | 4 | 10),
        "a class maps outside the published exit-code set"
    );
}

/// Contract `KANI-FVS-002`: the exit code is a function of the VARIANT alone,
/// and distinct classes never collapse onto one code.
///
/// Injectivity is the half worth proving mechanically, because losing it is
/// precisely the defect this taxonomy replaced. The old classifier chose the
/// code by substring-matching the error PROSE, so every failure whose message
/// happened to contain "transport" collapsed onto 4 — the connection code CI
/// retries — including a deterministic bashrs rejection that fails identically
/// on every retry. A non-injective classifier cannot be acted on: the caller
/// cannot tell which failure it has.
///
/// `exit_code` takes `self: ErrorClass` by value, so it is structurally
/// incapable of reading a message. Injectivity plus that signature is the
/// allocation-free statement of "never of message length or content".
#[cfg(kani)]
#[kani::proof]
fn proof_fvs_classification_ignores_prose() {
    let a = any_class();
    let b = any_class();
    if a == b {
        // Determinism: the same variant always yields the same code, with no
        // other input in scope that could vary it.
        assert!(a.exit_code() == b.exit_code());
    } else {
        // Injectivity: two different classes never share a code.
        assert!(
            a.exit_code() != b.exit_code(),
            "two distinct error classes share an exit code — a caller cannot \
             tell them apart, which is the defect prose classification had"
        );
    }
}
