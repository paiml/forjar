#![allow(unused_imports)]
use super::*;
use crate::core::types::{MachineTarget, Resource};
use crate::tripwire::hasher;

#[test]
fn test_fj016_check_file_drift_directory() {
    let dir = tempfile::tempdir().unwrap();
    let subdir = dir.path().join("mydir");
    std::fs::create_dir(&subdir).unwrap();
    let hash = hasher::hash_directory(&subdir).unwrap();

    // No drift when hash matches
    let result = check_file_drift("dir-resource", subdir.to_str().unwrap(), &hash);
    assert!(result.is_none());

    // Create a file inside — hash changes
    std::fs::write(subdir.join("new.txt"), "surprise").unwrap();
    let result = check_file_drift("dir-resource", subdir.to_str().unwrap(), &hash);
    assert!(result.is_some());
}

#[test]
fn test_fj016_drift_finding_fields() {
    let finding = DriftFinding {
        resource_id: "nginx-config".to_string(),
        resource_type: ResourceType::File,
        expected_hash: "blake3:aaa".to_string(),
        actual_hash: "blake3:bbb".to_string(),
        detail: "content changed".to_string(),
    };
    assert_eq!(finding.resource_id, "nginx-config");
    assert_eq!(finding.resource_type, ResourceType::File);
    assert_ne!(finding.expected_hash, finding.actual_hash);
}

#[test]
fn test_fj016_missing_file_detail_message() {
    let result = check_file_drift("my-conf", "/does/not/exist/at/all.conf", "blake3:abc");
    let finding = result.unwrap();
    assert_eq!(finding.actual_hash, "MISSING");
    assert!(
        finding.detail.contains("does not exist"),
        "detail should say file does not exist: {}",
        finding.detail
    );
}

#[test]
fn test_fj016_content_drift_detail_message() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("x.txt");
    std::fs::write(&file, "original").unwrap();
    let result = check_file_drift("x", file.to_str().unwrap(), "blake3:wrong");
    let finding = result.unwrap();
    assert!(
        finding.detail.contains("content changed"),
        "detail should mention content changed: {}",
        finding.detail
    );
}

#[test]
fn test_fj016_detect_drift_with_machine_local() {
    // detect_drift_with_machine on local machine uses local hash path
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
            hash: "blake3:x".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "local".to_string(),
        hostname: "local".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
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
    };

    let findings = detect_drift_with_machine(&lock, &machine);
    assert!(findings.is_empty(), "no drift expected for matching file");
}

#[test]
fn test_fj016_detect_drift_with_machine_local_drift() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("changed.txt");
    std::fs::write(&file, "before").unwrap();

    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:stale".to_string()),
    );
    resources.insert(
        "changed-file".to_string(),
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
        machine: "local".to_string(),
        hostname: "local".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
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
    };

    let findings = detect_drift_with_machine(&lock, &machine);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].resource_id, "changed-file");
}

#[test]
fn test_fj016_detect_drift_multiple_files() {
    let dir = tempfile::tempdir().unwrap();
    let file1 = dir.path().join("a.txt");
    let file2 = dir.path().join("b.txt");
    std::fs::write(&file1, "aaa").unwrap();
    std::fs::write(&file2, "bbb").unwrap();
    let hash1 = hasher::hash_file(&file1).unwrap();

    let mut resources = indexmap::IndexMap::new();
    for (id, path, hash) in [
        ("file-a", file1.to_str().unwrap(), hash1.as_str()),
        ("file-b", file2.to_str().unwrap(), "blake3:wrong"),
    ] {
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(path.to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(hash.to_string()),
        );
        resources.insert(
            id.to_string(),
            crate::core::types::ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "blake3:x".to_string(),
                details,
            },
        );
    }

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };

    let findings = detect_drift(&lock);
    // file-a matches, file-b drifted
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].resource_id, "file-b");
}

#[test]
fn test_fj016_directory_drift_new_file_inside() {
    let dir = tempfile::tempdir().unwrap();
    let subdir = dir.path().join("watched");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("original.txt"), "content").unwrap();
    let hash = hasher::hash_directory(&subdir).unwrap();

    // No drift initially
    assert!(check_file_drift("dir", subdir.to_str().unwrap(), &hash).is_none());

    // Add a new file — drift detected
    std::fs::write(subdir.join("intruder.txt"), "surprise").unwrap();
    let finding = check_file_drift("dir", subdir.to_str().unwrap(), &hash).unwrap();
    assert_eq!(finding.resource_id, "dir");
    assert!(finding.detail.contains("content changed"));
}

#[test]
fn test_fj016_missing_content_hash_skipped() {
    // File resource with path but no content_hash should be skipped
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("no-hash.txt");
    std::fs::write(&file, "data").unwrap();

    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    // no content_hash key
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
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };

    let findings = detect_drift(&lock);
    assert!(
        findings.is_empty(),
        "missing content_hash should skip drift check"
    );
}

#[test]
fn test_fj016_full_drift_non_string_live_hash_skipped() {
    // Non-file resource with non-string live_hash should be skipped
    let mut resources = indexmap::IndexMap::new();
    let mut details = std::collections::HashMap::new();
    details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(999)),
    );
    resources.insert(
        "bad-live".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:abc".to_string(),
            details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    };
    let machine = Machine {
        hostname: "test".to_string(),
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
    let config_resources = indexmap::IndexMap::new();
    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert!(
        findings.is_empty(),
        "non-string live_hash should be skipped"
    );
}

// ── Additional edge case tests ─────────────────────────────────

#[test]
fn test_fj016_drift_finding_debug_and_clone() {
    let f = DriftFinding {
        resource_id: "test".to_string(),
        resource_type: ResourceType::File,
        expected_hash: "blake3:aaa".to_string(),
        actual_hash: "blake3:bbb".to_string(),
        detail: "changed".to_string(),
    };
    // Debug
    let dbg = format!("{:?}", f);
    assert!(dbg.contains("test"));
    // Clone
    let c = f.clone();
    assert_eq!(c.resource_id, "test");
    assert_eq!(c.expected_hash, "blake3:aaa");
}
