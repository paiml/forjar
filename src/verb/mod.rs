//! The forjar Unified Verb Surface (FVS).
//!
//! Spec: `docs/specifications/unified-verb-surface.md`
//! Contract: `contracts/verb-surface-v1.yaml`
//! Tracking: paiml/forjar#288
//!
//! A verb is declared exactly once in [`registry`], and every transport renders
//! its own view of that declaration. [`partition`] accounts for all 193 CLI
//! leaves so the boundary of this surface is stated rather than implied.
//!
//! The lesson this module is shaped around: a sibling project's four-way parity
//! suite stayed green for its entire life while its transports had no caller
//! from `main.rs`. Agreement between transports cannot falsify reachability. So
//! the derived tree SHIPS — `forjar verb` is a real subcommand — and an
//! unreachable surface becomes a user-visible break instead of a green test.

pub mod cli;
pub mod partition;
pub mod registry;
pub mod spec;

pub use partition::{partition, Bucket, Leaf};
pub use registry::{find, verbs};
pub use spec::{Effects, VerbSpec};
