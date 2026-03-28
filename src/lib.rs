//! Forjar — Rust-native Infrastructure as Code.
//!
//! Bare-metal first. BLAKE3 state hashing. Provenance tracing.
//! Faster, more provable, more sovereign than Terraform, Pulumi, or Ansible.

// Contract assertions from YAML (pv codegen)
#[macro_use]
#[allow(unused_macros)]
mod generated_contracts;
pub mod cli;
pub mod copia;
pub mod core;
pub mod mcp;
pub mod resources;
pub mod transport;
pub mod tripwire;
