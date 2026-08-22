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
pub mod http;
pub mod partition;
pub mod registry;
pub mod spec;

pub use partition::{partition, Bucket, Leaf};

/// Render a verb's result. EVERY transport calls this.
///
/// The renderer-fidelity gate (`http_and_cli_return_identical_bytes`) asserts
/// that HTTP and the CLI produce the same bytes for the same invocation. That
/// property is easy to assert and easy to lose: two transports can expose the
/// same verb and quietly disagree about how they print it. Routing both through
/// one function makes the property true by construction, and the test then
/// guards the construction rather than a coincidence.
pub fn render_result(v: &serde_json::Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}
pub use registry::{find, verbs};
pub use spec::{Effects, VerbSpec};
