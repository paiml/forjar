//! FJ-2101–2106: Popperian falsification for container/OCI image builds.
//!
//! Each test states conditions under which the container build system
//! would be rejected as invalid. If any assertion fails, the build
//! pipeline is measuring or producing the wrong thing.
#![allow(clippy::field_reassign_with_default)]

use forjar::core::store::layer_builder::{build_layer, compute_dual_digest, LayerEntry};
use forjar::core::types::{
    BaseImageRef, DualDigest, ImageBuildPlan, LayerBuildPath, LayerCompression, LayerStrategy,
    OciBuildResult, OciCompression, OciImageConfig, OciIndex, OciLayerConfig, OciManifest,
    TarSortOrder, WhiteoutEntry,
};

// ── FJ-2101: OCI Assembly ──────────────────────────────────────────

#[test]
fn f_2101_1_oci_manifest_schema_always_2() {
    let manifest = OciManifest::new("sha256:test".into(), vec![]);
    assert_eq!(manifest.schema_version, 2);
    assert_eq!(
        manifest.media_type,
        "application/vnd.oci.image.manifest.v1+json"
    );
}

#[test]
fn f_2101_2_oci_manifest_preserves_layer_count() {
    use forjar::core::types::OciDescriptor;
    let layers = vec![
        OciDescriptor::gzip_layer("sha256:aaa".into(), 100),
        OciDescriptor::gzip_layer("sha256:bbb".into(), 200),
        OciDescriptor::zstd_layer("sha256:ccc".into(), 150),
    ];
    let manifest = OciManifest::new("sha256:cfg".into(), layers);
    assert_eq!(manifest.layers.len(), 3);
    assert_eq!(manifest.total_layer_size(), 450);
}

#[test]
fn f_2101_3_oci_config_layer_count_matches_diff_ids() {
    let diff_ids = vec!["sha256:a".into(), "sha256:b".into()];
    let config = OciImageConfig::linux_amd64(diff_ids);
    assert_eq!(config.layer_count(), 2);
    assert_eq!(config.architecture, "amd64");
    assert_eq!(config.os, "linux");
    assert_eq!(config.rootfs.rootfs_type, "layers");
}

#[test]
fn f_2101_4_oci_index_single_manifest() {
    use forjar::core::types::OciDescriptor;
    let desc = OciDescriptor::gzip_layer("sha256:manifest".into(), 1024);
    let index = OciIndex::single(desc);
    assert_eq!(index.schema_version, 2);
    assert_eq!(index.manifests.len(), 1);
}

#[test]
fn f_2101_5_dual_digest_formats_correctly() {
    let dd = DualDigest {
        blake3: "abcdef0123456789".into(),
        sha256: "fedcba9876543210".into(),
        size_bytes: 512,
    };
    assert_eq!(dd.oci_digest(), "sha256:fedcba9876543210");
    assert_eq!(dd.forjar_digest(), "blake3:abcdef0123456789");
    let display = dd.to_string();
    assert!(display.contains("blake3:abcdef01"));
    assert!(display.contains("sha256:fedcba98"));
    assert!(display.contains("512B"));
}

// ── FJ-2102: Direct Layer Assembly ─────────────────────────────────

#[test]
fn f_2102_1_deterministic_layer_same_input_same_digest() {
    let entries = vec![
        LayerEntry::dir("etc/", 0o755),
        LayerEntry::file("etc/config.yaml", b"key: value\n", 0o644),
    ];
    let config = OciLayerConfig::default();
    let (r1, d1) = build_layer(&entries, &config).unwrap();
    let (r2, d2) = build_layer(&entries, &config).unwrap();

    // Falsifier: same inputs MUST produce identical digests
    assert_eq!(r1.digest, r2.digest, "digest must be deterministic");
    assert_eq!(r1.diff_id, r2.diff_id, "diff_id must be deterministic");
    assert_eq!(
        r1.store_hash, r2.store_hash,
        "store_hash must be deterministic"
    );
    assert_eq!(d1, d2, "compressed bytes must be identical");
}

