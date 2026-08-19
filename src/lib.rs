//! Forjar — Rust-native Infrastructure as Code.
//!
//! Bare-metal first. BLAKE3 state hashing. Provenance tracing.
//! Faster, more provable, more sovereign than Terraform, Pulumi, or Ansible.
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
pub mod cli;
pub mod copia;
pub mod core;
pub mod mcp;
pub mod resources;
pub mod transport;
pub mod tripwire;
