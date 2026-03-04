use super::*;
use crate::tripwire::hasher;

// ── FJ-036: Drift detection tests ─────────────────────────────

#[test]
fn test_fj036_drift_with_changed_hash() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("config.conf");
    std::fs::write(&file, "version=1").unwrap();
    let hash_a = hasher::hash_file(&file).unwrap();

    // Modify the file so it now has hash B
    std::fs::write(&file, "version=2").unwrap();
    let hash_b = hasher::hash_file(&file).unwrap();
    assert_ne!(hash_a, hash_b, "precondition: hashes must differ");

    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String(hash_a.clone()),
    );
    resources.insert(
        "config-file".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-02-25T00:00:00Z".to_string()),
            duration_seconds: Some(0.05),
            hash: "blake3:desired".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test-box".to_string(),
        generated_at: "2026-02-25T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };

    let findings = detect_drift(&lock);
    assert_eq!(
        findings.len(),
        1,
        "drift must be detected when hash changed"
    );
    assert_eq!(findings[0].resource_id, "config-file");
    assert_eq!(findings[0].expected_hash, hash_a);
    assert_eq!(findings[0].actual_hash, hash_b);
    assert!(
        findings[0].detail.contains("content changed"),
        "detail should mention content changed"
    );
}

#[test]
fn test_fj036_drift_absent_resource_no_drift() {
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String("/etc/removed/old.conf".to_string()),
    );
    details.insert(
        "state".to_string(),
        serde_yaml_ng::Value::String("absent".to_string()),
    );
    // No content_hash key — absent resources have nothing to hash
    resources.insert(
        "removed-config".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-02-25T00:00:00Z".to_string()),
            duration_seconds: Some(0.01),
            hash: "blake3:desired".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test-box".to_string(),
        generated_at: "2026-02-25T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };

    let findings = detect_drift(&lock);
    assert!(
        findings.is_empty(),
        "absent resource with no content_hash and no file on disk should not report drift"
    );
}

// -- Coverage boost tests --

#[test]
fn test_detect_drift_service_resource() {
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "name".to_string(),
        serde_yaml_ng::Value::String("nginx".to_string()),
    );
    details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:svc-hash-123".to_string()),
    );
    resources.insert(
        "nginx-service".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-02-25T10:00:00Z".to_string()),
            duration_seconds: Some(1.5),
            hash: "blake3:svc-hash-123".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test-box".to_string(),
        generated_at: "2026-02-25T10:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };

    let findings = detect_drift(&lock);
    assert!(
        findings.is_empty(),
        "detect_drift should skip service resources: {findings:?}"
    );
}

#[test]
fn test_detect_drift_directory_resource() {
    let dir = tempfile::tempdir().unwrap();
    let subdir = dir.path().join("config.d");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("a.conf"), "setting=1").unwrap();

    let dir_hash = hasher::hash_directory(&subdir).unwrap();

    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(subdir.to_str().unwrap().to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String(dir_hash.clone()),
    );
    resources.insert(
        "config-dir".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:dir".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test-box".to_string(),
        generated_at: "2026-02-25T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };

    let findings = detect_drift(&lock);
    assert!(
        findings.is_empty(),
        "directory with matching hash should have no drift: {findings:?}"
    );

    std::fs::write(subdir.join("b.conf"), "new-setting=2").unwrap();
    let findings = detect_drift(&lock);
    assert_eq!(
        findings.len(),
        1,
        "directory content change should trigger drift"
    );
    assert_eq!(findings[0].resource_id, "config-dir");
    assert!(
        findings[0].detail.contains("content changed"),
        "detail should mention content changed: {}",
        findings[0].detail
    );
}

#[test]
fn test_check_file_drift_nonexistent_file() {
    let result = check_file_drift(
        "vanished-config",
        "/tmp/forjar-test-nonexistent-82739182/missing.conf",
        "blake3:expected-hash-abc",
    );
    assert!(result.is_some(), "nonexistent file must produce a finding");
    let finding = result.unwrap();
    assert_eq!(finding.resource_id, "vanished-config");
    assert_eq!(finding.actual_hash, "MISSING");
    assert_eq!(finding.expected_hash, "blake3:expected-hash-abc");
    assert!(
        finding.detail.contains("does not exist"),
        "detail must say file does not exist: {}",
        finding.detail
    );
    assert_eq!(
        finding.resource_type,
        ResourceType::File,
        "resource_type should be File"
    );
}
