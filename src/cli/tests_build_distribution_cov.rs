//! Coverage tests for build_distribution.rs — FAR archive creation.

use super::build_distribution::*;

// ── cmd_build_far ────────────────────────────────────────────────────

#[test]
fn build_far_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    std::fs::create_dir_all(&oci).unwrap();
    let result = cmd_build_far("test-image", &oci);
    assert!(result.is_ok());
    let far_path = oci.with_extension("far");
    assert!(far_path.exists());
}

#[test]
fn build_far_with_files() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    std::fs::create_dir_all(&oci).unwrap();
    std::fs::write(oci.join("manifest.json"), r#"{"schemaVersion":2}"#).unwrap();
    std::fs::write(oci.join("index.json"), r#"{"mediaType":"application/vnd.oci.image.index.v1+json"}"#).unwrap();
    let result = cmd_build_far("my-app", &oci);
    assert!(result.is_ok());
    let far_path = oci.with_extension("far");
    assert!(far_path.exists());
    assert!(far_path.metadata().unwrap().len() > 0);
}

#[test]
fn build_far_nested_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    std::fs::create_dir_all(oci.join("blobs/sha256")).unwrap();
    std::fs::write(oci.join("blobs/sha256/abc123"), "layer data").unwrap();
    std::fs::write(oci.join("oci-layout"), r#"{"imageLayoutVersion":"1.0.0"}"#).unwrap();
    let result = cmd_build_far("nested-test", &oci);
    assert!(result.is_ok());
}

// ── cmd_build_push ───────────────────────────────────────────────────

#[test]
fn build_push_no_blobs() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    std::fs::create_dir_all(&oci).unwrap();

    let res = crate::core::types::Resource {
        resource_type: crate::core::types::ResourceType::Docker,
        machine: crate::core::types::MachineTarget::Single("local".to_string()),
        name: Some("test/app".to_string()),
        version: Some("v1.0".to_string()),
        ..Default::default()
    };
    let result = cmd_build_push(&res, &oci);
    assert!(result.is_ok());
}

#[test]
fn build_push_with_registry_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    std::fs::create_dir_all(&oci).unwrap();

    let res = crate::core::types::Resource {
        resource_type: crate::core::types::ResourceType::Docker,
        machine: crate::core::types::MachineTarget::Single("local".to_string()),
        name: Some("ghcr.io/myorg/app".to_string()),
        version: Some("latest".to_string()),
        ..Default::default()
    };
    let result = cmd_build_push(&res, &oci);
    assert!(result.is_ok());
}

#[test]
fn build_push_default_name_and_tag() {
    let dir = tempfile::tempdir().unwrap();
    let oci = dir.path().join("oci");
    std::fs::create_dir_all(&oci).unwrap();

    let res = crate::core::types::Resource {
        resource_type: crate::core::types::ResourceType::Docker,
        machine: crate::core::types::MachineTarget::Single("local".to_string()),
        name: None,
        version: None,
        ..Default::default()
    };
    let result = cmd_build_push(&res, &oci);
    assert!(result.is_ok());
}