#[test]
fn f_2102_2_different_content_different_digest() {
    let config = OciLayerConfig::default();
    let e1 = vec![LayerEntry::file("a.txt", b"hello", 0o644)];
    let e2 = vec![LayerEntry::file("a.txt", b"world", 0o644)];
    let (r1, _) = build_layer(&e1, &config).unwrap();
    let (r2, _) = build_layer(&e2, &config).unwrap();

    // Falsifier: different content MUST produce different digests
    assert_ne!(r1.digest, r2.digest);
    assert_ne!(r1.diff_id, r2.diff_id);
    assert_ne!(r1.store_hash, r2.store_hash);
}

#[test]
fn f_2102_3_layer_build_path_is_direct_assembly() {
    let entries = vec![LayerEntry::file("test", b"data", 0o644)];
    let config = OciLayerConfig::default();
    let (result, _) = build_layer(&entries, &config).unwrap();
    assert_eq!(result.build_path, LayerBuildPath::DirectAssembly);
}

#[test]
fn f_2102_4_empty_layer_produces_valid_result() {
    let entries: Vec<LayerEntry> = vec![];
    let config = OciLayerConfig::default();
    let (result, data) = build_layer(&entries, &config).unwrap();
    assert_eq!(result.file_count, 0);
    assert!(!data.is_empty(), "even empty tar has headers");
    assert!(result.digest.starts_with("sha256:"));
}

#[test]
fn f_2102_5_gzip_compression_smaller_than_uncompressed() {
    // Large enough content that compression helps
    let big_content = "x".repeat(10_000);
    let entries = vec![LayerEntry::file("big.txt", big_content.as_bytes(), 0o644)];
    let config = OciLayerConfig {
        compression: OciCompression::Gzip,
        ..Default::default()
    };
    let (result, _) = build_layer(&entries, &config).unwrap();
    assert_eq!(result.compression, LayerCompression::Gzip);
    assert!(
        result.compressed_size < result.uncompressed_size,
        "gzip must compress large content"
    );
}

#[test]
fn f_2102_6_zstd_compression_produces_valid_layer() {
    let entries = vec![LayerEntry::file("data.bin", &[42u8; 5000], 0o644)];
    let config = OciLayerConfig {
        compression: OciCompression::Zstd,
        ..Default::default()
    };
    let (result, _) = build_layer(&entries, &config).unwrap();
    assert_eq!(result.compression, LayerCompression::Zstd);
    assert!(result.compressed_size < result.uncompressed_size);
}

#[test]
fn f_2102_7_no_compression_preserves_size() {
    let entries = vec![LayerEntry::file("data", b"content", 0o644)];
    let config = OciLayerConfig {
        compression: OciCompression::None,
        ..Default::default()
    };
    let (result, _) = build_layer(&entries, &config).unwrap();
    assert_eq!(result.compression, LayerCompression::None);
    assert_eq!(result.compressed_size, result.uncompressed_size);
}

#[test]
fn f_2102_8_lexicographic_sort_order_verified() {
    let entries = vec![
        LayerEntry::file("z.txt", b"z", 0o644),
        LayerEntry::file("a.txt", b"a", 0o644),
        LayerEntry::file("m.txt", b"m", 0o644),
    ];
    let config_lex = OciLayerConfig {
        sort_order: TarSortOrder::Lexicographic,
        ..Default::default()
    };
    let config_dir = OciLayerConfig {
        sort_order: TarSortOrder::DirectoryFirst,
        ..Default::default()
    };
    let (r_lex, _) = build_layer(&entries, &config_lex).unwrap();
    let (r_dir, _) = build_layer(&entries, &config_dir).unwrap();
    // With no directories, both orderings should produce same output
    assert_eq!(r_lex.digest, r_dir.digest);
}

#[test]
fn f_2102_9_directory_first_differs_with_mixed_entries() {
    let entries = vec![
        LayerEntry::file("b.txt", b"b", 0o644),
        LayerEntry::dir("a_dir/", 0o755),
        LayerEntry::file("a.txt", b"a", 0o644),
    ];
    let config_lex = OciLayerConfig {
        sort_order: TarSortOrder::Lexicographic,
        ..Default::default()
    };
    let config_dir = OciLayerConfig {
        sort_order: TarSortOrder::DirectoryFirst,
        ..Default::default()
    };
    let (r_lex, _) = build_layer(&entries, &config_lex).unwrap();
    let (r_dir, _) = build_layer(&entries, &config_dir).unwrap();
    // Directory-first should produce different ordering
    assert_ne!(r_lex.digest, r_dir.digest);
}

