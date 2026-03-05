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

pub use config::*;
pub use policy::*;
pub use resource::*;
pub use state_types::*;
pub use task_types::*;
pub use coverage_types::*;
pub use run_log_types::*;
pub use behavior_types::*;

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
