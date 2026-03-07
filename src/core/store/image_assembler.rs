//! FJ-2104: OCI image assembler — builds complete images from resource definitions.
//!
//! Connects the layer builder (FJ-2102) with OCI types (FJ-2101) to produce
//! loadable OCI images from `type: image` resource definitions.

use crate::core::store::layer_builder::{
    build_layer, compute_dual_digest, write_oci_layout, LayerEntry,
};
use crate::core::types::{
    ImageBuildPlan, LayerBuildResult, LayerStrategy, OciDescriptor, OciHistoryEntry,
    OciImageConfig, OciIndex, OciLayerConfig, OciManifest,
};
use std::collections::HashMap;
use std::path::Path;

/// Result of assembling a complete OCI image.
#[derive(Debug)]
pub struct AssembledImage {
    /// Path to the OCI layout directory.
    pub layout_dir: std::path::PathBuf,
    /// Image manifest.
    pub manifest: OciManifest,
    /// Image config.
    pub config: OciImageConfig,
    /// Per-layer build results.
    pub layers: Vec<LayerBuildResult>,
    /// Total compressed size.
    pub total_size: u64,
}

/// Assemble a complete OCI image from a build plan.
///
/// Takes an `ImageBuildPlan` and produces an OCI layout directory
/// containing blobs, manifest, config, and index. The resulting
/// directory can be loaded with `docker load` or pushed to a registry.
pub fn assemble_image(
    plan: &ImageBuildPlan,
    layer_entries: &[Vec<LayerEntry>],
    output_dir: &Path,
    layer_config: &OciLayerConfig,
    target_arch: Option<&str>,
) -> Result<AssembledImage, String> {
    if plan.layers.len() != layer_entries.len() {
        return Err(format!(
            "layer count mismatch: plan has {} layers but {} entry sets provided",
            plan.layers.len(),
            layer_entries.len(),
        ));
    }

    // Build each layer
    let mut built_layers: Vec<(LayerBuildResult, Vec<u8>)> = Vec::new();
    let mut history: Vec<OciHistoryEntry> = Vec::new();

    for (i, (strategy, entries)) in plan.layers.iter().zip(layer_entries.iter()).enumerate() {
        let (result, data) = build_layer(entries, layer_config)
            .map_err(|e| format!("layer {i} build failed: {e}"))?;

        history.push(OciHistoryEntry {
            created: None,
            created_by: Some(strategy_description(strategy)),
            empty_layer: false,
            comment: None,
        });

        built_layers.push((result, data));
    }

    // Collect diff_ids and descriptors
    let diff_ids: Vec<String> = built_layers
        .iter()
        .map(|(r, _)| r.diff_id.clone())
        .collect();
    let layer_descriptors: Vec<OciDescriptor> = built_layers
        .iter()
        .map(|(r, _)| r.to_descriptor())
        .collect();
    let layer_results: Vec<LayerBuildResult> =
        built_layers.iter().map(|(r, _)| r.clone()).collect();
    let total_size: u64 = built_layers.iter().map(|(r, _)| r.compressed_size).sum();

    // Build image config (E12: support target architecture)
    let arch = target_arch.unwrap_or("amd64");
    let mut config = OciImageConfig::for_arch(arch, "linux", diff_ids);
    config.history = history;
    if let Some(ref ep) = plan.entrypoint {
        config.config.entrypoint = ep.clone();
    }
    for (k, v) in &plan.labels {
        config.config.labels.insert(k.clone(), v.clone());
    }

    // Serialize config
    let config_json =
        serde_json::to_vec_pretty(&config).map_err(|e| format!("serialize config: {e}"))?;
    let config_digest = compute_dual_digest(&config_json);

    // Build manifest
    let manifest = OciManifest::new(config_digest.oci_digest(), layer_descriptors);

    // Serialize manifest
    let manifest_json =
        serde_json::to_vec_pretty(&manifest).map_err(|e| format!("serialize manifest: {e}"))?;
    let manifest_digest = compute_dual_digest(&manifest_json);

    // Write OCI layout
    write_oci_layout(output_dir, &built_layers, &config_json)?;

    // Write manifest blob
    let manifest_hex = manifest_digest.sha256.clone();
    std::fs::write(
        output_dir.join(format!("blobs/sha256/{manifest_hex}")),
        &manifest_json,
    )
    .map_err(|e| format!("write manifest blob: {e}"))?;

    // Write index.json
    let index = OciIndex::single(OciDescriptor {
        media_type: "application/vnd.oci.image.manifest.v1+json".into(),
        digest: format!("sha256:{manifest_hex}"),
        size: manifest_json.len() as u64,
        annotations: HashMap::new(),
    });
    let index_json =
        serde_json::to_vec_pretty(&index).map_err(|e| format!("serialize index: {e}"))?;
    std::fs::write(output_dir.join("index.json"), &index_json)
        .map_err(|e| format!("write index.json: {e}"))?;

    // Write Docker-compat manifest.json (for docker load)
    let tag = &plan.tag;
    let docker_layers: Vec<String> = layer_results
        .iter()
        .map(|r| {
            let hex = r.digest.strip_prefix("sha256:").unwrap_or(&r.digest);
            format!("blobs/sha256/{hex}")
        })
        .collect();
    let docker_manifest = serde_json::json!([{
        "RepoTags": [tag],
        "Config": format!("blobs/sha256/{}", config_digest.sha256),
        "Layers": docker_layers,
    }]);
    std::fs::write(
        output_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&docker_manifest)
            .map_err(|e| format!("serialize docker manifest: {e}"))?,
    )
    .map_err(|e| format!("write manifest.json: {e}"))?;

    // FJ-2200: Postcondition — valid OCI layout
    debug_assert!(
        output_dir.join("oci-layout").exists(),
        "assemble_image: oci-layout missing"
    );
    debug_assert!(
        output_dir.join("index.json").exists(),
        "assemble_image: index.json missing"
    );
    debug_assert!(
        manifest.layers.len() == layer_results.len(),
        "assemble_image: manifest layer count mismatch"
    );

    Ok(AssembledImage {
        layout_dir: output_dir.to_path_buf(),
        manifest,
        config,
        layers: layer_results,
        total_size,
    })
}

fn strategy_description(strategy: &LayerStrategy) -> String {
    match strategy {
        LayerStrategy::Packages { names } => format!("forjar: packages {}", names.join(", ")),
        LayerStrategy::Files { paths } => format!("forjar: files {}", paths.join(", ")),
        LayerStrategy::Build { command, .. } => format!("forjar: build {command}"),
        LayerStrategy::Derivation { store_path } => format!("forjar: derivation {store_path}"),
    }
}
