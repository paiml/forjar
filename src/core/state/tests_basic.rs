use super::*;
use super::tests_helpers::make_lock;
use crate::core::types::{ResourceLock, ResourceStatus, ResourceType};
use proptest::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[test]
fn test_fj013_lock_file_path() {
    let p = lock_file_path(Path::new("/state"), "lambda");
    assert_eq!(p, PathBuf::from("/state/lambda/state.lock.yaml"));
}

#[test]
fn test_fj013_save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let lock = make_lock();
    save_lock(dir.path(), &lock).unwrap();

    let loaded = load_lock(dir.path(), "test").unwrap().unwrap();
    assert_eq!(loaded.machine, "test");
    assert_eq!(loaded.resources.len(), 1);
    assert_eq!(
        loaded.resources["test-pkg"].status,
        ResourceStatus::Converged
    );
}

#[test]
fn test_fj013_load_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    let result = load_lock(dir.path(), "ghost").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_fj013_atomic_write() {
    let dir = tempfile::tempdir().unwrap();
    let lock = make_lock();
    save_lock(dir.path(), &lock).unwrap();

    // Verify temp file is cleaned up
    let tmp = dir.path().join("test").join("state.lock.yaml.tmp");
    assert!(!tmp.exists());

    // Verify actual file exists
    let actual = lock_file_path(dir.path(), "test");
    assert!(actual.exists());
}

#[test]
fn test_fj013_new_lock() {
    let lock = new_lock("lambda", "lambda-box");
    assert_eq!(lock.machine, "lambda");
    assert_eq!(lock.hostname, "lambda-box");
    assert!(lock.generated_at.contains('T'));
    assert!(lock.resources.is_empty());
}

#[test]
fn test_fj013_load_corrupted_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let machine_dir = dir.path().join("broken");
    std::fs::create_dir_all(&machine_dir).unwrap();
    std::fs::write(machine_dir.join("state.lock.yaml"), "not: [valid: yaml: {{").unwrap();
    let result = load_lock(dir.path(), "broken");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid lock file"));
}

#[test]
fn test_fj013_save_lock_creates_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let lock = make_lock();
    // state_dir/test/ doesn't exist yet; save_lock should create it
    save_lock(dir.path(), &lock).unwrap();
    assert!(dir.path().join("test").exists());
    assert!(lock_file_path(dir.path(), "test").exists());
}

#[test]
fn test_fj013_roundtrip_preserves_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = make_lock();
    lock.resources.insert(
        "aaa-first".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:xxx".to_string(),
            details: HashMap::new(),
        },
    );
    save_lock(dir.path(), &lock).unwrap();
    let loaded = load_lock(dir.path(), "test").unwrap().unwrap();
    let keys: Vec<_> = loaded.resources.keys().collect();
    assert_eq!(keys, vec!["test-pkg", "aaa-first"]);
}

// ── Falsification tests (Execution Safety Contract) ─────────

proptest! {
    /// FALSIFY-ES-001: Atomic write leaves no temp file after success.
    #[test]
    fn falsify_es_001_atomic_write_no_temp(machine in "[a-z]{1,8}", hostname in "[a-z]{1,12}") {
        let dir = tempfile::tempdir().unwrap();
        let lock = StateLock {
            schema: "1.0".to_string(),
            machine: machine.clone(),
            hostname,
            generated_at: "2026-02-24T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources: indexmap::IndexMap::new(),
        };
        save_lock(dir.path(), &lock).unwrap();

        // Temp file must not exist
        let tmp = dir.path().join(&machine).join("state.lock.yaml.tmp");
        prop_assert!(!tmp.exists(), "temp file must not remain after save_lock");

        // Actual file must exist
        let actual = lock_file_path(dir.path(), &machine);
        prop_assert!(actual.exists(), "lock file must exist after save_lock");
    }
}

#[test]
fn test_fj013_global_lock_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = new_global_lock("test-infra");
    lock.machines.insert(
        "lambda".to_string(),
        MachineSummary {
            resources: 5,
            converged: 4,
            failed: 1,
            last_apply: "2026-02-24T00:00:00Z".to_string(),
        },
    );
    save_global_lock(dir.path(), &lock).unwrap();

    let loaded = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.name, "test-infra");
    assert_eq!(loaded.schema, "1.0");
    assert_eq!(loaded.machines.len(), 1);
    assert_eq!(loaded.machines["lambda"].resources, 5);
    assert_eq!(loaded.machines["lambda"].converged, 4);
    assert_eq!(loaded.machines["lambda"].failed, 1);
}

#[test]
fn test_fj013_global_lock_missing() {
    let dir = tempfile::tempdir().unwrap();
    let result = load_global_lock(dir.path()).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_fj013_update_global_lock() {
    let dir = tempfile::tempdir().unwrap();
    let results = vec![
        ("web".to_string(), 3_usize, 2_usize, 0_usize),
        ("db".to_string(), 5, 5, 0),
    ];
    update_global_lock(dir.path(), "my-infra", &results).unwrap();

    let loaded = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.name, "my-infra");
    assert_eq!(loaded.machines.len(), 2);
    assert_eq!(loaded.machines["web"].resources, 3);
    assert_eq!(loaded.machines["web"].converged, 2);
    assert_eq!(loaded.machines["db"].resources, 5);
    assert_eq!(loaded.machines["db"].converged, 5);
}

#[test]
fn test_fj013_global_lock_path() {
    let p = global_lock_path(Path::new("/state"));
    assert_eq!(p, PathBuf::from("/state/forjar.lock.yaml"));
}