// ── FJ-2101: Dual Digest ──────────────────────────────────────────

#[test]
fn f_2101_6_dual_digest_both_non_empty() {
    let content = b"arbitrary content for hashing";
    let dd = compute_dual_digest(content);
    assert!(!dd.blake3.is_empty());
    assert!(!dd.sha256.is_empty());
    assert_eq!(dd.size_bytes, content.len() as u64);
}

#[test]
fn f_2101_7_dual_digest_deterministic() {
    let content = b"repeatable hash test";
    let dd1 = compute_dual_digest(content);
    let dd2 = compute_dual_digest(content);
    assert_eq!(dd1.blake3, dd2.blake3);
    assert_eq!(dd1.sha256, dd2.sha256);
}

#[test]
fn f_2101_8_dual_digest_different_for_different_content() {
    let dd1 = compute_dual_digest(b"content A");
    let dd2 = compute_dual_digest(b"content B");
    assert_ne!(dd1.blake3, dd2.blake3);
    assert_ne!(dd1.sha256, dd2.sha256);
}

// ── FJ-2104: Image Build Plan ──────────────────────────────────────

#[test]
fn f_2104_1_scratch_image_has_no_base() {
    let plan = ImageBuildPlan {
        tag: "scratch:latest".into(),
        base_image: None,
        layers: vec![LayerStrategy::Files {
            paths: vec!["/bin/app".into()],
        }],
        labels: vec![],
        entrypoint: None,
    };
    assert!(plan.is_scratch());
    assert_eq!(plan.layer_count(), 1);
}

#[test]
fn f_2104_2_tier_plan_assigns_correct_tiers() {
    let plan = ImageBuildPlan {
        tag: "multi:latest".into(),
        base_image: Some("ubuntu:22.04".into()),
        layers: vec![
            LayerStrategy::Packages {
                names: vec!["nginx".into()],
            },
            LayerStrategy::Build {
                command: "make".into(),
                workdir: None,
            },
            LayerStrategy::Files {
                paths: vec!["/etc/app.conf".into()],
            },
            LayerStrategy::Derivation {
                store_path: "/nix/store/abc-pkg".into(),
            },
        ],
        labels: vec![],
        entrypoint: None,
    };
    let tiers = plan.tier_plan();
    assert_eq!(tiers.len(), 4);
    assert_eq!(tiers[0].0, 0); // Packages → tier 0
    assert_eq!(tiers[1].0, 1); // Build → tier 1
    assert_eq!(tiers[2].0, 2); // Files → tier 2
    assert_eq!(tiers[3].0, 3); // Derivation → tier 3
}

#[test]
fn f_2104_3_layer_strategy_from_file_resource() {
    let mut resource = forjar::core::types::Resource::default();
    resource.resource_type = forjar::core::types::ResourceType::File;
    resource.path = Some("/etc/app.conf".into());
    let strategy = LayerStrategy::from_resource(&resource);
    assert!(strategy.is_some());
    assert!(matches!(strategy.unwrap(), LayerStrategy::Files { .. }));
}

#[test]
fn f_2104_4_layer_strategy_from_package_resource() {
    let mut resource = forjar::core::types::Resource::default();
    resource.resource_type = forjar::core::types::ResourceType::Package;
    resource.packages = vec!["curl".into(), "jq".into()];
    let strategy = LayerStrategy::from_resource(&resource);
    assert!(strategy.is_some());
    if let Some(LayerStrategy::Packages { names }) = strategy {
        assert_eq!(names.len(), 2);
    } else {
        panic!("expected Packages strategy");
    }
}

// ── FJ-2105: Distribution Types ────────────────────────────────────

#[test]
fn f_2105_1_base_image_ref_default_registry() {
    let r = BaseImageRef::new("ubuntu:22.04");
    assert_eq!(r.registry(), "docker.io");
    assert!(!r.resolved);
    assert!(r.manifest_digest.is_none());
}

