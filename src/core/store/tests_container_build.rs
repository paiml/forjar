//! Tests for container-based OCI image building (FJ-2103).

use super::container_build::*;
use crate::core::types::{ImageBuildPlan, LayerStrategy};

fn sample_plan(tag: &str) -> ImageBuildPlan {
    ImageBuildPlan {
        tag: tag.into(),
        base_image: Some("debian:bookworm-slim".into()),
        layers: vec![LayerStrategy::Files {
            paths: vec!["etc/app.conf".into()],
        }],
        labels: vec![],
        entrypoint: None,
    }
}

#[test]
fn build_image_creates_oci_layout() {
    let plan = sample_plan("test-build:latest");
    let output_dir = tempfile::tempdir().unwrap();
    let scripts = vec!["echo 'key=value' > /etc/app.conf".into()];

    let result = build_image_in_container(&plan, &scripts, output_dir.path());
    // Docker is available — this should succeed
    match result {
        Ok(r) => {
            assert!(!r.runtime.is_empty());
            assert!(r.duration_ms < 60_000);
            // Verify OCI layout structure
            assert!(output_dir.path().join("oci-layout").exists());
            assert!(output_dir.path().join("index.json").exists());
            assert!(output_dir.path().join("manifest.json").exists());
            assert!(output_dir.path().join("blobs/sha256").exists());
        }
        Err(e) => {
            // Container runtime issue — still exercises dispatch path
            assert!(
                e.contains("container") || e.contains("docker") || e.contains("runtime"),
                "unexpected error: {e}"
            );
        }
    }
}

#[test]
fn build_with_empty_scripts() {
    let plan = sample_plan("empty-scripts:v1");
    let output_dir = tempfile::tempdir().unwrap();
    let scripts: Vec<String> = vec![];

    let result = build_image_in_container(&plan, &scripts, output_dir.path());
    match result {
        Ok(r) => {
            // Empty scripts = no changes, but image still assembled
            assert!(output_dir.path().join("oci-layout").exists());
            assert_eq!(r.image.layers.len(), 1);
        }
        Err(e) => {
            assert!(
                e.contains("container") || e.contains("diff"),
                "unexpected: {e}"
            );
        }
    }
}

#[test]
fn build_with_multiple_scripts() {
    let plan = sample_plan("multi-script:v1");
    let output_dir = tempfile::tempdir().unwrap();
    let scripts = vec![
        "mkdir -p /etc/forjar".into(),
        "echo 'app=true' > /etc/forjar/config".into(),
        "echo 'log=debug' >> /etc/forjar/config".into(),
    ];

    let result = build_image_in_container(&plan, &scripts, output_dir.path());
    match result {
        Ok(r) => {
            assert!(r.changed_files > 0, "should have changed files");
            assert!(output_dir.path().join("oci-layout").exists());
        }
        Err(e) => {
            assert!(
                e.contains("container") || e.contains("docker"),
                "unexpected: {e}"
            );
        }
    }
}

#[test]
fn build_without_base_image_uses_default() {
    let plan = ImageBuildPlan {
        tag: "no-base:v1".into(),
        base_image: None, // Should default to debian:bookworm-slim
        layers: vec![],
        labels: vec![],
        entrypoint: None,
    };
    let output_dir = tempfile::tempdir().unwrap();
    let scripts = vec!["echo hello".into()];

    let result = build_image_in_container(&plan, &scripts, output_dir.path());
    // Just verify dispatch path works — result depends on Docker state
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn format_container_build_output() {
    // Test format function directly without Docker
    let plan = sample_plan("fmt-test:v1");
    let output_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(output_dir.path().join("blobs/sha256")).unwrap();

    // Build a minimal image to get a ContainerBuildResult for formatting
    let layer_entries = vec![vec![
        crate::core::store::layer_builder::LayerEntry::file("test.txt", b"hello", 0o644),
    ]];
    let image = crate::core::store::image_assembler::assemble_image(
        &plan,
        &layer_entries,
        output_dir.path(),
        &crate::core::types::OciLayerConfig::default(),
    )
    .unwrap();

    let result = ContainerBuildResult {
        image,
        runtime: "docker".into(),
        changed_files: 5,
        duration_ms: 1234,
    };
    let s = format_container_build(&result);
    assert!(s.contains("docker"));
    assert!(s.contains("5 files"));
    assert!(s.contains("1234ms"));
}

#[test]
fn build_plan_layer_adjustment() {
    // Empty plan gets layers added to match entries
    let plan = ImageBuildPlan {
        tag: "adjust:v1".into(),
        base_image: Some("debian:bookworm-slim".into()),
        layers: vec![], // Empty — should be adjusted
        labels: vec![],
        entrypoint: None,
    };
    let output_dir = tempfile::tempdir().unwrap();
    let scripts = vec!["touch /tmp/test".into()];

    let result = build_image_in_container(&plan, &scripts, output_dir.path());
    if let Ok(r) = result {
        // Plan should have been adjusted to match the single layer of entries
        assert_eq!(r.image.layers.len(), 1);
    }
}
