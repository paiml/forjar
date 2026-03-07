//! Tests for FJ-2104: OCI image assembler.

use super::image_assembler::*;
use super::layer_builder::LayerEntry;
use crate::core::types::{ImageBuildPlan, LayerStrategy, OciLayerConfig};

fn test_plan() -> ImageBuildPlan {
    ImageBuildPlan {
        tag: "test/myapp:v1.0".into(),
        base_image: Some("ubuntu:22.04".into()),
        layers: vec![
            LayerStrategy::Files {
                paths: vec!["/etc/app/config.yaml".into()],
            },
            LayerStrategy::Files {
                paths: vec!["/usr/local/bin/myapp".into()],
            },
        ],
        labels: vec![("maintainer".into(), "test@example.com".into())],
        entrypoint: Some(vec!["/usr/local/bin/myapp".into()]),
    }
}

fn test_entries() -> Vec<Vec<LayerEntry>> {
    vec![
        vec![
            LayerEntry::dir("etc/app/", 0o755),
            LayerEntry::file("etc/app/config.yaml", b"port: 8080\n", 0o644),
        ],
        vec![
            LayerEntry::dir("usr/local/bin/", 0o755),
            LayerEntry::file("usr/local/bin/myapp", b"#!/bin/sh\necho hi\n", 0o755),
        ],
    ]
}

#[test]
fn assemble_two_layer_image() {
    let dir = tempfile::tempdir().unwrap();
    let plan = test_plan();
    let entries = test_entries();
    let config = OciLayerConfig::default();

    let result = assemble_image(&plan, &entries, dir.path(), &config).unwrap();

    assert_eq!(result.layers.len(), 2);
    assert!(result.total_size > 0);
    assert_eq!(result.manifest.layers.len(), 2);
    assert_eq!(result.config.layer_count(), 2);
}

#[test]
fn assemble_creates_oci_layout() {
    let dir = tempfile::tempdir().unwrap();
    let result = assemble_image(
        &test_plan(),
        &test_entries(),
        dir.path(),
        &OciLayerConfig::default(),
    )
    .unwrap();

    // OCI layout files
    assert!(dir.path().join("oci-layout").exists());
    assert!(dir.path().join("index.json").exists());
    assert!(dir.path().join("manifest.json").exists());
    assert!(dir.path().join("blobs/sha256").is_dir());

    // Layer blobs exist
    for layer in &result.layers {
        let hex = layer.digest.strip_prefix("sha256:").unwrap();
        assert!(dir.path().join(format!("blobs/sha256/{hex}")).exists());
    }
}

#[test]
fn assemble_index_json_valid() {
    let dir = tempfile::tempdir().unwrap();
    assemble_image(
        &test_plan(),
        &test_entries(),
        dir.path(),
        &OciLayerConfig::default(),
    )
    .unwrap();

    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("index.json")).unwrap())
            .unwrap();
    assert_eq!(index["schemaVersion"], 2);
    assert_eq!(index["manifests"].as_array().unwrap().len(), 1);
    assert!(index["manifests"][0]["digest"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
}

#[test]
fn assemble_docker_compat_manifest() {
    let dir = tempfile::tempdir().unwrap();
    assemble_image(
        &test_plan(),
        &test_entries(),
        dir.path(),
        &OciLayerConfig::default(),
    )
    .unwrap();

    let docker: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("manifest.json")).unwrap())
            .unwrap();
    let entry = &docker[0];
    assert_eq!(entry["RepoTags"][0], "test/myapp:v1.0");
    assert!(entry["Config"]
        .as_str()
        .unwrap()
        .starts_with("blobs/sha256/"));
    assert_eq!(entry["Layers"].as_array().unwrap().len(), 2);
}

#[test]
fn assemble_sets_entrypoint_and_labels() {
    let dir = tempfile::tempdir().unwrap();
    let result = assemble_image(
        &test_plan(),
        &test_entries(),
        dir.path(),
        &OciLayerConfig::default(),
    )
    .unwrap();

    assert_eq!(
        result.config.config.entrypoint,
        vec!["/usr/local/bin/myapp"]
    );
    assert_eq!(
        result.config.config.labels.get("maintainer").unwrap(),
        "test@example.com"
    );
}

#[test]
fn assemble_history_entries() {
    let dir = tempfile::tempdir().unwrap();
    let result = assemble_image(
        &test_plan(),
        &test_entries(),
        dir.path(),
        &OciLayerConfig::default(),
    )
    .unwrap();

    assert_eq!(result.config.history.len(), 2);
    assert!(result.config.history[0]
        .created_by
        .as_ref()
        .unwrap()
        .contains("files"));
    assert!(!result.config.history[0].empty_layer);
}

#[test]
fn assemble_determinism() {
    let dir1 = tempfile::tempdir().unwrap();
    let dir2 = tempfile::tempdir().unwrap();
    let plan = test_plan();
    let entries = test_entries();
    let config = OciLayerConfig::default();

    let r1 = assemble_image(&plan, &entries, dir1.path(), &config).unwrap();
    let r2 = assemble_image(&plan, &entries, dir2.path(), &config).unwrap();

    for (l1, l2) in r1.layers.iter().zip(r2.layers.iter()) {
        assert_eq!(l1.digest, l2.digest, "layer digests must match");
        assert_eq!(l1.diff_id, l2.diff_id, "DiffIDs must match");
    }
}

#[test]
fn assemble_mismatch_layer_count() {
    let dir = tempfile::tempdir().unwrap();
    let plan = test_plan(); // 2 layers
    let entries = vec![vec![LayerEntry::file("a", b"a", 0o644)]]; // 1 entry set

    let err = assemble_image(&plan, &entries, dir.path(), &OciLayerConfig::default()).unwrap_err();
    assert!(
        err.contains("mismatch"),
        "error should mention mismatch: {err}"
    );
}

#[test]
fn assemble_single_layer_scratch() {
    let dir = tempfile::tempdir().unwrap();
    let plan = ImageBuildPlan {
        tag: "scratch:latest".into(),
        base_image: None,
        layers: vec![LayerStrategy::Files {
            paths: vec!["/app".into()],
        }],
        labels: vec![],
        entrypoint: Some(vec!["/app".into()]),
    };
    let entries = vec![vec![LayerEntry::file("app", b"binary", 0o755)]];

    let result = assemble_image(&plan, &entries, dir.path(), &OciLayerConfig::default()).unwrap();
    assert_eq!(result.layers.len(), 1);
    assert_eq!(result.config.layer_count(), 1);
}

#[test]
fn assemble_no_entrypoint_no_labels() {
    let dir = tempfile::tempdir().unwrap();
    let plan = ImageBuildPlan {
        tag: "base:latest".into(),
        base_image: Some("ubuntu:22.04".into()),
        layers: vec![LayerStrategy::Packages {
            names: vec!["nginx".into()],
        }],
        labels: vec![],
        entrypoint: None,
    };
    let entries = vec![vec![LayerEntry::file(
        "usr/sbin/nginx",
        b"nginx-bin",
        0o755,
    )]];

    let result = assemble_image(&plan, &entries, dir.path(), &OciLayerConfig::default()).unwrap();
    assert!(result.config.config.entrypoint.is_empty());
    assert!(result.config.config.labels.is_empty());
}
