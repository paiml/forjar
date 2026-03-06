//! FJ-1300: Content-addressed store — path derivation, metadata, profiles, purity.
//!
//! Implements a Nix-inspired content-addressed store model where every build
//! output is placed under a deterministic path derived from its inputs.

pub mod cache;
pub mod cache_exec;
pub mod chunker;
pub mod convergence_runner;
pub mod db;
pub mod closure;
pub mod ingest;
pub mod conda;
pub mod contract_coverage;
pub mod contract_scaffold;
pub mod convert;
pub mod convert_exec;
pub mod derivation;
pub mod derivation_exec;
pub mod far;
pub mod gc;
pub mod gc_exec;
pub mod hf_config;
pub mod kernel_far;
pub mod lockfile;
pub mod meta;
pub mod mutation_runner;
pub mod path;
pub mod pin_resolve;
pub mod pin_tripwire;
pub mod profile;
pub mod provider;
pub mod provider_exec;
pub mod purity;
pub mod reference;
pub mod registry_push;
pub mod repro_score;
pub mod sandbox;
pub mod sandbox_exec;
pub mod sandbox_run;
pub mod secret_scan;
pub mod store_diff;
pub mod substitution;
pub mod sync_exec;
pub mod validate;

#[cfg(test)]
mod tests_bash_provability;
#[cfg(test)]
mod tests_cache;
#[cfg(test)]
mod tests_cache_exec;
#[cfg(test)]
mod tests_chunker;
#[cfg(test)]
mod tests_db;
#[cfg(test)]
mod tests_ingest;
#[cfg(test)]
mod tests_closure;
#[cfg(test)]
mod tests_convergence_runner;
#[cfg(test)]
mod tests_conda;
#[cfg(test)]
mod tests_contract_coverage;
#[cfg(test)]
mod tests_contract_scaffold;
#[cfg(test)]
mod tests_convert;
#[cfg(test)]
mod tests_convert_exec;
#[cfg(test)]
mod tests_cov_exec;
#[cfg(test)]
mod tests_derivation;
#[cfg(test)]
mod tests_derivation_exec;
#[cfg(test)]
mod tests_falsify_spec_a;
#[cfg(test)]
mod tests_falsify_spec_b;
#[cfg(test)]
mod tests_falsify_spec_c;
#[cfg(test)]
mod tests_falsify_spec_d;
#[cfg(test)]
mod tests_falsify_spec_e;
#[cfg(test)]
mod tests_falsify_spec_f;
#[cfg(test)]
mod tests_falsify_spec_g;
#[cfg(test)]
mod tests_falsify_spec_h;
#[cfg(test)]
mod tests_falsify_spec_i;
#[cfg(test)]
mod tests_falsify_spec_j;
#[cfg(test)]
mod tests_far;
#[cfg(test)]
mod tests_gc;
#[cfg(test)]
mod tests_gc_exec;
#[cfg(test)]
mod tests_hf_config;
#[cfg(test)]
mod tests_kernel_far;
#[cfg(test)]
mod tests_lockfile;
#[cfg(test)]
mod tests_meta;
#[cfg(test)]
mod tests_mutation_runner;
#[cfg(test)]
mod tests_path;
#[cfg(test)]
mod tests_pin_resolve;
#[cfg(test)]
mod tests_pin_tripwire;
#[cfg(test)]
mod tests_profile;
#[cfg(test)]
mod tests_provider;
#[cfg(test)]
mod tests_provider_exec;
#[cfg(test)]
mod tests_purity;
#[cfg(test)]
mod tests_reference;
#[cfg(test)]
mod tests_registry_push;
#[cfg(test)]
mod tests_repro_score;
#[cfg(test)]
mod tests_sandbox;
#[cfg(test)]
mod tests_sandbox_exec;
#[cfg(test)]
mod tests_sandbox_run;
#[cfg(test)]
mod tests_secret_scan;
#[cfg(test)]
mod tests_store_diff;
#[cfg(test)]
mod tests_substitution;
#[cfg(test)]
mod tests_sync_exec;
#[cfg(test)]
mod tests_validate;
