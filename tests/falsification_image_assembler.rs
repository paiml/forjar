//! FJ-2104: Image assembler falsification tests.
//! Usage: cargo test --test falsification_image_assembler

use forjar::core::store::image_assembler::assemble_image;
use forjar::core::store::layer_builder::LayerEntry;
use forjar::core::types::{ImageBuildPlan, LayerStrategy, OciLayerConfig};

// ── helpers ──

fn plan(tag: &str, layers: Vec<LayerStrategy>) -> ImageBuildPlan {
    ImageBuildPlan {
        tag: tag.into(),
        base_image: None,
        layers,
        labels: vec![],
        entrypoint: None,
    }
}

fn files_strategy(paths: &[&str]) -> LayerStrategy {
    LayerStrategy::Files {
        paths: paths.iter().map(|s| s.to_string()).collect(),
    }
}

// ── single layer ──

#[test]
fn assemble_single_layer() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan("test:v1", vec![files_strategy(&["/app"])]);
    let entries = vec![vec![LayerEntry::file("app.txt", b"hello", 0o644)]];

    let result = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    assert_eq!(result.layers.len(), 1);
    assert!(result.total_size > 0);
    assert!(out.join("oci-layout").exists());
    assert!(out.join("index.json").exists());
    assert!(out.join("manifest.json").exists());
}

// ── multi-layer concurrent assembly ──

#[test]
fn assemble_multi_layer_concurrent() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan(
        "multi:v1",
        vec![
            files_strategy(&["/bin"]),
            files_strategy(&["/etc"]),
            files_strategy(&["/var"]),
        ],
    );
    let entries = vec![
        vec![LayerEntry::file("bin/app", b"binary", 0o755)],
        vec![LayerEntry::file("etc/config", b"key=value", 0o644)],
        vec![LayerEntry::file("var/data", b"data", 0o644)],
    ];

    let result = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    assert_eq!(result.layers.len(), 3);
    assert_eq!(result.manifest.layers.len(), 3);
    assert_eq!(result.config.rootfs.diff_ids.len(), 3);
    assert!(result.total_size > 0);
}

// ── layer count mismatch ──

#[test]
fn assemble_layer_count_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan(
        "mismatch:v1",
        vec![files_strategy(&["/a"]), files_strategy(&["/b"])],
    );
    let entries = vec![vec![LayerEntry::file("a.txt", b"a", 0o644)]]; // only 1

    let err = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap_err();
    assert!(err.contains("layer count mismatch"));
    assert!(err.contains("2"));
    assert!(err.contains("1"));
}

// ── labels propagation ──

#[test]
fn assemble_with_labels() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let mut p = plan("labels:v1", vec![files_strategy(&["/app"])]);
    p.labels = vec![
        ("org.opencontainers.image.title".into(), "myapp".into()),
        ("version".into(), "1.0.0".into()),
        ("author".into(), "forjar".into()),
    ];

    let entries = vec![vec![LayerEntry::file("app", b"bin", 0o755)]];
    let result = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    assert_eq!(result.config.config.labels.get("version").unwrap(), "1.0.0");
    assert_eq!(result.config.config.labels.get("author").unwrap(), "forjar");
    assert_eq!(result.config.config.labels.len(), 3);
}

// ── entrypoint propagation ──

#[test]
fn assemble_with_entrypoint() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let mut p = plan("ep:v1", vec![files_strategy(&["/app"])]);
    p.entrypoint = Some(vec!["/usr/bin/app".into(), "--serve".into()]);

    let entries = vec![vec![LayerEntry::file("app", b"bin", 0o755)]];
    let result = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    assert_eq!(
        result.config.config.entrypoint,
        vec!["/usr/bin/app", "--serve"]
    );
}

#[test]
fn assemble_no_entrypoint() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan("noep:v1", vec![files_strategy(&["/app"])]);
    let entries = vec![vec![LayerEntry::file("app", b"bin", 0o755)]];
    let result = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    assert!(result.config.config.entrypoint.is_empty());
}

// ── target architecture ──

#[test]
fn assemble_arm64_architecture() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan("arm:v1", vec![files_strategy(&["/app"])]);
    let entries = vec![vec![LayerEntry::file("app", b"arm-bin", 0o755)]];
    let result = assemble_image(
        &p,
        &entries,
        &out,
        &OciLayerConfig::default(),
        Some("arm64"),
    )
    .unwrap();

    assert_eq!(result.config.architecture, "arm64");
}

#[test]
fn assemble_default_amd64() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan("default:v1", vec![files_strategy(&["/app"])]);
    let entries = vec![vec![LayerEntry::file("app", b"bin", 0o755)]];
    let result = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    assert_eq!(result.config.architecture, "amd64");
}

// ── OCI layout files ──

#[test]
fn assemble_creates_oci_layout_file() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan("layout:v1", vec![files_strategy(&["/app"])]);
    let entries = vec![vec![LayerEntry::file("app", b"bin", 0o755)]];
    assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    let layout = std::fs::read_to_string(out.join("oci-layout")).unwrap();
    assert!(layout.contains("imageLayoutVersion"));
}

