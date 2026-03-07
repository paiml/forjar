//! Tests for FJ-2102: Runtime OCI layer builder.

use super::layer_builder::*;
use crate::core::types::{OciCompression, OciLayerConfig, TarSortOrder};

fn default_config() -> OciLayerConfig {
    OciLayerConfig {
        compression: OciCompression::Gzip,
        deterministic: true,
        epoch_mtime: 1,
        sort_order: TarSortOrder::Lexicographic,
    }
}

#[test]
fn build_empty_layer() {
    let (result, data) = build_layer(&[], &default_config()).unwrap();
    assert_eq!(result.file_count, 0);
    assert!(result.compressed_size > 0); // gzip header even for empty tar
    assert!(result.digest.starts_with("sha256:"));
    assert!(result.diff_id.starts_with("sha256:"));
    assert!(result.store_hash.starts_with("blake3:"));
    assert!(!data.is_empty());
}

#[test]
fn build_single_file_layer() {
    let entries = vec![LayerEntry::file("etc/app.conf", b"key=value\n", 0o644)];
    let config = default_config();
    let (result, data) = build_layer(&entries, &config).unwrap();
    assert_eq!(result.file_count, 1);
    assert!(result.uncompressed_size > 10);
    assert!(result.compressed_size > 0);
    assert!(result.compressed_size <= result.uncompressed_size + 100); // gzip overhead
    assert!(!data.is_empty());
}

#[test]
fn build_layer_determinism() {
    let entries = vec![
        LayerEntry::file("etc/a.conf", b"aaa", 0o644),
        LayerEntry::file("etc/b.conf", b"bbb", 0o644),
    ];
    let config = default_config();

    let (r1, d1) = build_layer(&entries, &config).unwrap();
    let (r2, d2) = build_layer(&entries, &config).unwrap();

    assert_eq!(
        r1.digest, r2.digest,
        "compressed digest must be deterministic"
    );
    assert_eq!(r1.diff_id, r2.diff_id, "DiffID must be deterministic");
    assert_eq!(
        r1.store_hash, r2.store_hash,
        "BLAKE3 hash must be deterministic"
    );
    assert_eq!(d1, d2, "compressed bytes must be identical");
}

#[test]
fn build_layer_order_independence() {
    let a = LayerEntry::file("etc/z.conf", b"z", 0o644);
    let b = LayerEntry::file("etc/a.conf", b"a", 0o644);
    let config = default_config();

    let (r1, _) = build_layer(&[a.clone(), b.clone()], &config).unwrap();
    let (r2, _) = build_layer(&[b, a], &config).unwrap();

    assert_eq!(
        r1.digest, r2.digest,
        "lexicographic sort must produce same output regardless of input order"
    );
}

#[test]
fn build_layer_with_directories() {
    let entries = vec![
        LayerEntry::dir("etc/", 0o755),
        LayerEntry::dir("etc/app/", 0o755),
        LayerEntry::file("etc/app/config.yaml", b"port: 8080\n", 0o644),
    ];
    let (result, _) = build_layer(&entries, &default_config()).unwrap();
    assert_eq!(result.file_count, 3);
}

#[test]
fn build_layer_no_compression() {
    let entries = vec![LayerEntry::file("app/main", b"#!/bin/sh\necho hi", 0o755)];
    let config = OciLayerConfig {
        compression: OciCompression::None,
        ..default_config()
    };
    let (result, data) = build_layer(&entries, &config).unwrap();
    assert_eq!(result.compressed_size, result.uncompressed_size);
    assert_eq!(data.len() as u64, result.uncompressed_size);
}

#[test]
fn build_layer_zstd_compression() {
    let entries = vec![LayerEntry::file("data/big.txt", &[b'x'; 10000], 0o644)];
    let config = OciLayerConfig {
        compression: OciCompression::Zstd,
        ..default_config()
    };
    let (result, _) = build_layer(&entries, &config).unwrap();
    assert!(result.compressed_size < result.uncompressed_size);
    assert!(matches!(
        result.compression,
        crate::core::types::LayerCompression::Zstd
    ));
}

#[test]
fn build_layer_directory_first_sort() {
    let entries = vec![
        LayerEntry::file("etc/z.conf", b"z", 0o644),
        LayerEntry::dir("etc/", 0o755),
        LayerEntry::file("etc/a.conf", b"a", 0o644),
    ];
    let config = OciLayerConfig {
        sort_order: TarSortOrder::DirectoryFirst,
        ..default_config()
    };
    let (result, _) = build_layer(&entries, &config).unwrap();
    assert_eq!(result.file_count, 3);
}

#[test]
fn layer_entry_path_normalization() {
    let e = LayerEntry::file("/etc/app.conf", b"data", 0o644);
    assert_eq!(e.path, "etc/app.conf", "leading / must be stripped");

    let e2 = LayerEntry::dir("/var/log/", 0o755);
    assert_eq!(e2.path, "var/log/", "dir must end with /");
}

#[test]
fn compute_dual_digest_determinism() {
    let data = b"hello world";
    let d1 = compute_dual_digest(data);
    let d2 = compute_dual_digest(data);
    assert_eq!(d1.blake3, d2.blake3);
    assert_eq!(d1.sha256, d2.sha256);
    assert_eq!(d1.size_bytes, 11);
}

#[test]
fn compute_dual_digest_known_values() {
    let data = b"";
    let d = compute_dual_digest(data);
    // SHA-256 of empty string is well-known
    assert_eq!(
        d.sha256,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(d.size_bytes, 0);
}

#[test]
fn write_oci_layout_creates_structure() {
    let dir = tempfile::tempdir().unwrap();
    let entries = vec![LayerEntry::file("app/main", b"hello", 0o755)];
    let (result, data) = build_layer(&entries, &default_config()).unwrap();
    let config_json = b"{}";

    write_oci_layout(dir.path(), &[(result.clone(), data)], config_json).unwrap();

    assert!(dir.path().join("oci-layout").exists());
    assert!(dir.path().join("blobs/sha256").is_dir());

    // Layer blob exists
    let layer_hex = result.digest.strip_prefix("sha256:").unwrap();
    assert!(dir
        .path()
        .join(format!("blobs/sha256/{layer_hex}"))
        .exists());

    // Config blob exists
    let config_digest = compute_dual_digest(config_json);
    assert!(dir
        .path()
        .join(format!("blobs/sha256/{}", config_digest.sha256))
        .exists());
}

#[test]
fn build_layer_to_descriptor() {
    let entries = vec![LayerEntry::file("bin/app", b"binary", 0o755)];
    let (result, _) = build_layer(&entries, &default_config()).unwrap();
    let desc = result.to_descriptor();
    assert!(desc.media_type.contains("gzip"));
    assert_eq!(desc.digest, result.digest);
    assert_eq!(desc.size, result.compressed_size);
}

#[test]
fn build_large_layer_compresses() {
    // 100KB of repetitive data should compress well
    let big = vec![b'A'; 100_000];
    let entries = vec![LayerEntry::file("data/big.bin", &big, 0o644)];
    let (result, _) = build_layer(&entries, &default_config()).unwrap();
    assert!(
        result.compressed_size < result.uncompressed_size / 2,
        "repetitive data should compress to <50% (got {}%)",
        result.compression_ratio() as u32
    );
}
