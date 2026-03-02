//! FJ-1300: Content-addressed store — path derivation, metadata, profiles, purity.
//!
//! Implements a Nix-inspired content-addressed store model where every build
//! output is placed under a deterministic path derived from its inputs.

pub mod path;
pub mod meta;
pub mod profile;
pub mod reference;
pub mod purity;

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
