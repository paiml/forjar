//! Core infrastructure logic — types, parsing, resolution, planning, execution.

pub mod codegen;
pub mod compliance;
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
pub mod security_scanner;
pub mod state;
pub mod store;
pub mod task;
pub mod types;

pub mod do330;
pub mod ferrocene;
pub mod flight_grade;
mod kani_proofs;
pub mod mcdc;
pub mod repro_build;
mod scoring_b;
#[cfg(test)]
mod tests_compliance;
#[cfg(test)]
mod tests_kani_proofs;
#[cfg(test)]
mod tests_proptest_convergence;
#[cfg(test)]
mod tests_proptest_handlers;
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
#[cfg(all(test, feature = "encryption"))]
mod tests_secrets;
#[cfg(test)]
mod tests_security_scanner;
mod verus_spec;
