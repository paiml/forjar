//! Tests for OCI image types (FJ-2101).

use super::oci_types::*;

#[test]
fn oci_manifest_new() {
    let manifest = OciManifest::new("sha256:abc".into(), vec![]);
    assert_eq!(manifest.schema_version, 2);
    assert_eq!(manifest.config.digest, "sha256:abc");
    assert!(manifest.layers.is_empty());
    assert_eq!(manifest.total_layer_size(), 0);
}

#[test]
fn oci_manifest_total_size() {
    let layers = vec![
        OciDescriptor::gzip_layer("sha256:a".into(), 1000),
        OciDescriptor::gzip_layer("sha256:b".into(), 2000),
    ];
    let manifest = OciManifest::new("sha256:cfg".into(), layers);
    assert_eq!(manifest.total_layer_size(), 3000);
}

#[test]
fn oci_descriptor_gzip_media_type() {
    let d = OciDescriptor::gzip_layer("sha256:x".into(), 500);
    assert!(d.media_type.contains("gzip"));
}

#[test]
fn oci_descriptor_zstd_media_type() {
    let d = OciDescriptor::zstd_layer("sha256:x".into(), 500);
    assert!(d.media_type.contains("zstd"));
}

#[test]
fn oci_image_config_linux_amd64() {
    let cfg = OciImageConfig::linux_amd64(vec![
        "sha256:diff1".into(),
        "sha256:diff2".into(),
    ]);
    assert_eq!(cfg.architecture, "amd64");
    assert_eq!(cfg.os, "linux");
    assert_eq!(cfg.layer_count(), 2);
    assert_eq!(cfg.rootfs.rootfs_type, "layers");
}

#[test]
fn oci_runtime_config_default() {
    let rc = OciRuntimeConfig::default();
    assert!(rc.entrypoint.is_empty());
    assert!(rc.cmd.is_empty());
    assert!(rc.env.is_empty());
    assert!(rc.working_dir.is_none());
}

#[test]
fn oci_index_single() {
    let desc = OciDescriptor::gzip_layer("sha256:m".into(), 100);
    let idx = OciIndex::single(desc);
    assert_eq!(idx.schema_version, 2);
    assert_eq!(idx.manifests.len(), 1);
}

#[test]
fn layer_build_result_compression_ratio() {
    let result = LayerBuildResult {
        digest: "sha256:a".into(),
        diff_id: "sha256:b".into(),
        store_hash: "blake3:c".into(),
        compressed_size: 500,
        uncompressed_size: 1000,
        compression: LayerCompression::Gzip,
        file_count: 10,
        build_path: LayerBuildPath::DirectAssembly,
    };
    assert!((result.compression_ratio() - 50.0).abs() < 0.1);
}

#[test]
fn layer_build_result_zero_uncompressed() {
    let result = LayerBuildResult {
        digest: "sha256:a".into(),
        diff_id: "sha256:b".into(),
        store_hash: "blake3:c".into(),
        compressed_size: 0,
        uncompressed_size: 0,
        compression: LayerCompression::None,
        file_count: 0,
        build_path: LayerBuildPath::DirectAssembly,
    };
    assert!((result.compression_ratio() - 100.0).abs() < 0.1);
}

#[test]
fn layer_build_result_to_descriptor() {
    let result = LayerBuildResult {
        digest: "sha256:abc".into(),
        diff_id: "sha256:def".into(),
        store_hash: "blake3:xyz".into(),
        compressed_size: 2048,
        uncompressed_size: 4096,
        compression: LayerCompression::Zstd,
        file_count: 5,
        build_path: LayerBuildPath::PepitaExport,
    };
    let desc = result.to_descriptor();
    assert!(desc.media_type.contains("zstd"));
    assert_eq!(desc.digest, "sha256:abc");
    assert_eq!(desc.size, 2048);
}

#[test]
fn manifest_serde_roundtrip() {
    let layers = vec![OciDescriptor::gzip_layer("sha256:l1".into(), 100)];
    let manifest = OciManifest::new("sha256:cfg".into(), layers);
    let json = serde_json::to_string(&manifest).unwrap();
    let parsed: OciManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.schema_version, 2);
    assert_eq!(parsed.layers.len(), 1);
    assert_eq!(parsed.config.digest, "sha256:cfg");
}

#[test]
fn image_config_serde_roundtrip() {
    let cfg = OciImageConfig::linux_amd64(vec!["sha256:d1".into()]);
    let json = serde_json::to_string(&cfg).unwrap();
    let parsed: OciImageConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.architecture, "amd64");
    assert_eq!(parsed.layer_count(), 1);
}

#[test]
fn determinism_level_default() {
    assert_eq!(DeterminismLevel::default(), DeterminismLevel::False);
}

#[test]
fn layer_compression_default() {
    assert_eq!(LayerCompression::default(), LayerCompression::Gzip);
}

#[test]
fn image_build_config_defaults() {
    let yaml = r#"
name: test/app
tag: latest
"#;
    let cfg: ImageBuildConfig = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(cfg.name, "test/app");
    assert_eq!(cfg.tag, "latest");
    assert!(cfg.base.is_none());
    assert!(cfg.cache);
    assert_eq!(cfg.max_layers, 10);
    assert_eq!(cfg.compress, LayerCompression::Gzip);
    assert_eq!(cfg.deterministic, DeterminismLevel::False);
}

// FJ-2200 G4: Verify media type contracts fire correctly
#[test]
fn manifest_media_type_contract() {
    let manifest = OciManifest::new("sha256:test".into(), vec![]);
    assert_eq!(manifest.media_type, "application/vnd.oci.image.manifest.v1+json");
    assert_eq!(manifest.config.media_type, "application/vnd.oci.image.config.v1+json");
}
