use super::*;
use crate::core::types::Machine;
use crate::tripwire::hasher;

#[test]
fn test_fj016_check_file_drift_transport_local() {
    // check_file_drift_via_transport on a local file should work
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("transport-test.txt");
    std::fs::write(&file, "via transport").unwrap();
    let expected = hasher::hash_string("via transport");

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
    };

    let finding = check_file_drift_via_transport("f", file.to_str().unwrap(), &expected, &machine);
    assert!(finding.is_none(), "matching content should show no drift");
}

#[test]
fn test_fj016_check_file_drift_transport_drift() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("transport-drift.txt");
    std::fs::write(&file, "original").unwrap();

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
    };

    // Expected hash of different content
    let finding =
        check_file_drift_via_transport("f", file.to_str().unwrap(), "blake3:wrong-hash", &machine);
    assert!(finding.is_some(), "mismatched hash should detect drift");
    let f = finding.unwrap();
    assert!(f.detail.contains("content changed"));
}

#[test]
fn test_fj016_check_file_drift_transport_missing_file() {
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
    };

    let finding = check_file_drift_via_transport(
        "missing",
        "/nonexistent/file/forjar-test.txt",
        "blake3:abc",
        &machine,
    );
    assert!(finding.is_some());
    let f = finding.unwrap();
    assert_eq!(f.actual_hash, "MISSING");
}

#[test]
fn test_fj016_detect_drift_multiple_resources_mixed() {
    // Lock with 3 resources: file (no drift), file (drifted), package (skipped)
    let dir = tempfile::tempdir().unwrap();
    let good_file = dir.path().join("good.txt");
    let bad_file = dir.path().join("bad.txt");
    std::fs::write(&good_file, "good").unwrap();
    std::fs::write(&bad_file, "changed").unwrap();

    let good_hash = hasher::hash_file(&good_file).unwrap();

    let mut resources = indexmap::IndexMap::new();
    let mut good_details = std::collections::HashMap::new();
    good_details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(good_file.to_str().unwrap().to_string()),
    );
    good_details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String(good_hash),
    );
    resources.insert(
        "good-file".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "desired".to_string(),
            details: good_details,
        },
    );

    let mut bad_details = std::collections::HashMap::new();
    bad_details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(bad_file.to_str().unwrap().to_string()),
    );
    bad_details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:stale-hash".to_string()),
    );
    resources.insert(
        "bad-file".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "desired".to_string(),
            details: bad_details,
        },
    );

    // Package resource should be skipped by detect_drift
    resources.insert(
        "pkg".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "desired".to_string(),
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

    let findings = detect_drift(&lock);
    assert_eq!(findings.len(), 1, "only the drifted file should appear");
    assert_eq!(findings[0].resource_id, "bad-file");
}

#[test]
fn test_fj016_detect_drift_failed_resource_skipped() {
    // Failed resources should not be drift-checked
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String("/nonexistent".to_string()),
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
            hash: "".to_string(),
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
fn test_fj016_check_file_drift_empty_file() {
    // Empty file should still have a valid hash
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("empty.txt");
    std::fs::write(&file, "").unwrap();
    let hash = hasher::hash_file(&file).unwrap();
    let finding = check_file_drift("empty", file.to_str().unwrap(), &hash);
    assert!(finding.is_none(), "empty file with correct hash = no drift");
}

#[test]
fn test_fj016_check_file_drift_wrong_hash_format() {
    // Even a non-blake3 hash should trigger drift if it doesn't match
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "test").unwrap();
    let finding = check_file_drift("f", file.to_str().unwrap(), "sha256:wrong");
    assert!(finding.is_some(), "wrong hash format should show as drift");
}

#[test]
fn test_fj016_check_file_drift_transport_directory() {
    // Transport drift check on a directory
    let dir = tempfile::tempdir().unwrap();
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
    };
    // Using a directory path should work via transport (ls -la)
    let finding = check_file_drift_via_transport(
        "d",
        dir.path().to_str().unwrap(),
        "blake3:definitely-wrong",
        &machine,
    );
    assert!(finding.is_some(), "directory hash should differ from dummy");
}
