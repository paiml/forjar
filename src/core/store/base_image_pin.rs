//! IMG-001 C1: pinned base-image layer extraction (split out of `base_image.rs`
//! to keep that file under the file-health line limit).

use super::base_image::{read_blob, resolve_manifest_digest, BaseImageLayers};
use crate::core::types::{OciImageConfig, OciIndex, OciManifest};
use std::path::Path;

/// Like [`extract_base_layers`], but when `pin` is a manifest digest present at the
/// top of `index.json`, layers and config come from THAT manifest rather than
/// `manifests[0]` (IMG-001 C1). A pin that matches no top entry (an image-index
/// pin) keeps the default entry; the caller has already proven it.
pub(crate) fn extract_base_layers_pinned(
    layout_dir: &Path,
    pin: Option<&str>,
) -> Result<BaseImageLayers, String> {
    // 1. Read and parse index.json
    let index_path = layout_dir.join("index.json");
    let index_data = std::fs::read(&index_path).map_err(|e| format!("read index.json: {e}"))?;
    let index: OciIndex =
        serde_json::from_slice(&index_data).map_err(|e| format!("parse index.json: {e}"))?;

    if index.manifests.is_empty() {
        return Err("index.json has no manifests".into());
    }

    let top = pin
        .and_then(|p| index.manifests.iter().find(|m| m.digest == p))
        .unwrap_or(&index.manifests[0]);

    // 2. Read the first manifest; if it is itself an index (multi-platform
    //    image), resolve it to the linux/<host arch> manifest (or its first entry).
    let manifest_digest = resolve_manifest_digest(layout_dir, &top.digest, 0)
        .map_err(crate::core::error::ForjarError::into_untyped)?;
    let manifest_data = read_blob(layout_dir, &manifest_digest)?;
    let manifest: OciManifest =
        serde_json::from_slice(&manifest_data).map_err(|e| format!("parse manifest: {e}"))?;

    // 3. Read the image config
    let config_data = read_blob(layout_dir, &manifest.config.digest)?;
    let config: OciImageConfig =
        serde_json::from_slice(&config_data).map_err(|e| format!("parse image config: {e}"))?;

    // 4. Validate layer count matches diff_ids
    if manifest.layers.len() != config.rootfs.diff_ids.len() {
        return Err(format!(
            "layer count mismatch: manifest has {} layers, config has {} diff_ids",
            manifest.layers.len(),
            config.rootfs.diff_ids.len(),
        ));
    }

    Ok(BaseImageLayers {
        layers: manifest.layers,
        diff_ids: config.rootfs.diff_ids,
        history: config.history,
        architecture: config.architecture,
        os: config.os,
        config: config.config,
    })
}
