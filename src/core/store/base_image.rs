//! FJ-2104: Base image layer extraction from OCI layout directories.
//!
//! Reads an existing OCI layout (e.g., pulled via `skopeo copy`) to extract
//! layer descriptors and diff_ids for use as base layers in `assemble_image()`.

use crate::core::types::{OciDescriptor, OciImageConfig, OciIndex, OciManifest};
use std::path::Path;

/// Extracted base image information.
#[derive(Debug, Clone)]
pub struct BaseImageLayers {
    /// Layer descriptors from the base image manifest (ordered bottom→top).
    pub layers: Vec<OciDescriptor>,
    /// DiffIDs from the base image config (uncompressed SHA-256).
    pub diff_ids: Vec<String>,
    /// History from the base image config.
    pub history: Vec<crate::core::types::OciHistoryEntry>,
    /// Architecture (e.g., "amd64").
    pub architecture: String,
    /// OS (e.g., "linux").
    pub os: String,
    /// Runtime config from base image.
    pub config: crate::core::types::OciRuntimeConfig,
}

/// Read an OCI layout directory and extract base image layer information.
///
/// The layout directory must contain:
/// - `index.json` — OCI index pointing to manifest
/// - `blobs/sha256/...` — manifest and config blobs
///
/// # Examples
///
/// ```no_run
/// use forjar::core::store::base_image::extract_base_layers;
///
/// let layers = extract_base_layers(std::path::Path::new("state/images/ubuntu")).unwrap();
/// assert!(!layers.layers.is_empty());
/// assert_eq!(layers.layers.len(), layers.diff_ids.len());
/// ```
pub fn extract_base_layers(layout_dir: &Path) -> Result<BaseImageLayers, String> {
    // 1. Read and parse index.json
    let index_path = layout_dir.join("index.json");
    let index_data = std::fs::read(&index_path)
        .map_err(|e| format!("read index.json: {e}"))?;
    let index: OciIndex = serde_json::from_slice(&index_data)
        .map_err(|e| format!("parse index.json: {e}"))?;

    if index.manifests.is_empty() {
        return Err("index.json has no manifests".into());
    }

    // 2. Read the first manifest
    let manifest_desc = &index.manifests[0];
    let manifest_data = read_blob(layout_dir, &manifest_desc.digest)?;
    let manifest: OciManifest = serde_json::from_slice(&manifest_data)
        .map_err(|e| format!("parse manifest: {e}"))?;

    // 3. Read the image config
    let config_data = read_blob(layout_dir, &manifest.config.digest)?;
    let config: OciImageConfig = serde_json::from_slice(&config_data)
        .map_err(|e| format!("parse image config: {e}"))?;

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

/// Read a blob from the OCI layout's blobs directory.
fn read_blob(layout_dir: &Path, digest: &str) -> Result<Vec<u8>, String> {
    let hex = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| format!("unsupported digest algorithm: {digest}"))?;
    let blob_path = layout_dir.join(format!("blobs/sha256/{hex}"));
    std::fs::read(&blob_path)
        .map_err(|e| format!("read blob {digest}: {e}"))
}

/// Verify that all base image layer blobs exist in the layout directory.
///
/// Returns a list of missing blob digests.
pub fn verify_base_blobs(layout_dir: &Path, layers: &BaseImageLayers) -> Vec<String> {
    layers
        .layers
        .iter()
        .filter(|layer| {
            let hex = layer.digest.strip_prefix("sha256:").unwrap_or(&layer.digest);
            !layout_dir.join(format!("blobs/sha256/{hex}")).exists()
        })
        .map(|layer| layer.digest.clone())
        .collect()
}

/// Copy base image layer blobs from source layout to destination layout.
///
/// Only copies blobs that don't already exist in the destination.
pub fn copy_base_blobs(
    src_layout: &Path,
    dst_layout: &Path,
    layers: &BaseImageLayers,
) -> Result<u64, String> {
    let dst_blobs = dst_layout.join("blobs/sha256");
    std::fs::create_dir_all(&dst_blobs)
        .map_err(|e| format!("create destination blobs dir: {e}"))?;

    let mut bytes_copied: u64 = 0;
    for layer in &layers.layers {
        let hex = layer.digest.strip_prefix("sha256:").unwrap_or(&layer.digest);
        let dst_path = dst_blobs.join(hex);
        if dst_path.exists() {
            continue;
        }
        let src_path = src_layout.join(format!("blobs/sha256/{hex}"));
        std::fs::copy(&src_path, &dst_path)
            .map_err(|e| format!("copy blob {}: {e}", layer.digest))?;
        bytes_copied += layer.size;
    }
    Ok(bytes_copied)
}

