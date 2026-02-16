//! Forjar — Rust-native Infrastructure as Code.
//!
//! Bare-metal first. BLAKE3 state hashing. Provenance tracing.
//! Faster, more provable, more sovereign than Terraform, Pulumi, or Ansible.

pub mod cli;
pub mod core;
pub mod resources;
pub mod transport;
pub mod tripwire;
