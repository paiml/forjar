//! FJ-861: `cloud_instance` — ONE declared spot instance, inert until gated.
//!
//! Nothing in this module calls a cloud. It *generates* shell; the shell runs
//! `aws` only when the machine it is applied on has it, and `apply_script`
//! refuses outright unless a parity-gate file, a budget cap and a kill switch
//! are all declared (and the gate file says pass, and the kill switch is
//! absent). `plan_diff` is pure Rust: no process, no network.

use crate::core::types::Resource;

#[cfg(test)]
mod tests;

/// A validated `cloud_instance` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudInstance {
    pub region: String,
    pub instance_type: String,
    pub max_price: String,
    pub image: String,
    pub max_lifetime_min: u32,
    pub tags: Vec<String>,
}

/// Resolve a validated [`CloudInstance`] from a resource declaration.
///
/// # Errors
///
/// Returns `Err` when the declaration is not aws, lacks a field, names an
/// image without an `@sha256:<64 hex>` digest, or has a non-positive price or
/// lifetime.
pub fn spec_of(_resource: &Resource) -> Result<CloudInstance, String> {
    Err("not implemented".to_string())
}

/// Declared-vs-intended diff, rendered with no network call.
pub fn plan_diff(_resource: &Resource) -> String {
    String::new()
}

/// Script asking the provider whether the instance exists (read-only).
pub fn check_script(_resource: &Resource) -> String {
    String::new()
}

/// Script that creates the instance, or refuses.
pub fn apply_script(_resource: &Resource) -> String {
    String::new()
}

/// Script reporting observed state (read-only provider query).
pub fn state_query_script(_resource: &Resource) -> String {
    String::new()
}