/// Format base image info for human output.
pub fn format_base_info(base_ref: &str, layers: &BaseImageLayers) -> String {
    let total_size: u64 = layers.layers.iter().map(|l| l.size).sum();
    format!(
        "Base: {} ({}/{}, {} layers, {:.1} MB)",
        base_ref,
        layers.architecture,
        layers.os,
        layers.layers.len(),
        total_size as f64 / (1024.0 * 1024.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{OciDescriptor, OciImageConfig, OciIndex, OciManifest};

    const CONFIG_HEX: &str = "aabbccdd00112233445566778899aabbccddeeff00112233445566778899aabb";
    const LAYER_HEX: &str = "11223344556677889900aabbccddeeff00112233445566778899aabbccddeeff";
    const MANIFEST_HEX: &str = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";

    fn create_test_layout(dir: &Path) {
        let blobs = dir.join("blobs/sha256");
        std::fs::create_dir_all(&blobs).unwrap();

        // Config
        let config = OciImageConfig::linux_amd64(vec!["sha256:diff1".into()]);
        let config_json = serde_json::to_vec(&config).unwrap();
        std::fs::write(blobs.join(CONFIG_HEX), &config_json).unwrap();

        // Layer blob
        std::fs::write(blobs.join(LAYER_HEX), b"fake-layer-data").unwrap();

        // Manifest
        let manifest = OciManifest::new(
            format!("sha256:{CONFIG_HEX}"),
            vec![OciDescriptor::gzip_layer(format!("sha256:{LAYER_HEX}"), 15)],
        );
        let manifest_json = serde_json::to_vec(&manifest).unwrap();
        std::fs::write(blobs.join(MANIFEST_HEX), &manifest_json).unwrap();

        // Index
        let index = OciIndex::single(OciDescriptor {
            media_type: "application/vnd.oci.image.manifest.v1+json".into(),
            digest: format!("sha256:{MANIFEST_HEX}"),
            size: manifest_json.len() as u64,
            annotations: std::collections::HashMap::new(),
        });
        std::fs::write(
            dir.join("index.json"),
            serde_json::to_vec_pretty(&index).unwrap(),
        ).unwrap();

        std::fs::write(dir.join("oci-layout"), r#"{"imageLayoutVersion":"1.0.0"}"#).unwrap();
    }

    #[test]
    fn extract_base_layers_valid() {
        let dir = tempfile::tempdir().unwrap();
        create_test_layout(dir.path());
        let result = extract_base_layers(dir.path()).unwrap();
        assert_eq!(result.layers.len(), 1);
        assert_eq!(result.diff_ids.len(), 1);
        assert_eq!(result.architecture, "amd64");
        assert_eq!(result.os, "linux");
    }

    #[test]
    fn extract_base_layers_missing_index() {
        let dir = tempfile::tempdir().unwrap();
        let result = extract_base_layers(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("index.json"));
    }

    #[test]
    fn verify_base_blobs_all_present() {
        let dir = tempfile::tempdir().unwrap();
        create_test_layout(dir.path());
        let layers = extract_base_layers(dir.path()).unwrap();
        let missing = verify_base_blobs(dir.path(), &layers);
        assert!(missing.is_empty());
    }

    #[test]
    fn verify_base_blobs_missing() {
        let dir = tempfile::tempdir().unwrap();
        create_test_layout(dir.path());
        let mut layers = extract_base_layers(dir.path()).unwrap();
        // Add a fake layer that doesn't exist
        layers.layers.push(OciDescriptor::gzip_layer("sha256:nonexistent".into(), 100));
        let missing = verify_base_blobs(dir.path(), &layers);
        assert_eq!(missing.len(), 1);
        assert!(missing[0].contains("nonexistent"));
    }

    #[test]
    fn copy_base_blobs_to_new_dir() {
        let src = tempfile::tempdir().unwrap();
        create_test_layout(src.path());
        let layers = extract_base_layers(src.path()).unwrap();

        let dst = tempfile::tempdir().unwrap();
        let bytes = copy_base_blobs(src.path(), dst.path(), &layers).unwrap();
        assert!(bytes > 0);

        // Verify blob exists in destination
        let missing = verify_base_blobs(dst.path(), &layers);
        assert!(missing.is_empty());
    }

    #[test]
    fn copy_base_blobs_skip_existing() {
        let src = tempfile::tempdir().unwrap();
        create_test_layout(src.path());
        let layers = extract_base_layers(src.path()).unwrap();

        let dst = tempfile::tempdir().unwrap();
        let bytes1 = copy_base_blobs(src.path(), dst.path(), &layers).unwrap();
        let bytes2 = copy_base_blobs(src.path(), dst.path(), &layers).unwrap();
        assert!(bytes1 > 0);
        assert_eq!(bytes2, 0); // already exists
    }

    #[test]
    fn format_base_info_output() {
        let layers = BaseImageLayers {
            layers: vec![
                OciDescriptor::gzip_layer("sha256:a".into(), 5_000_000),
                OciDescriptor::gzip_layer("sha256:b".into(), 3_000_000),
            ],
            diff_ids: vec!["sha256:da".into(), "sha256:db".into()],
            history: vec![],
            architecture: "arm64".into(),
            os: "linux".into(),
            config: Default::default(),
        };
        let info = format_base_info("ubuntu:22.04", &layers);
        assert!(info.contains("ubuntu:22.04"));
        assert!(info.contains("arm64/linux"));
        assert!(info.contains("2 layers"));
        assert!(info.contains("MB"));
    }

    #[test]
    fn extract_base_layers_empty_index() {
        let dir = tempfile::tempdir().unwrap();
        let index = OciIndex { schema_version: 2, manifests: vec![], annotations: std::collections::HashMap::new() };
        std::fs::write(dir.path().join("index.json"), serde_json::to_vec(&index).unwrap()).unwrap();
        let result = extract_base_layers(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no manifests"));
    }
}
