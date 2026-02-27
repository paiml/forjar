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
pub mod secrets;
pub mod state;
pub mod types;

#[cfg(test)]
mod tests_purifier;
#[cfg(test)]
mod tests_secrets;
