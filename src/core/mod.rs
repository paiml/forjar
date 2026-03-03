//! Core infrastructure logic — types, parsing, resolution, planning, execution.

pub mod codegen;
pub mod conditions;
pub mod executor;
pub mod migrate;
pub mod parser;
pub mod planner;
pub mod purifier;
pub mod recipe;
pub mod resolver;
pub mod scoring;
pub mod secrets;
pub mod state;
pub mod store;
pub mod types;

mod scoring_b;
#[cfg(test)]
mod tests_proptest_idempotency;
#[cfg(test)]
mod tests_purifier;
#[cfg(test)]
mod tests_purifier_b;
#[cfg(test)]
mod tests_scoring;
#[cfg(test)]
mod tests_scoring_b;
#[cfg(test)]
mod tests_secrets;
