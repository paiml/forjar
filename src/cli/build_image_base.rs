//! IMG-001 (forjar#756): base resolution and the build-cache key for a
//! files-only `forjar build`. The base is resolved BEFORE the cache is
//! consulted and its identity is folded into the input hash, so a layout first
//! built as scratch is never served as the based image.

use crate::core::store::image_assembler::BaseLayout;
use crate::core::store::layer_builder::LayerEntry;
use crate::core::types::ImageBuildPlan;

/// Resolve the plan's declared base (`None` when it declares none). A base that
/// cannot be produced, or fails its pinned digest, is an error: it never
/// becomes a scratch image. `state_images` is where local layouts live.
pub(super) fn resolve_plan_base(
    plan: &ImageBuildPlan,
    state_images: &std::path::Path,
    work_dir: &std::path::Path,
    runtime: Option<&str>,
) -> Result<Option<BaseLayout>, String> {
    let Some(name) = plan.base_image.as_deref() else {
        return Ok(None);
    };
    let b = crate::core::store::base_resolve::resolve_build_base(
        name,
        state_images,
        work_dir,
        runtime,
    )?;
    // Human text goes to stderr so `--json` owns stdout.
    eprintln!(
        "  {}",
        crate::core::store::base_image::format_base_info(name, &b.layers)
    );
    Ok(Some(b))
}

/// The container runtime to export a base from, if one is installed.
pub(super) fn detect_runtime() -> Option<&'static str> {
    ["docker", "podman"]
        .into_iter()
        .find(|r| super::dispatch_misc_b::which_runtime(r))
}

/// Identity of the base for the cache key: an explicit marker for none, else the
/// base's layer digests, diff_ids and platform.
fn base_identity(base: Option<&BaseLayout>) -> String {
    let Some(b) = base else {
        return "base:none".to_string();
    };
    let mut h = blake3::Hasher::new();
    for l in &b.layers.layers {
        h.update(l.digest.as_bytes());
        h.update(b"\n");
    }
    for d in &b.layers.diff_ids {
        h.update(d.as_bytes());
        h.update(b"\n");
    }
    h.update(b.layers.architecture.as_bytes());
    h.update(b.layers.os.as_bytes());
    format!("base:{}", h.finalize().to_hex())
}

/// FJ-2403/E16: Compute a BLAKE3 hash of all layer input content.
pub(super) fn compute_layer_input_hash(
    layer_entries: &[Vec<LayerEntry>],
    base: Option<&BaseLayout>,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(base_identity(base).as_bytes());
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
