use super::*;
use crate::core::types::Machine;
use crate::tripwire::hasher;

#[test]
fn test_fj132_detect_drift_full_non_file_non_string_live_hash() {
    let mut lock_resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42)),
    );
    lock_resources.insert(
        "my-package".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources: lock_resources,
    };

    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };

    let config_resources = indexmap::IndexMap::new();
    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert!(
        findings.is_empty(),
        "non-string live_hash should be skipped"
    );
}

#[test]
fn test_fj132_detect_drift_full_non_file_missing_config_resource() {
    let mut lock_resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:abc123".to_string()),
    );
    lock_resources.insert(
        "orphaned-service".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources: lock_resources,
    };

    let machine = Machine {
        hostname: "local".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };

    // Empty config resources — the lock has a resource that config doesn't
    let config_resources = indexmap::IndexMap::new();
    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert!(
        findings.is_empty(),
        "orphaned lock resource should be skipped"
    );
}

#[test]
fn test_fj132_drift_finding_resource_type_preserved() {
    let result = check_file_drift("test", "/nonexistent/path.txt", "blake3:abc");
    let finding = result.unwrap();
    assert_eq!(finding.resource_type, ResourceType::File);
    assert_eq!(finding.actual_hash, "MISSING");
}

#[test]
fn test_fj132_check_file_drift_hash_error_format() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "content").unwrap();

    let result = check_file_drift("test", file.to_str().unwrap(), "blake3:wrong");
    let finding = result.unwrap();
    assert!(
        finding.actual_hash.starts_with("blake3:"),
        "actual hash should be valid blake3"
    );
    assert_ne!(finding.actual_hash, "blake3:wrong");
}

#[test]
fn test_fj132_detect_drift_empty_lock() {
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources: indexmap::IndexMap::new(),
    };
    let findings = detect_drift(&lock);
    assert!(findings.is_empty(), "empty lock should produce no drift");
}

#[test]
fn test_fj132_detect_drift_skips_non_converged() {
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String("/etc/test.conf".to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:abc".to_string()),
    );
    resources.insert(
        "failed-file".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Failed,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            details,
        },
    );
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    let findings = detect_drift(&lock);
    assert!(findings.is_empty(), "failed resources should be skipped");
}

#[test]
fn test_fj132_detect_drift_file_without_path_skipped() {
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:abc".to_string()),
    );
    // No "path" key
    resources.insert(
        "no-path-file".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            details,
        },
    );
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    let findings = detect_drift(&lock);
    assert!(
        findings.is_empty(),
        "file without path in details should be skipped"
    );
}

#[test]
fn test_fj132_drift_finding_fields_complete() {
    let finding = DriftFinding {
        resource_id: "my-config".to_string(),
        resource_type: ResourceType::File,
        expected_hash: "blake3:expected".to_string(),
        actual_hash: "blake3:actual".to_string(),
        detail: "file content changed".to_string(),
    };
    assert_eq!(finding.resource_id, "my-config");
    assert_eq!(finding.resource_type, ResourceType::File);
    assert_ne!(finding.expected_hash, finding.actual_hash);
    assert!(!finding.detail.is_empty());
}

#[test]
fn test_fj132_detect_drift_matching_hash_no_drift() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("stable.txt");
    std::fs::write(&file, "stable content").unwrap();
    let hash = hasher::hash_file(&file).unwrap();

    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String(hash.clone()),
    );
    resources.insert(
        "stable-file".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            details,
        },
    );
    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    let findings = detect_drift(&lock);
    assert!(findings.is_empty(), "matching hash should produce no drift");
}
