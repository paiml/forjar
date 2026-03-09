//! FJ-001: All types from the forjar specification.
//!
//! Defines the YAML schema types for machines, resources, policy, state locks,
//! and provenance events. All types derive Serialize/Deserialize for YAML roundtripping.

mod behavior_types;
mod build_metrics;
mod ci_pipeline_types;
mod config;
mod container_build_types;
mod contract_tier_types;
mod coverage_types;
mod distribution_types;
mod doctor_types;
pub mod environment;
mod event_types;
mod generation_diff_types;
mod generation_types;
mod handler_contract_types;
mod image_log_types;
mod mutation_types;
mod observability_types;
mod oci_types;
mod plugin_types;
mod policy;
mod policy_rule_types;
mod query_types;
pub mod refinement;
mod resource;
pub(crate) mod resource_enums;
mod run_log_types;
mod security_types;
mod service_mode_types;
mod sqlite_schema_types;
mod state_types;
mod task_types;
mod test_runner_types;
#[cfg(test)]
mod tests_container_build;
#[cfg(test)]
mod tests_mutation;
#[cfg(test)]
mod tests_run_log;
#[cfg(test)]
mod tests_task;
#[cfg(test)]
mod tests_validation_types;
mod undo_types;
mod validation_types;
mod wasm_types;

pub use behavior_types::*;
pub use build_metrics::*;
pub use ci_pipeline_types::*;
pub use config::*;
pub use container_build_types::*;
pub use contract_tier_types::*;
pub use coverage_types::*;
pub use distribution_types::*;
pub use doctor_types::*;
pub use environment::*;
pub use event_types::*;
pub use generation_diff_types::*;
pub use generation_types::*;
pub use handler_contract_types::*;
pub use image_log_types::*;
pub use mutation_types::*;
pub use observability_types::*;
pub use oci_types::*;
pub use plugin_types::*;
pub use policy::*;
pub use policy_rule_types::*;
pub use query_types::*;
pub use resource::*;
pub use resource_enums::*;
pub use run_log_types::*;
pub use security_types::*;
pub use service_mode_types::*;
pub use sqlite_schema_types::*;
pub use state_types::*;
pub use task_types::*;
pub use test_runner_types::*;
pub use undo_types::*;
pub use validation_types::*;
pub use wasm_types::*;

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
mod tests_environment;
#[cfg(test)]
mod tests_oci_types;
#[cfg(test)]
pub(crate) mod tests_proptest_resource;
#[cfg(test)]
mod tests_resource;
#[cfg(test)]
mod tests_state;
