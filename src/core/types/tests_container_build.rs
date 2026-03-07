//! Tests for container_build_types.rs — OCI layer config, build plan, whiteout entries.

use super::container_build_types::*;

#[test]
fn oci_layer_config_default() {
    let c = OciLayerConfig::default();
    assert_eq!(c.compression, OciCompression::Gzip);
    assert!(c.deterministic);
    assert_eq!(c.epoch_mtime, 0);
    assert_eq!(c.sort_order, TarSortOrder::Lexicographic);
}

#[test]
fn oci_compression_display_and_serde() {
    assert_eq!(OciCompression::None.to_string(), "none");
    assert_eq!(OciCompression::Gzip.to_string(), "gzip");
    assert_eq!(OciCompression::Zstd.to_string(), "zstd");
    let parsed: OciCompression =
        serde_json::from_str(&serde_json::to_string(&OciCompression::Zstd).unwrap()).unwrap();
    assert_eq!(parsed, OciCompression::Zstd);
}

#[test]
fn dual_digest_formats() {
    let d = DualDigest {
        blake3: "abcdef0123456789".into(),
        sha256: "deadbeef01234567".into(),
        size_bytes: 4096,
    };
    assert_eq!(d.oci_digest(), "sha256:deadbeef01234567");
    assert_eq!(d.forjar_digest(), "blake3:abcdef0123456789");
    let s = d.to_string();
    assert!(s.contains("blake3:abcdef01") && s.contains("sha256:deadbeef") && s.contains("4096B"));
}

#[test]
fn layer_cache_entry_serde() {
    let e = LayerCacheEntry {
        content_hash: "blake3:abc".into(),
        oci_digest: "sha256:def".into(),
        compressed_size: 1024,
        uncompressed_size: 4096,
        compression: OciCompression::Gzip,
        store_path: "store/abc".into(),
    };
    let json = serde_json::to_string(&e).unwrap();
    let parsed: LayerCacheEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.compressed_size, 1024);
}

#[test]
fn image_build_plan_basics() {
    let plan = ImageBuildPlan {
        tag: "app:v1".into(),
        base_image: Some("ubuntu:22.04".into()),
        layers: vec![
            LayerStrategy::Packages {
                names: vec!["nginx".into()],
            },
            LayerStrategy::Files {
                paths: vec!["/etc/app.conf".into()],
            },
        ],
        labels: vec![],
        entrypoint: None,
    };
    assert_eq!(plan.layer_count(), 2);
    assert!(!plan.is_scratch());
    let scratch = ImageBuildPlan {
        tag: "s:latest".into(),
        base_image: None,
        layers: vec![LayerStrategy::Files {
            paths: vec!["/app".into()],
        }],
        labels: vec![],
        entrypoint: Some(vec!["/app".into()]),
    };
    assert!(scratch.is_scratch());
}

#[test]
fn base_image_ref_registry() {
    assert_eq!(BaseImageRef::new("ubuntu:22.04").registry(), "docker.io");
    assert!(!BaseImageRef::new("ubuntu:22.04").resolved);
    assert_eq!(
        BaseImageRef::new("ghcr.io/org/app:v1").registry(),
        "ghcr.io"
    );
    assert_eq!(
        BaseImageRef::new("localhost:5000/myimage").registry(),
        "localhost:5000"
    );
}

#[test]
fn oci_build_result_display() {
    let r = OciBuildResult {
        tag: "app:v1".into(),
        manifest_digest: "sha256:abc".into(),
        layer_count: 3,
        total_size: 50 * 1024 * 1024,
        duration_secs: 12.5,
        layout_path: "out/oci".into(),
    };
    let s = r.to_string();
    assert!(s.contains("app:v1") && s.contains("3 layers") && s.contains("50.0 MB"));
}

#[test]
fn whiteout_oci_paths() {
    let w1 = WhiteoutEntry::FileDelete {
        path: "etc/nginx/old.conf".into(),
    };
    assert_eq!(w1.oci_path(), "etc/nginx/.wh.old.conf");
    let w2 = WhiteoutEntry::FileDelete {
        path: "orphan.txt".into(),
    };
    assert_eq!(w2.oci_path(), ".wh.orphan.txt");
    let w3 = WhiteoutEntry::OpaqueDir {
        path: "var/cache".into(),
    };
    assert_eq!(w3.oci_path(), "var/cache/.wh..wh..opq");
}

#[test]
fn layer_strategy_serde() {
    let pkg = LayerStrategy::Packages {
        names: vec!["nginx".into()],
    };
    let json = serde_json::to_string(&pkg).unwrap();
    assert!(json.contains("packages"));
    let build = LayerStrategy::Build {
        command: "make".into(),
        workdir: Some("/src".into()),
    };
    assert!(serde_json::to_string(&build).unwrap().contains("build"));
}

#[test]
fn layer_strategy_from_resource() {
    use crate::core::types::{Resource, ResourceType};
    let mut r = Resource::default();
    r.resource_type = ResourceType::Package;
    r.packages = vec!["nginx".into(), "curl".into()];
    let ls = LayerStrategy::from_resource(&r).unwrap();
    assert!(matches!(ls, LayerStrategy::Packages { names } if names.len() == 2));

    let mut r2 = Resource::default();
    r2.resource_type = ResourceType::File;
    r2.path = Some("/etc/app.conf".into());
    let ls2 = LayerStrategy::from_resource(&r2).unwrap();
    assert!(matches!(ls2, LayerStrategy::Files { paths } if paths[0] == "/etc/app.conf"));

    let mut r3 = Resource::default();
    r3.resource_type = ResourceType::Service;
    assert!(LayerStrategy::from_resource(&r3).is_none());
}

#[test]
fn tier_plan_ordering() {
    let plan = ImageBuildPlan {
        tag: "app:v1".into(),
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
        ],
        labels: vec![],
        entrypoint: None,
    };
    let tiers = plan.tier_plan();
    assert_eq!(tiers[0].0, 0); // Packages → tier 0
    assert_eq!(tiers[1].0, 1); // Build → tier 1
    assert_eq!(tiers[2].0, 2); // Files → tier 2
}
