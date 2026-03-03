//! FJ-003: Template resolution and dependency DAG construction.
//!
//! Resolves `{{params.key}}` and `{{machine.name.field}}` templates.
//! Builds a DAG from explicit depends_on edges and computes topological order
//! using Kahn's algorithm with deterministic (alphabetical) tie-breaking.

mod dag;
mod data;
pub(crate) mod functions;
mod resource;
pub(crate) mod template;

pub use dag::{build_execution_order, compute_parallel_waves};
pub use data::resolve_data_sources;
pub use resource::resolve_resource_templates;
pub use template::resolve_template;

#[cfg(test)]
pub(super) use crate::core::types::*;

#[cfg(test)]
mod tests_dag;
#[cfg(test)]
mod tests_data;
#[cfg(test)]
mod tests_functions;
#[cfg(test)]
mod tests_helpers;
#[cfg(test)]
mod tests_proptest;
#[cfg(test)]
mod tests_resource;
#[cfg(test)]
mod tests_resource_b;
#[cfg(test)]
mod tests_template;
#[cfg(test)]
mod tests_waves;
