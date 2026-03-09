//! Core infrastructure logic — types, parsing, resolution, planning, execution.

pub mod cis_ubuntu_pack;
pub mod codegen;
pub mod compliance;
pub mod compliance_pack;
pub mod conditions;
pub mod cron_source;
pub mod ephemeral;
pub mod executor;
pub mod metric_source;
pub mod migrate;
pub mod parser;
pub mod planner;
pub mod plugin_dispatch;
pub mod plugin_hot_reload;
pub mod plugin_loader;
pub mod policy_boundary;
pub mod policy_coverage;
pub mod promotion;
pub mod promotion_events;
pub mod purifier;
pub mod recipe;
pub mod resolver;
pub mod rollout;
pub mod rules_engine;
pub mod rules_runtime;
pub mod scoring;
pub mod script_secret_lint;
pub mod secret_audit;
pub mod secret_namespace;
pub mod secret_provider;
pub mod secrets;
pub mod security_scanner;
pub mod shell_provider;
pub mod state;
pub mod state_encryption;
pub mod store;
pub mod task;
pub mod types;
pub mod webhook_source;

pub mod do330;
pub mod ferrocene;
pub mod flight_grade;
mod kani_production_proofs;
mod kani_proofs;
pub mod mcdc;
pub mod repro_build;
mod scoring_b;
#[cfg(test)]
mod tests_compliance;
#[cfg(test)]
mod tests_kani_proofs;
#[cfg(test)]
mod tests_policy_boundary;
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
