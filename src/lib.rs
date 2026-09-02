//! Forjar — Rust-native Infrastructure as Code.
//!
//! Bare-metal first. BLAKE3 state hashing. Provenance tracing.
//! Faster, more provable, more sovereign than Terraform, Pulumi, or Ansible.
//!
//! # Where the contract code comes from
//!
//! `#[contract]` and the `build_helper` that verifies `contracts/binding.yaml`
//! are workspace members under `crates/forjar-contracts*` (forjar#423) — a
//! byte-faithful copy of `aprender-contracts` 0.31.2 with the library names
//! `provable_contracts` / `provable_contracts_macros` unchanged. Nothing
//! about contract code is fetched from a registry or a sibling checkout;
//! `cargo build --offline` proves it.
//!
//! # Using forjar as a library
//!
//! **Start at [`api`].** That module is the supported surface and the only part
//! of this crate covered by semver.
//!
//! Everything else — the seven modules below and roughly 1,844 `pub` items
//! beneath them — is reachable and documented, but it is implementation. It may
//! be renamed, moved or removed in a patch release. Depending on it is fine if
//! you know that; pin an exact version when you do (GH-240).

// Contract assertions from YAML (pv codegen)
#[macro_use]
#[allow(unused_macros)]
mod generated_contracts;
/// The supported, semver-covered library API. See the module docs.
pub mod api;
// `cli`, `mcp` and `verb` are one severable unit gated on the `cli` feature
// (GH-237). They are mutually recursive — mcp::handlers calls into cli, cli::infra
// calls mcp::{serve,export_schema}, verb::registry pulls both — so Cargo cannot
// express them as separate features today.
#[cfg(feature = "cli")]
pub mod cli;
pub mod copia;
pub mod core;
#[cfg(feature = "cli")]
pub mod mcp;
pub mod resources;
pub mod transport;
pub mod tripwire;
#[cfg(feature = "cli")]
pub mod verb;
