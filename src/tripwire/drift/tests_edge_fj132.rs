use super::*;
use crate::core::types::Machine;
use crate::tripwire::hasher;

#[test]
fn test_fj132_detect_drift_with_local_machine() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("local.txt");
    std::fs::write(&file, "local content").unwrap();
    let hash = hasher::hash_file(&file).unwrap();

    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String(hash),
    );
    resources.insert(
        "local-file".to_string(),
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

    let findings = detect_drift_with_machine(&lock, &machine);
    assert!(
        findings.is_empty(),
        "no drift for matching file with local machine"
    );
}

#[test]
fn test_fj132_detect_drift_with_machine_drift_detected() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("tamper.txt");
    std::fs::write(&file, "original").unwrap();
    let hash = hasher::hash_file(&file).unwrap();
    std::fs::write(&file, "tampered").unwrap();

    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String(hash),
    );
    resources.insert(
        "tampered-file".to_string(),
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

    let machine = Machine {
        hostname: "local".to_string(),
        addr: "localhost".to_string(),
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

    let findings = detect_drift_with_machine(&lock, &machine);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].resource_id, "tampered-file");
}

#[test]
fn test_fj132_detect_drift_drifted_status_skipped() {
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String("/tmp/nonexistent".to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:abc".to_string()),
    );
    resources.insert(
        "drifted-resource".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Drifted,
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
        "drifted status resources should be skipped"
    );
}

#[test]
fn test_fj132_detect_drift_unknown_status_skipped() {
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String("/tmp/nonexistent".to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:abc".to_string()),
    );
    resources.insert(
        "unknown-resource".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Unknown,
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
        "unknown status resources should be skipped"
    );
}

#[test]
fn test_fj132_detect_drift_full_skips_file_resources() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.txt");
    std::fs::write(&file, "content").unwrap();
    let hash = hasher::hash_file(&file).unwrap();

    let mut lock_resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String(hash),
    );
    lock_resources.insert(
        "my-file".to_string(),
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
        "matching file should not trigger drift in full mode"
    );
}

#[test]
fn test_fj132_detect_drift_full_non_file_no_live_hash() {
    let mut lock_resources = indexmap::IndexMap::new();
    let details = std::collections::HashMap::new(); // no live_hash
    lock_resources.insert(
        "my-service".to_string(),
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

    let config_resources = indexmap::IndexMap::new();
    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert!(
        findings.is_empty(),
        "service without live_hash should be skipped"
    );
}
