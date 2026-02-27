//! FJ-019: Recipe loading, input validation, and expansion into resources.
//!
//! Recipes are reusable, parameterized infrastructure patterns. A recipe
//! declares typed inputs and a set of resources. When instantiated, the
//! recipe's resources are expanded into the main config with namespaced IDs
//! (e.g., `my-recipe/resource-name`).

pub mod expansion;
pub mod types;
pub mod validation;
pub mod validation_types;

// Re-export public API so existing `use crate::core::recipe::*` keeps working.
pub use expansion::{expand_recipe, load_recipe, parse_recipe, recipe_terminal_id};
pub use types::*;
pub use validation::validate_inputs;

#[cfg(test)]
mod tests_expansion;
#[cfg(test)]
mod tests_integration;
#[cfg(test)]
mod tests_validation;
