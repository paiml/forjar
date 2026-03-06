//! Demonstrates FJ-2102 runtime OCI layer building: tar creation, dual digest, OCI layout.
//!
//! This example creates actual OCI-compliant layer tarballs from in-memory
//! resource definitions — the core of the container build pipeline.

use forjar::core::store::layer_builder::{
    build_layer, compute_dual_digest, write_oci_layout, LayerEntry,
};
use forjar::core::types::{
    OciCompression, OciImageConfig, OciLayerConfig, OciManifest, TarSortOrder,
};

fn main() {
    println!("=== FJ-2102: Runtime OCI Layer Builder ===\n");

    // 1. Build a file layer (Path 1: Direct Assembly)
    println!("--- Layer 1: Config files ---");
    let config_entries = vec![
        LayerEntry::dir("etc/", 0o755),
        LayerEntry::dir("etc/app/", 0o755),
        LayerEntry::file("etc/app/config.yaml", b"port: 8080\nlog_level: info\n", 0o644),
        LayerEntry::file("etc/app/secrets.env", b"DB_URL=postgres://localhost/app\n", 0o600),
    ];
    let layer_config = OciLayerConfig::default();
    let (result1, data1) = build_layer(&config_entries, &layer_config).unwrap();
    println!("  Files:       {}", result1.file_count);
    println!("  Uncompressed: {} bytes", result1.uncompressed_size);
    println!("  Compressed:   {} bytes ({:.0}%)", result1.compressed_size, result1.compression_ratio());
    println!("  DiffID:       {}", result1.diff_id);
    println!("  Digest:       {}", result1.digest);
    println!("  Store hash:   {}", result1.store_hash);

    // 2. Build an application binary layer
    println!("\n--- Layer 2: Application binary ---");
    let app_binary = vec![0xCFu8; 50_000]; // simulate a 50KB binary
    let app_entries = vec![
        LayerEntry::dir("usr/", 0o755),
        LayerEntry::dir("usr/local/", 0o755),
        LayerEntry::dir("usr/local/bin/", 0o755),
        LayerEntry::file("usr/local/bin/myapp", &app_binary, 0o755),
    ];
    let (result2, data2) = build_layer(&app_entries, &layer_config).unwrap();
    println!("  Files:       {}", result2.file_count);
    println!("  Uncompressed: {} bytes", result2.uncompressed_size);
    println!("  Compressed:   {} bytes ({:.0}%)", result2.compressed_size, result2.compression_ratio());
    println!("  DiffID:       {}", result2.diff_id);

    // 3. Determinism verification
    println!("\n--- Determinism Check ---");
    let (verify, _) = build_layer(&config_entries, &layer_config).unwrap();
    assert_eq!(result1.digest, verify.digest);
    assert_eq!(result1.diff_id, verify.diff_id);
    assert_eq!(result1.store_hash, verify.store_hash);
    println!("  PASSED: Same inputs produce identical digests");

    // 4. Order independence
    println!("\n--- Order Independence Check ---");
    let reversed = vec![
        LayerEntry::file("etc/app/secrets.env", b"DB_URL=postgres://localhost/app\n", 0o600),
        LayerEntry::file("etc/app/config.yaml", b"port: 8080\nlog_level: info\n", 0o644),
        LayerEntry::dir("etc/app/", 0o755),
        LayerEntry::dir("etc/", 0o755),
    ];
    let (reordered, _) = build_layer(&reversed, &layer_config).unwrap();
    assert_eq!(result1.digest, reordered.digest);
    println!("  PASSED: Lexicographic sort normalizes entry order");

    // 5. Dual digest computation
    println!("\n--- Dual Digest ---");
    let dual = compute_dual_digest(b"hello world");
    println!("  BLAKE3:  {}", dual.blake3);
    println!("  SHA-256: {}", dual.sha256);
    println!("  OCI:     {}", dual.oci_digest());
    println!("  Forjar:  {}", dual.forjar_digest());

    // 6. Compression comparison
    println!("\n--- Compression Comparison ---");
    let big_entries = vec![LayerEntry::file("data/big.txt", &vec![b'A'; 100_000], 0o644)];
    for comp in [OciCompression::None, OciCompression::Gzip, OciCompression::Zstd] {
        let cfg = OciLayerConfig {
            compression: comp,
            deterministic: true,
            epoch_mtime: 1,
            sort_order: TarSortOrder::Lexicographic,
        };
        let (r, _) = build_layer(&big_entries, &cfg).unwrap();
        println!("  {comp:5}: {} -> {} bytes ({:.0}%)", r.uncompressed_size, r.compressed_size, r.compression_ratio());
    }

    // 7. OCI image assembly
    println!("\n--- OCI Image Assembly ---");
    let layers = vec![
        (result1.clone(), data1),
        (result2.clone(), data2),
    ];
    let diff_ids = vec![result1.diff_id.clone(), result2.diff_id.clone()];
    let image_config = OciImageConfig::linux_amd64(diff_ids);
    let config_json = serde_json::to_vec_pretty(&image_config).unwrap();
    let config_digest = compute_dual_digest(&config_json);

    let manifest = OciManifest::new(
        config_digest.oci_digest(),
        vec![result1.to_descriptor(), result2.to_descriptor()],
    );
    println!("  Manifest layers: {}", manifest.layers.len());
    println!("  Total size:      {} bytes", manifest.total_layer_size());
    println!("  Config digest:   {}", config_digest.oci_digest());

    // Write to temp dir
    let dir = tempfile::tempdir().unwrap();
    write_oci_layout(dir.path(), &layers, &config_json).unwrap();
    println!("  OCI layout:      {}", dir.path().display());
    println!("  oci-layout:      {}", dir.path().join("oci-layout").exists());
    println!("  blobs/sha256/:   {}", dir.path().join("blobs/sha256").is_dir());

    println!("\n=== Done: 2 layers, 1 OCI image, fully content-addressed ===");
}
