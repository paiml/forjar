//! Demonstrates FJ-2101 container build types: OCI layers, dual digest, build plans.

use forjar::core::types::{
    BaseImageRef, DualDigest, ImageBuildPlan, LayerCacheEntry, LayerStrategy, OciBuildResult,
    OciCompression, OciLayerConfig, WhiteoutEntry,
};

fn main() {
    // OCI layer configuration
    println!("=== OCI Layer Config ===");
    let config = OciLayerConfig::default();
    println!("  Compression:   {}", config.compression);
    println!("  Deterministic: {}", config.deterministic);
    println!("  Epoch mtime:   {}", config.epoch_mtime);

    // Dual digest (BLAKE3 + SHA-256)
    println!("\n=== Dual Digest ===");
    let digest = DualDigest {
        blake3: "a1b2c3d4e5f60718".into(),
        sha256: "deadbeef01234567890abcdef0123456".into(),
        size_bytes: 2_097_152,
    };
    println!("  OCI:    {}", digest.oci_digest());
    println!("  Forjar: {}", digest.forjar_digest());
    println!("  Display: {digest}");

    // Layer cache entry
    println!("\n=== Layer Cache ===");
    let entry = LayerCacheEntry {
        content_hash: "blake3:abc123".into(),
        oci_digest: "sha256:def456".into(),
        compressed_size: 1_048_576,
        uncompressed_size: 4_194_304,
        compression: OciCompression::Gzip,
        store_path: "store/abc123/layer.tar.gz".into(),
    };
    println!(
        "  {} -> {} ({} -> {} bytes)",
        entry.content_hash, entry.store_path, entry.uncompressed_size, entry.compressed_size,
    );

    // Image build plan
    println!("\n=== Image Build Plan ===");
    let plan = ImageBuildPlan {
        tag: "myapp:v1.2.3".into(),
        base_image: Some("ubuntu:22.04".into()),
        layers: vec![
            LayerStrategy::Packages {
                names: vec!["nginx".into(), "curl".into(), "jq".into()],
            },
            LayerStrategy::Files {
                paths: vec![
                    "/etc/nginx/nginx.conf".into(),
                    "/etc/nginx/sites-enabled/default".into(),
                ],
            },
            LayerStrategy::Build {
                command: "cargo build --release".into(),
                workdir: Some("/src".into()),
            },
        ],
        labels: vec![
            ("maintainer".into(), "team@example.com".into()),
            ("version".into(), "1.2.3".into()),
        ],
        entrypoint: Some(vec!["/usr/sbin/nginx".into(), "-g".into(), "daemon off;".into()]),
    };
    println!("  Tag:    {}", plan.tag);
    println!("  Base:   {:?}", plan.base_image);
    println!("  Layers: {}", plan.layer_count());
    println!("  Scratch: {}", plan.is_scratch());

    // Base image reference
    println!("\n=== Base Image Resolution ===");
    for ref_str in ["ubuntu:22.04", "ghcr.io/org/app:v1", "localhost:5000/myimage"] {
        let img = BaseImageRef::new(ref_str);
        println!("  {} -> registry: {}", ref_str, img.registry());
    }

    // Whiteout entries (overlay-to-OCI conversion)
    println!("\n=== Whiteout Conversion ===");
    let entries = vec![
        WhiteoutEntry::FileDelete {
            path: "etc/nginx/old.conf".into(),
        },
        WhiteoutEntry::OpaqueDir {
            path: "var/cache/apt".into(),
        },
    ];
    for w in &entries {
        println!("  {:?} -> {}", w, w.oci_path());
    }

    // Build result
    println!("\n=== Build Result ===");
    let result = OciBuildResult {
        tag: "myapp:v1.2.3".into(),
        manifest_digest: "sha256:abc123def456".into(),
        layer_count: 3,
        total_size: 75 * 1024 * 1024,
        duration_secs: 23.4,
        layout_path: "out/oci-layout".into(),
    };
    println!("  {result}");
    println!("  Size: {:.1} MB", result.size_mb());
}
