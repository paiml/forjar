//! FJ-1300: Store path derivation — content-addressed paths from input hashes.
//!
//! Every store entry lives under `STORE_BASE/<hash>` where the hash is
//! deterministically computed from the recipe hash, input hashes, architecture,
//! and provider.  This mirrors the Nix store model (`/nix/store/<hash>-name`).

use crate::tripwire::hasher::composite_hash;

/// Base directory for the content-addressed store.
pub const STORE_BASE: &str = "/var/lib/forjar/store";

/// Compute a deterministic store hash from recipe inputs.
///
/// The hash is computed from sorted input components so that identical
/// inputs always produce the same store path regardless of argument order.
pub fn store_path(recipe_hash: &str, input_hashes: &[&str], arch: &str, provider: &str) -> String {
    let mut components: Vec<&str> = Vec::with_capacity(input_hashes.len() + 3);
    components.push(recipe_hash);
    let mut sorted_inputs: Vec<&str> = input_hashes.to_vec();
    sorted_inputs.sort();
    components.extend(sorted_inputs);
    components.push(arch);
    components.push(provider);
    composite_hash(&components)
}

/// Build the full store entry path from a store hash.
///
/// Returns `STORE_BASE/<hash>` (stripping the `blake3:` prefix).
pub fn store_entry_path(store_hash: &str) -> String {
    let hash_hex = store_hash.strip_prefix("blake3:").unwrap_or(store_hash);
    format!("{}/{}", STORE_BASE, hash_hex)
}