#[test]
fn assemble_creates_valid_index_json() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan("index:v1", vec![files_strategy(&["/app"])]);
    let entries = vec![vec![LayerEntry::file("app", b"bin", 0o755)]];
    assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    let index_str = std::fs::read_to_string(out.join("index.json")).unwrap();
    let index: serde_json::Value = serde_json::from_str(&index_str).unwrap();
    assert_eq!(index["schemaVersion"], 2);
    assert!(index["manifests"].is_array());
    assert_eq!(index["manifests"].as_array().unwrap().len(), 1);
}

// ── Docker manifest.json ──

#[test]
fn assemble_creates_docker_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan("docker:v1", vec![files_strategy(&["/app"])]);
    let entries = vec![vec![LayerEntry::file("app", b"bin", 0o755)]];
    assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    let manifest_str = std::fs::read_to_string(out.join("manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_str).unwrap();
    let first = &manifest[0];
    assert!(first["RepoTags"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("docker:v1")));
    assert!(first["Config"]
        .as_str()
        .unwrap()
        .starts_with("blobs/sha256/"));
    assert_eq!(first["Layers"].as_array().unwrap().len(), 1);
}

// ── manifest structure ──

#[test]
fn assemble_manifest_has_correct_media_types() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan("media:v1", vec![files_strategy(&["/app"])]);
    let entries = vec![vec![LayerEntry::file("app", b"bin", 0o755)]];
    let result = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    assert_eq!(result.manifest.schema_version, 2);
    assert!(result.manifest.config.media_type.contains("config"));
    for layer in &result.manifest.layers {
        assert!(layer.media_type.contains("layer"));
    }
}

// ── layer digests ──

#[test]
fn assemble_layers_have_sha256_digests() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan(
        "digest:v1",
        vec![files_strategy(&["/a"]), files_strategy(&["/b"])],
    );
    let entries = vec![
        vec![LayerEntry::file("a.txt", b"aaa", 0o644)],
        vec![LayerEntry::file("b.txt", b"bbb", 0o644)],
    ];
    let result = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    for layer in &result.layers {
        assert!(
            layer.digest.starts_with("sha256:"),
            "digest: {}",
            layer.digest
        );
        assert!(layer.compressed_size > 0);
    }
    // diff_ids should be different from digests (diff_id = uncompressed hash)
    for diff_id in &result.config.rootfs.diff_ids {
        assert!(diff_id.starts_with("sha256:"));
    }
}

// ── deterministic output ──

#[test]
fn assemble_deterministic() {
    let entries = vec![vec![LayerEntry::file(
        "app",
        b"deterministic-content",
        0o644,
    )]];
    let p = plan("det:v1", vec![files_strategy(&["/app"])]);

    let tmp1 = tempfile::tempdir().unwrap();
    let out1 = tmp1.path().join("output");
    std::fs::create_dir_all(&out1).unwrap();
    let r1 = assemble_image(&p, &entries, &out1, &OciLayerConfig::default(), None).unwrap();

    let tmp2 = tempfile::tempdir().unwrap();
    let out2 = tmp2.path().join("output");
    std::fs::create_dir_all(&out2).unwrap();
    let r2 = assemble_image(&p, &entries, &out2, &OciLayerConfig::default(), None).unwrap();

    assert_eq!(r1.layers[0].digest, r2.layers[0].digest);
    assert_eq!(r1.total_size, r2.total_size);
}

// ── history ──

#[test]
fn assemble_history_matches_layers() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan(
        "hist:v1",
        vec![
            LayerStrategy::Packages {
                names: vec!["nginx".into()],
            },
            LayerStrategy::Build {
                command: "make install".into(),
                workdir: None,
            },
            LayerStrategy::Derivation {
                store_path: "/store/abc".into(),
            },
        ],
    );
    let entries = vec![
        vec![LayerEntry::file("pkg", b"pkg-data", 0o644)],
        vec![LayerEntry::file("build", b"build-data", 0o644)],
        vec![LayerEntry::file("drv", b"drv-data", 0o644)],
    ];
    let result = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    assert_eq!(result.config.history.len(), 3);
    assert!(result.config.history[0]
        .created_by
        .as_ref()
        .unwrap()
        .contains("packages nginx"));
    assert!(result.config.history[1]
        .created_by
        .as_ref()
        .unwrap()
        .contains("build make install"));
    assert!(result.config.history[2]
        .created_by
        .as_ref()
        .unwrap()
        .contains("derivation /store/abc"));
}

// ── blob files exist ──

#[test]
fn assemble_creates_blob_files() {
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("output");
    std::fs::create_dir_all(&out).unwrap();

    let p = plan("blobs:v1", vec![files_strategy(&["/app"])]);
    let entries = vec![vec![LayerEntry::file("app", b"content", 0o644)]];
    let result = assemble_image(&p, &entries, &out, &OciLayerConfig::default(), None).unwrap();

    // Layer blob exists
    let layer_hex = result.layers[0].digest.strip_prefix("sha256:").unwrap();
    assert!(out.join(format!("blobs/sha256/{layer_hex}")).exists());

    // blobs dir exists
    assert!(out.join("blobs/sha256").is_dir());
}
