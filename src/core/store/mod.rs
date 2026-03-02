//! FJ-1300: Content-addressed store — path derivation, metadata, profiles, purity.
//!
//! Implements a Nix-inspired content-addressed store model where every build
//! output is placed under a deterministic path derived from its inputs.

pub mod path;
pub mod meta;
pub mod profile;
pub mod reference;
pub mod purity;
pub mod far;
pub mod chunker;
pub mod conda;
pub mod hf_config;
pub mod contract_coverage;
pub mod contract_scaffold;
pub mod kernel_far;
pub mod closure;
pub mod lockfile;
pub mod gc;
pub mod repro_score;
pub mod sandbox;
pub mod cache;
pub mod provider;
pub mod derivation;
pub mod store_diff;
pub mod convert;
pub mod pin_tripwire;
pub mod validate;
pub mod sandbox_exec;
pub mod substitution;
pub mod derivation_exec;

#[cfg(test)]
mod tests_path;
#[cfg(test)]
mod tests_meta;
#[cfg(test)]
mod tests_profile;
#[cfg(test)]
mod tests_reference;
#[cfg(test)]
mod tests_purity;
#[cfg(test)]
mod tests_far;
#[cfg(test)]
mod tests_chunker;
#[cfg(test)]
mod tests_conda;
#[cfg(test)]
mod tests_hf_config;
#[cfg(test)]
mod tests_contract_coverage;
#[cfg(test)]
mod tests_contract_scaffold;
#[cfg(test)]
mod tests_kernel_far;
#[cfg(test)]
mod tests_closure;
#[cfg(test)]
mod tests_lockfile;
#[cfg(test)]
mod tests_gc;
#[cfg(test)]
mod tests_repro_score;
#[cfg(test)]
mod tests_sandbox;
#[cfg(test)]
mod tests_cache;
#[cfg(test)]
mod tests_provider;
#[cfg(test)]
mod tests_derivation;
#[cfg(test)]
mod tests_store_diff;
#[cfg(test)]
mod tests_convert;
#[cfg(test)]
mod tests_pin_tripwire;
#[cfg(test)]
mod tests_validate;
#[cfg(test)]
mod tests_sandbox_exec;
#[cfg(test)]
mod tests_substitution;
#[cfg(test)]
mod tests_derivation_exec;