#[test]
fn test_fj013_update_global_lock_idempotent() {
    // Calling update_global_lock twice should overwrite, not duplicate
    let dir = tempfile::tempdir().unwrap();
    let results1 = vec![("web".to_string(), 3_usize, 2_usize, 1_usize)];
    update_global_lock(dir.path(), "infra", &results1).unwrap();

    let results2 = vec![("web".to_string(), 3_usize, 3_usize, 0_usize)];
    update_global_lock(dir.path(), "infra", &results2).unwrap();

    let loaded = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.machines.len(), 1);
    assert_eq!(loaded.machines["web"].converged, 3);
    assert_eq!(loaded.machines["web"].failed, 0);
}

#[test]
fn test_fj013_update_global_lock_adds_new_machines() {
    let dir = tempfile::tempdir().unwrap();
    let results1 = vec![("web".to_string(), 3_usize, 3_usize, 0_usize)];
    update_global_lock(dir.path(), "infra", &results1).unwrap();

    let results2 = vec![("db".to_string(), 5_usize, 5_usize, 0_usize)];
    update_global_lock(dir.path(), "infra", &results2).unwrap();

    let loaded = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.machines.len(), 2);
    assert!(loaded.machines.contains_key("web"));
    assert!(loaded.machines.contains_key("db"));
}

#[test]
fn test_fj013_save_lock_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let lock1 = make_lock();
    save_lock(dir.path(), &lock1).unwrap();

    let mut lock2 = make_lock();
    lock2.resources.clear();
    save_lock(dir.path(), &lock2).unwrap();

    let loaded = load_lock(dir.path(), "test").unwrap().unwrap();
    assert!(
        loaded.resources.is_empty(),
        "overwrite should replace all resources"
    );
}

#[test]
fn test_fj013_new_lock_version() {
    let lock = new_lock("m", "h");
    assert_eq!(lock.schema, "1.0");
    assert_eq!(lock.blake3_version, "1.8");
    assert!(lock.generator.starts_with("forjar "));
}

#[test]
fn test_fj013_save_lock_multiple_resources() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = make_lock();
    lock.resources.insert(
        "conf-file".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-02-25T10:00:00Z".to_string()),
            duration_seconds: Some(0.1),
            hash: "blake3:def456".to_string(),
            details: HashMap::new(),
        },
    );
    lock.resources.insert(
        "web-svc".to_string(),
        ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Failed,
            applied_at: Some("2026-02-25T10:00:01Z".to_string()),
            duration_seconds: Some(5.0),
            hash: "blake3:ghi789".to_string(),
            details: HashMap::new(),
        },
    );
    save_lock(dir.path(), &lock).unwrap();

    let loaded = load_lock(dir.path(), "test").unwrap().unwrap();
    assert_eq!(loaded.resources.len(), 3);
    assert_eq!(
        loaded.resources["conf-file"].status,
        ResourceStatus::Converged
    );
    assert_eq!(loaded.resources["web-svc"].status, ResourceStatus::Failed);
}

#[test]
fn test_fj013_lock_file_path_special_chars() {
    let p = lock_file_path(Path::new("/var/lib/forjar/state"), "web-server-01");
    assert_eq!(
        p,
        PathBuf::from("/var/lib/forjar/state/web-server-01/state.lock.yaml")
    );
}

#[test]
fn test_fj013_new_global_lock_empty_machines() {
    let lock = new_global_lock("my-infra");
    assert_eq!(lock.schema, "1.0");
    assert_eq!(lock.name, "my-infra");
    assert!(lock.machines.is_empty());
    assert!(lock.last_apply.contains('T'));
    assert!(lock.generator.starts_with("forjar "));
}

#[test]
fn test_fj013_update_global_lock_empty_results() {
    let dir = tempfile::tempdir().unwrap();
    let results: Vec<(String, usize, usize, usize)> = vec![];
    update_global_lock(dir.path(), "infra", &results).unwrap();

    let loaded = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.name, "infra");
    assert!(loaded.machines.is_empty());
}

#[test]
fn test_fj013_save_load_lock_with_details() {
    let dir = tempfile::tempdir().unwrap();
    let mut details = HashMap::new();
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:aaa".to_string()),
    );
    details.insert(
        "service_name".to_string(),
        serde_yaml_ng::Value::String("nginx".to_string()),
    );

    let mut lock = make_lock();
    lock.resources.get_mut("test-pkg").unwrap().details = details;
    save_lock(dir.path(), &lock).unwrap();

    let loaded = load_lock(dir.path(), "test").unwrap().unwrap();
    assert_eq!(
        loaded.resources["test-pkg"].details["content_hash"],
        serde_yaml_ng::Value::String("blake3:aaa".to_string())
    );
    assert_eq!(
        loaded.resources["test-pkg"].details["service_name"],
        serde_yaml_ng::Value::String("nginx".to_string())
    );
}

#[test]
fn test_fj013_global_lock_atomic_no_temp() {
    let dir = tempfile::tempdir().unwrap();
    let lock = new_global_lock("test");
    save_global_lock(dir.path(), &lock).unwrap();

    let tmp = dir.path().join("forjar.lock.yaml.tmp");
    assert!(!tmp.exists(), "temp file must be cleaned up");
    assert!(global_lock_path(dir.path()).exists());
}

#[test]
fn test_fj013_global_lock_corrupted() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("forjar.lock.yaml"), "{{broken yaml").unwrap();
    let result = load_global_lock(dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid global lock"));
}
