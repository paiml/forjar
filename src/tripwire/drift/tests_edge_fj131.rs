use super::*;
use crate::core::types::Machine;
use crate::tripwire::hasher;

#[test]
fn test_fj131_check_file_drift_directory() {
    // check_file_drift should hash directories too
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("subdir");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("a.txt"), "content").unwrap();
    let hash = hasher::hash_directory(&sub).unwrap();

    let result = check_file_drift("dir-resource", sub.to_str().unwrap(), &hash);
    assert!(
        result.is_none(),
        "directory with matching hash should not drift"
    );
}

#[test]
fn test_fj131_check_file_drift_directory_changed() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("subdir");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("a.txt"), "original").unwrap();
    let hash = hasher::hash_directory(&sub).unwrap();

    // Modify directory contents
    std::fs::write(sub.join("a.txt"), "modified").unwrap();

    let result = check_file_drift("dir-resource", sub.to_str().unwrap(), &hash);
    assert!(result.is_some(), "changed directory should drift");
}

#[test]
fn test_fj131_drift_finding_debug() {
    let f = DriftFinding {
        resource_id: "test".to_string(),
        resource_type: ResourceType::File,
        expected_hash: "a".to_string(),
        actual_hash: "b".to_string(),
        detail: "changed".to_string(),
    };
    let debug = format!("{:?}", f);
    assert!(debug.contains("test"));
    assert!(debug.contains("changed"));
}

#[test]
fn test_fj131_drift_finding_clone() {
    let f = DriftFinding {
        resource_id: "res".to_string(),
        resource_type: ResourceType::Service,
        expected_hash: "h1".to_string(),
        actual_hash: "h2".to_string(),
        detail: "state changed".to_string(),
    };
    let cloned = f.clone();
    assert_eq!(cloned.resource_id, "res");
    assert_eq!(cloned.actual_hash, "h2");
}

#[test]
fn test_fj131_detect_drift_skips_failed_resources() {
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String("/tmp/nonexistent".to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:stale".to_string()),
    );
    resources.insert(
        "failed-file".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Failed, // not converged
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:x".to_string(),
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
fn test_fj131_detect_drift_skips_non_string_path() {
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    // Path is a number instead of string — should be skipped
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(42)),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:x".to_string()),
    );
    resources.insert(
        "bad-path".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:x".to_string(),
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
    assert!(findings.is_empty(), "non-string path should be skipped");
}

#[test]
fn test_fj131_detect_drift_skips_non_string_content_hash() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "content").unwrap();

    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    // content_hash is a bool instead of string
    details.insert("content_hash".to_string(), serde_yaml_ng::Value::Bool(true));
    resources.insert(
        "bad-hash".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:x".to_string(),
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
        "non-string content_hash should be skipped"
    );
}

#[test]
fn test_fj131_detect_drift_no_content_hash_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "content").unwrap();

    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    // Has path but no content_hash at all
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    resources.insert(
        "no-hash".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:x".to_string(),
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
        "missing content_hash should be skipped"
    );
}

#[test]
fn test_fj131_detect_drift_skips_non_file_resources() {
    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "my-svc".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:x".to_string(),
            details: std::collections::HashMap::new(),
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

    // detect_drift (not detect_drift_full) only checks files
    let findings = detect_drift(&lock);
    assert!(
        findings.is_empty(),
        "service resources should be skipped by detect_drift"
    );
}

#[test]
fn test_fj131_detect_drift_multiple_resources() {
    let dir = tempfile::tempdir().unwrap();
    let file1 = dir.path().join("ok.txt");
    let file2 = dir.path().join("drifted.txt");
    std::fs::write(&file1, "stable").unwrap();
    std::fs::write(&file2, "original").unwrap();
    let hash1 = hasher::hash_file(&file1).unwrap();
    let hash2 = hasher::hash_file(&file2).unwrap();

    // Tamper with file2
    std::fs::write(&file2, "tampered").unwrap();

    let mut resources = indexmap::IndexMap::new();
    for (name, file, hash) in [
        ("ok-file", &file1, &hash1),
        ("drifted-file", &file2, &hash2),
    ] {
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
            name.to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:desired".to_string(),
                details,
            },
        );
    }

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
    assert_eq!(findings.len(), 1, "only drifted file should be reported");
    assert_eq!(findings[0].resource_id, "drifted-file");
}
