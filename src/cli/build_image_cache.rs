//! FJ-2403/E16: the build cache of `forjar build` — a BLAKE3 key over every
//! input of the layers, stored next to the layout. Split out of `build_image.rs`.

use super::build_image_base::hash_base_identity;
use super::build_image_exec::{hash_exec_inputs, ExecDecl};
use crate::core::store::image_assembler::BaseLayout;
use crate::core::store::layer_builder::LayerEntry;

/// FJ-2403/E16: Compute a BLAKE3 hash of all layer input content.
///
/// PMAT-989: the base layout's identity is part of the key, so a changed base
/// misses the cache and an unchanged one still hits.
pub(super) fn compute_layer_input_hash(
    layer_entries: &[Vec<LayerEntry>],
    base: Option<&BaseLayout>,
    exec: Option<&ExecDecl>,
    entrypoint: &[String],
) -> String {
    let mut hasher = blake3::Hasher::new();
    hash_base_identity(&mut hasher, base);
    hash_exec_inputs(&mut hasher, exec, entrypoint);
    for entries in layer_entries {
        for entry in entries {
            hasher.update(entry.path.as_bytes());
            hasher.update(&entry.content);
            hasher.update(&entry.mode.to_le_bytes());
        }
    }
    hasher.finalize().to_hex().to_string()
}

/// FJ-2403/E16: Check if a cached build with the same input hash exists.
/// Returns a cache-hit message if found, None otherwise.
pub(super) fn check_build_cache(output_dir: &std::path::Path, input_hash: &str) -> Option<String> {
    let cache_path = output_dir.join("build-cache.hash");
    let cached_hash = std::fs::read_to_string(&cache_path).ok()?;
    if cached_hash.trim() == input_hash {
        let metrics_path = output_dir.join("build-metrics.json");
        if metrics_path.exists() {
            return Some(format!(
                "Layer inputs unchanged (hash: {:.16}…), skipping rebuild",
                input_hash
            ));
        }
    }
    None
}

/// FJ-2403/E16: Write the input hash for cache checking on next build.
pub(super) fn write_build_cache(output_dir: &std::path::Path, input_hash: &str) {
    let cache_path = output_dir.join("build-cache.hash");
    let _ = std::fs::write(cache_path, input_hash);
}
