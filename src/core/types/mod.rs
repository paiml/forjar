//! FJ-001: All types from the forjar specification.
//!
//! Defines the YAML schema types for machines, resources, policy, state locks,
//! and provenance events. All types derive Serialize/Deserialize for YAML roundtripping.

mod config;
mod policy;
pub mod refinement;
mod resource;
mod state_types;
mod task_types;
mod coverage_types;
mod run_log_types;
mod behavior_types;
mod generation_types;
mod build_metrics;
mod oci_types;
mod doctor_types;
mod query_types;
mod undo_types;
mod distribution_types;
mod security_types;
mod validation_types;
mod mutation_types;
mod contract_tier_types;
mod wasm_types;
mod observability_types;
mod container_build_types;
mod generation_diff_types;
mod service_mode_types;
mod test_runner_types;
mod handler_contract_types;

pub use config::*;
pub use policy::*;
pub use resource::*;
pub use state_types::*;
pub use task_types::*;
pub use coverage_types::*;
pub use run_log_types::*;
pub use behavior_types::*;
pub use generation_types::*;
pub use build_metrics::*;
pub use oci_types::*;
pub use doctor_types::*;
pub use query_types::*;
pub use undo_types::*;
pub use distribution_types::*;
pub use security_types::*;
pub use validation_types::*;
pub use mutation_types::*;
pub use contract_tier_types::*;
pub use wasm_types::*;
pub use observability_types::*;
pub use container_build_types::*;
pub use generation_diff_types::*;
pub use service_mode_types::*;
pub use test_runner_types::*;
pub use handler_contract_types::*;

// Shared default functions used by serde across multiple submodules.
fn default_true() -> bool {
    true
}

fn default_one() -> u32 {
    1
}

#[cfg(test)]
mod tests_config;
#[cfg(test)]
pub(crate) mod tests_proptest_resource;
#[cfg(test)]
mod tests_resource;
#[cfg(test)]
mod tests_state;
