//! Demonstrates FJ-2104: End-to-end OCI image assembly from a build plan.
//!
//! Creates a complete, loadable OCI image with two layers, entrypoint,
//! labels, and Docker-compat manifest — all from declarative definitions.

use forjar::core::store::image_assembler::assemble_image;
use forjar::core::store::layer_builder::LayerEntry;
use forjar::core::types::{ImageBuildPlan, LayerStrategy, OciLayerConfig};

fn main() {
    println!("=== FJ-2104: OCI Image Assembler ===\n");

    // Define the build plan (what `type: image` in YAML compiles to)
    let plan = ImageBuildPlan {
        tag: "myapp:v1.0.0".into(),
        base_image: Some("ubuntu:22.04".into()),
        layers: vec![
            LayerStrategy::Files {
                paths: vec!["/etc/app/config.yaml".into(), "/etc/app/secrets.env".into()],
            },
            LayerStrategy::Files {
                paths: vec!["/usr/local/bin/myapp".into()],
            },
        ],
        labels: vec![
            ("org.opencontainers.image.source".into(), "https://github.com/example/myapp".into()),
            ("org.opencontainers.image.version".into(), "1.0.0".into()),
        ],
        entrypoint: Some(vec!["/usr/local/bin/myapp".into(), "--config".into(), "/etc/app/config.yaml".into()]),
    };

    // Prepare file entries for each layer
    let layer_entries = vec![
        // Layer 0: config files
        vec![
            LayerEntry::dir("etc/", 0o755),
            LayerEntry::dir("etc/app/", 0o755),
            LayerEntry::file("etc/app/config.yaml", b"port: 8080\nlog_level: info\nworkers: 4\n", 0o644),
            LayerEntry::file("etc/app/secrets.env", b"DB_URL=postgres://db:5432/app\nAPI_KEY=change-me\n", 0o600),
        ],
        // Layer 1: application binary
        vec![
            LayerEntry::dir("usr/", 0o755),
            LayerEntry::dir("usr/local/", 0o755),
            LayerEntry::dir("usr/local/bin/", 0o755),
            LayerEntry::file("usr/local/bin/myapp", &vec![0xCFu8; 25_000], 0o755),
        ],
    ];

    // Assemble the image
    let dir = tempfile::tempdir().unwrap();
    let config = OciLayerConfig::default();

    println!("Building image: {}", plan.tag);
    println!("  Layers: {}", plan.layers.len());
    println!("  Output: {}\n", dir.path().display());

    let result = assemble_image(&plan, &layer_entries, dir.path(), &config).unwrap();

    // Report
    println!("--- Build Report ---");
    for (i, layer) in result.layers.iter().enumerate() {
        println!("  Layer {i}: {} files, {} -> {} bytes ({:.0}% compressed)",
            layer.file_count, layer.uncompressed_size, layer.compressed_size, layer.compression_ratio());
        println!("    DiffID: {}", layer.diff_id);
        println!("    Digest: {}", layer.digest);
    }
    println!("\n  Total size:  {} bytes", result.total_size);
    println!("  Manifest layers: {}", result.manifest.layers.len());
    println!("  Config layers:   {}", result.config.layer_count());
    println!("  Entrypoint:  {:?}", result.config.config.entrypoint);
    println!("  Labels:      {:?}", result.config.config.labels);

    // Verify OCI layout
    println!("\n--- OCI Layout ---");
    for name in ["oci-layout", "index.json", "manifest.json"] {
        let path = dir.path().join(name);
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        println!("  {name}: {size} bytes");
    }

    let blobs: Vec<_> = std::fs::read_dir(dir.path().join("blobs/sha256"))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    println!("  blobs/sha256/: {} blobs", blobs.len());

    // Verify Docker compat
    let docker: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("manifest.json")).unwrap(),
    ).unwrap();
    println!("\n--- Docker Compat ---");
    println!("  RepoTags: {}", docker[0]["RepoTags"][0]);
    println!("  Layers:   {}", docker[0]["Layers"].as_array().unwrap().len());

    println!("\n=== Image ready: {} ===", plan.tag);
    println!("  To load: tar -cf - -C {} . | docker load", dir.path().display());
}
