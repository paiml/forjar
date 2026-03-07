//! FJ-2106/E15: Image drift detection tests.

use super::*;
use crate::core::types::{ResourceLock, ResourceStatus, ResourceType, StateLock};
use indexmap::IndexMap;
use std::collections::HashMap;

fn make_image_lock(resource_id: &str, manifest_digest: &str, container_name: &str) -> StateLock {
    let mut details = HashMap::new();
    details.insert(
        "manifest_digest".to_string(),
        serde_yaml_ng::Value::String(manifest_digest.into()),
    );
    details.insert(
        "container_name".to_string(),
        serde_yaml_ng::Value::String(container_name.into()),
    );
    let rl = ResourceLock {
        resource_type: ResourceType::Image,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: "hash123".into(),
        details,
    };
    let mut resources = IndexMap::new();
    resources.insert(resource_id.to_string(), rl);
    StateLock {
        schema: "1".into(),
        machine: "builder".into(),
        hostname: "build-01".into(),
        generated_at: "2026-03-07T00:00:00Z".into(),
        generator: "forjar-test".into(),
        blake3_version: "1.5.0".into(),
        resources,
    }
}

#[test]
fn detect_image_drift_skips_non_image() {
    let mut lock = make_image_lock("app", "sha256:abc", "my-app");
    lock.resources.get_mut("app").unwrap().resource_type = ResourceType::File;
    let machine: Machine = serde_yaml_ng::from_str("hostname: m\naddr: 127.0.0.1").unwrap();
    let resources = IndexMap::new();
    let findings = detect_image_drift(&lock, &machine, &resources);
    assert!(findings.is_empty(), "non-image resources should be skipped");
}

#[test]
fn detect_image_drift_skips_non_converged() {
    let mut lock = make_image_lock("app", "sha256:abc", "my-app");
    lock.resources.get_mut("app").unwrap().status = ResourceStatus::Unknown;
    let machine: Machine = serde_yaml_ng::from_str("hostname: m\naddr: 127.0.0.1").unwrap();
    let resources = IndexMap::new();
    let findings = detect_image_drift(&lock, &machine, &resources);
    assert!(
        findings.is_empty(),
        "non-converged resources should be skipped"
    );
}

#[test]
fn detect_image_drift_skips_missing_digest() {
    let mut lock = make_image_lock("app", "sha256:abc", "my-app");
    lock.resources
        .get_mut("app")
        .unwrap()
        .details
        .remove("manifest_digest");
    let machine: Machine = serde_yaml_ng::from_str("hostname: m\naddr: 127.0.0.1").unwrap();
    let resources = IndexMap::new();
    let findings = detect_image_drift(&lock, &machine, &resources);
    assert!(findings.is_empty(), "missing digest should be skipped");
}

#[test]
fn detect_image_drift_skips_missing_container_name() {
    let mut lock = make_image_lock("app", "sha256:abc", "my-app");
    lock.resources
        .get_mut("app")
        .unwrap()
        .details
        .remove("container_name");
    let machine: Machine = serde_yaml_ng::from_str("hostname: m\naddr: 127.0.0.1").unwrap();
    let resources = IndexMap::new();
    let findings = detect_image_drift(&lock, &machine, &resources);
    assert!(
        findings.is_empty(),
        "missing container_name should be skipped"
    );
}

#[test]
fn check_image_drift_not_running() {
    let machine: Machine = serde_yaml_ng::from_str("hostname: m\naddr: 127.0.0.1").unwrap();
    let result = check_image_drift("app", "nonexistent-container-xyz", "sha256:abc", &machine);
    // docker inspect will fail on non-existent container → drift finding
    assert!(
        result.is_some(),
        "non-existent container should produce drift finding"
    );
    let f = result.unwrap();
    assert_eq!(f.resource_id, "app");
    assert_eq!(f.resource_type, ResourceType::Image);
}

#[test]
fn drift_finding_image_fields() {
    let f = DriftFinding {
        resource_id: "my-image".into(),
        resource_type: ResourceType::Image,
        expected_hash: "sha256:expected".into(),
        actual_hash: "sha256:actual".into(),
        detail: "deployed image differs from built image".into(),
    };
    assert_eq!(f.resource_type, ResourceType::Image);
    assert_eq!(f.expected_hash, "sha256:expected");
    assert_eq!(f.actual_hash, "sha256:actual");
}

#[test]
fn detect_image_drift_respects_ignore_drift() {
    let lock = make_image_lock("app", "sha256:abc", "my-app");
    let machine: Machine = serde_yaml_ng::from_str("hostname: m\naddr: 127.0.0.1").unwrap();

    let yaml = r#"
type: image
name: myapp
version: "1.0.0"
path: /opt/app
command: "/opt/app/server"
lifecycle:
  ignore_drift: ["*"]
"#;
    let resource: Resource = serde_yaml_ng::from_str(yaml).unwrap();
    let mut resources = IndexMap::new();
    resources.insert("app".to_string(), resource);

    let findings = detect_image_drift(&lock, &machine, &resources);
    assert!(
        findings.is_empty(),
        "resources with ignore_drift should be skipped"
    );
}
