//! The supported library API.
//!
//! # Why this module exists
//!
//! GH-240: `forjar` exports seven top-level modules and roughly 1,844 `pub`
//! items across 195 `pub` modules, with nothing distinguishing the supported
//! API from an implementation detail that happens to be reachable. Not one item
//! was marked `doc(hidden)`. The CLI implementation is public. In that state
//! forjar cannot make a semver promise it is able to keep: any internal rename
//! is a breaking change for somebody, and nobody — including us — can say which
//! ones.
//!
//! GH-245 is the constructive half: a real consumer (rmedia, building course
//! assets against paiml/catalogue) wants forjar's build-staleness logic inline
//! in a Rust pipeline rather than shelled out to the CLI and re-parsed. They
//! named the exact surface they need, compiled it against this tree, and asked
//! only that we say it is supported so they need not pin `=1.13.2` and diff
//! every bump by hand.
//!
//! This module is that promise, and it is deliberately small.
//!
//! # The promise
//!
//! **Everything re-exported here follows semver.** A breaking change to any of
//! these items requires a major version bump and a changelog entry.
//!
//! **Nothing else does.** The rest of the crate is reachable, documented, and
//! useful — and it may be renamed, moved or removed in a patch release. If you
//! depend on an item that is not re-exported here, you are depending on an
//! internal, and that is fine as long as you know it. Pin an exact version.
//!
//! This is a narrow promise on purpose. A promise over 1,844 items would be one
//! we break by accident within a release, which is worse than no promise at all
//! because it reads as a guarantee.
//!
//! # Deliberately absent
//!
//! [`crate::tripwire::hasher::composite_hash`] is **not** here. It is now
//! injective (GH-235), but the fix changed every digest it produces, so it has
//! no stability history yet. `hash_inputs` and `hash_outputs_in` funnel through
//! it, which is why they are documented below as content-identity signals whose
//! *values* are not stable across major versions — their comparison semantics
//! are.
//!
//! # Example
//!
//! Deciding whether a build artifact needs regenerating:
//!
//! ```no_run
//! use forjar::api::{hash_file, probe_resource, staleness_reason, Resource};
//! use std::path::Path;
//!
//! let content_id = hash_file(Path::new("lesson.srt"))?;
//! println!("srt identity: {content_id}");
//!
//! let resource = Resource::default();
//! if let Some(probe) = probe_resource(&resource) {
//!     // `None` recorded hash deliberately means "re-run once to establish a
//!     // baseline", not "nothing to compare, therefore fresh".
//!     match staleness_reason(&probe, None, None) {
//!         Some(why) => println!("rebuild: {why}"),
//!         None => println!("fresh"),
//!     }
//! }
//! # Ok::<(), String>(())
//! ```

// ── Content identity ────────────────────────────────────────────────────
//
// Raw-byte content identity. Deterministic, `blake3:`-prefixed, and sensitive
// to any byte change. Does not go through `composite_hash`, so it is unaffected
// by the GH-235 framing change.
pub use crate::tripwire::hasher::hash_file;

// ── Build staleness ─────────────────────────────────────────────────────
//
// The decision "does this artifact need regenerating?", with the branch
// ordering that was earned from real bugs:
//
// * `outputs_missing` is a flag distinct from `output_hash`, and is checked
//   FIRST — "absent" and "present but different" are different facts, and
//   letting the second alias the first is how a missing artifact gets reported
//   as an unchanged one.
// * A missing recorded baseline means "re-run once to establish one", NOT
//   "fresh". Getting that backwards is the classic cache bug: a corrected
//   source file silently fails to trigger a rebuild.
pub use crate::core::task::probe::{probe_all, probe_resource, staleness_reason, IoDigest};

// Glob expansion and base-directory resolution over a declared I/O spec.
//
// STABILITY NOTE: the comparison semantics are covered by this promise — equal
// hashes mean unchanged inputs, and that will keep holding. The literal hash
// VALUES are not, because these funnel through `composite_hash`, whose framing
// changed in GH-235 and could change again. Do not persist these values across
// a major version and expect them to compare equal; re-probe instead.
pub use crate::core::task::{hash_inputs, hash_outputs_in};

// ── Change propagation ──────────────────────────────────────────────────
//
// The no-early-cutoff rule: a dirty upstream resource promotes its NoOp
// dependents in a single topological sweep, so a downstream artifact is never
// left stale because the traversal stopped at the first unchanged node.
pub use crate::core::planner::propagation::propagate_changes;

// ── Argument types ──────────────────────────────────────────────────────
pub use crate::core::types::{PlanAction, PlannedChange, Resource};

#[cfg(test)]
#[path = "tests_api.rs"]
mod tests;
