//! Whether unknown YAML fields are fatal for a given config load (GH-272).
//!
//! Kept in its own module so the decision has exactly one home. `validate` used
//! to carry an inline copy of this check while every other verb went through a
//! permissive path — two implementations, one of them wrong, which is precisely
//! how the two commands came to disagree about whether the same file was valid.

/// Whether unknown fields are fatal for this load.
///
/// Total and allocation-free so KANI-CLC-001 can address it directly; a harness
/// aimed at the caller would have to reason through a file read and a per-field
/// diagnostic allocation. Production calls this — it is the decision, not a
/// model of it. Contract: contracts/config-load-consistency-v1.yaml
pub const fn rejects_unknown(deny_unknown: bool, unknown_count: usize) -> bool {
    deny_unknown && unknown_count > 0
}