#[test]
fn f_2105_2_base_image_ref_custom_registry() {
    let r = BaseImageRef::new("ghcr.io/my-org/app:latest");
    assert_eq!(r.registry(), "ghcr.io");
}

#[test]
fn f_2105_3_oci_build_result_size_mb() {
    let result = OciBuildResult {
        tag: "test:latest".into(),
        manifest_digest: "sha256:abc".into(),
        layer_count: 2,
        total_size: 10 * 1024 * 1024,
        duration_secs: 1.5,
        layout_path: "/tmp/oci".into(),
    };
    assert!((result.size_mb() - 10.0).abs() < 0.01);
    let display = result.to_string();
    assert!(display.contains("test:latest"));
    assert!(display.contains("2 layers"));
}

// ── FJ-2106: Whiteout Conversion ───────────────────────────────────

#[test]
fn f_2106_1_file_delete_whiteout_path() {
    let w = WhiteoutEntry::FileDelete {
        path: "etc/nginx/site.conf".into(),
    };
    assert_eq!(w.oci_path(), "etc/nginx/.wh.site.conf");
}

#[test]
fn f_2106_2_opaque_dir_whiteout_path() {
    let w = WhiteoutEntry::OpaqueDir {
        path: "var/cache".into(),
    };
    assert_eq!(w.oci_path(), "var/cache/.wh..wh..opq");
}

#[test]
fn f_2106_3_root_level_file_delete() {
    let w = WhiteoutEntry::FileDelete {
        path: "readme.txt".into(),
    };
    assert_eq!(w.oci_path(), ".wh.readme.txt");
}

// ── FJ-2102: Compression Ratio ─────────────────────────────────────

#[test]
fn f_2102_10_compression_ratio_zero_size() {
    use forjar::core::types::LayerBuildResult;
    let result = LayerBuildResult {
        digest: "sha256:x".into(),
        diff_id: "sha256:y".into(),
        store_hash: "blake3:z".into(),
        compressed_size: 0,
        uncompressed_size: 0,
        compression: LayerCompression::Gzip,
        file_count: 0,
        build_path: LayerBuildPath::DirectAssembly,
    };
    assert!((result.compression_ratio() - 100.0).abs() < 0.01);
}

#[test]
fn f_2102_11_to_descriptor_media_types() {
    use forjar::core::types::LayerBuildResult;
    let make = |c: LayerCompression| LayerBuildResult {
        digest: "sha256:d".into(),
        diff_id: "sha256:dd".into(),
        store_hash: "blake3:s".into(),
        compressed_size: 100,
        uncompressed_size: 200,
        compression: c,
        file_count: 1,
        build_path: LayerBuildPath::DirectAssembly,
    };
    let gzip_desc = make(LayerCompression::Gzip).to_descriptor();
    assert!(gzip_desc.media_type.contains("gzip"));
    let zstd_desc = make(LayerCompression::Zstd).to_descriptor();
    assert!(zstd_desc.media_type.contains("zstd"));
    let none_desc = make(LayerCompression::None).to_descriptor();
    assert!(!none_desc.media_type.contains("gzip"));
    assert!(!none_desc.media_type.contains("zstd"));
    assert!(none_desc.media_type.ends_with(".tar"));
}

// ── FJ-2102: OCI Layout Write ──────────────────────────────────────

#[test]
fn f_2102_12_write_oci_layout_creates_structure() {
    use forjar::core::store::layer_builder::write_oci_layout;

    let dir = tempfile::TempDir::new().unwrap();
    let entries = vec![LayerEntry::file("hello.txt", b"hello", 0o644)];
    let config = OciLayerConfig::default();
    let (result, data) = build_layer(&entries, &config).unwrap();

    let config_json = br#"{"architecture":"amd64","os":"linux"}"#;
    write_oci_layout(dir.path(), &[(result, data)], config_json).unwrap();

    // oci-layout file must exist
    assert!(dir.path().join("oci-layout").exists());
    // blobs directory must exist
    assert!(dir.path().join("blobs/sha256").exists());
}
