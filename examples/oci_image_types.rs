//! FJ-2101: OCI image types — manifest, config, layers, and build results.
//!
//! ```bash
//! cargo run --example oci_image_types
//! ```

use forjar::core::types::{
    DeterminismLevel, ImageBuildConfig, LayerBuildPath, LayerBuildResult, LayerCompression,
    OciDescriptor, OciImageConfig, OciIndex, OciManifest,
};

fn main() {
    // Build layers from forjar resources
    let system_layer = LayerBuildResult {
        digest: "sha256:aabbcc1111".into(),
        diff_id: "sha256:ddddee1111".into(),
        store_hash: "blake3:ff001122".into(),
        compressed_size: 45_000_000,
        uncompressed_size: 120_000_000,
        compression: LayerCompression::Gzip,
        file_count: 1247,
        build_path: LayerBuildPath::DirectAssembly,
    };

    let config_layer = LayerBuildResult {
        digest: "sha256:aabbcc2222".into(),
        diff_id: "sha256:ddddee2222".into(),
        store_hash: "blake3:ff003344".into(),
        compressed_size: 2_048,
        uncompressed_size: 8_192,
        compression: LayerCompression::Gzip,
        file_count: 3,
        build_path: LayerBuildPath::DirectAssembly,
    };

    let app_layer = LayerBuildResult {
        digest: "sha256:aabbcc3333".into(),
        diff_id: "sha256:ddddee3333".into(),
        store_hash: "blake3:ff005566".into(),
        compressed_size: 5_000_000,
        uncompressed_size: 12_000_000,
        compression: LayerCompression::Gzip,
        file_count: 1,
        build_path: LayerBuildPath::PepitaExport,
    };

    println!("=== Layer Build Results ===");
    for (name, layer) in [
        ("system", &system_layer),
        ("config", &config_layer),
        ("app", &app_layer),
    ] {
        println!(
            "  {name}: {} files, {:.1} MB compressed ({:.0}% ratio), path: {:?}",
            layer.file_count,
            layer.compressed_size as f64 / (1024.0 * 1024.0),
            layer.compression_ratio(),
            layer.build_path,
        );
    }
    println!();

    // Assemble OCI manifest
    let layer_descriptors: Vec<OciDescriptor> = [&system_layer, &config_layer, &app_layer]
        .iter()
        .map(|l| l.to_descriptor())
        .collect();

    let manifest = OciManifest::new("sha256:config_digest_here".into(), layer_descriptors);
    println!("=== OCI Manifest ===");
    println!("  Schema version: {}", manifest.schema_version);
    println!("  Layers: {}", manifest.layers.len());
    println!(
        "  Total layer size: {:.1} MB",
        manifest.total_layer_size() as f64 / (1024.0 * 1024.0)
    );
    println!();

    // Build image config
    let diff_ids = vec![
        system_layer.diff_id.clone(),
        config_layer.diff_id.clone(),
        app_layer.diff_id.clone(),
    ];
    let config = OciImageConfig::linux_amd64(diff_ids);
    println!("=== OCI Image Config ===");
    println!("  Architecture: {}", config.architecture);
    println!("  OS: {}", config.os);
    println!("  Layers: {}", config.layer_count());
    println!();

    // Create OCI index
    let manifest_desc = OciDescriptor {
        media_type: "application/vnd.oci.image.manifest.v1+json".into(),
        digest: "sha256:manifest_digest_here".into(),
        size: 512,
        annotations: Default::default(),
    };
    let index = OciIndex::single(manifest_desc);
    println!("=== OCI Index ===");
    println!("  Manifests: {}", index.manifests.len());
    println!();

    // Image build config from YAML
    let build_cfg = ImageBuildConfig {
        name: "myregistry.io/myapp".into(),
        tag: "1.0.0".into(),
        base: Some("ubuntu:22.04".into()),
        deterministic: DeterminismLevel::Strict,
        cache: true,
        max_layers: 10,
        compress: LayerCompression::Gzip,
    };
    println!("=== Image Build Config ===");
    println!("  Name: {}:{}", build_cfg.name, build_cfg.tag);
    println!("  Base: {:?}", build_cfg.base);
    println!("  Deterministic: {:?}", build_cfg.deterministic);
    println!("  Max layers: {}", build_cfg.max_layers);

    // JSON serialization
    println!();
    println!("=== Manifest JSON ===");
    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
}
