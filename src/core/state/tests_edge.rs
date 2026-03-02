use super::tests_helpers::make_lock;
use super::*;
use crate::core::types::{ResourceLock, ResourceStatus, ResourceType};
use std::collections::HashMap;
use std::path::Path;

// ── FJ-131: State edge case tests ─────────────────────────────

#[test]
fn test_fj131_update_global_lock_overwrites_machine_stats() {
    // Updating a machine should overwrite its stats completely
    let dir = tempfile::tempdir().unwrap();
    let results1 = vec![("web".to_string(), 10_usize, 8_usize, 2_usize)];
    update_global_lock(dir.path(), "infra", &results1).unwrap();

    let results2 = vec![("web".to_string(), 10_usize, 10_usize, 0_usize)];
    update_global_lock(dir.path(), "infra", &results2).unwrap();

    let loaded = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.machines["web"].converged, 10);
    assert_eq!(loaded.machines["web"].failed, 0);
}

#[test]
fn test_fj131_new_lock_generator_format() {
    let lock = new_lock("m", "h");
    // Generator should contain "forjar" and a version
    assert!(lock.generator.starts_with("forjar "));
    // Version is from CARGO_PKG_VERSION
    let version_part = lock.generator.strip_prefix("forjar ").unwrap();
    assert!(
        !version_part.is_empty(),
        "should have version after 'forjar '"
    );
}

#[test]
fn test_fj131_new_global_lock_generator_format() {
    let lock = new_global_lock("test");
    assert!(lock.generator.starts_with("forjar "));
    assert_eq!(lock.schema, "1.0");
}

#[test]
fn test_fj131_save_lock_deep_state_dir() {
    // Save to a deeply nested state directory that doesn't exist
    let dir = tempfile::tempdir().unwrap();
    let deep = dir.path().join("a").join("b").join("c");
    let lock = make_lock();
    // save_lock creates the machine subdirectory, but the parent must exist
    // Let's create the deep path first then save into it
    std::fs::create_dir_all(&deep).unwrap();
    save_lock(&deep, &lock).unwrap();

    let loaded = load_lock(&deep, "test").unwrap().unwrap();
    assert_eq!(loaded.machine, "test");
}

#[test]
fn test_fj131_update_global_lock_changes_name() {
    // Calling update_global_lock with a different name should update it
    let dir = tempfile::tempdir().unwrap();
    let results = vec![("web".to_string(), 3_usize, 3_usize, 0_usize)];
    update_global_lock(dir.path(), "old-name", &results).unwrap();

    let results2: Vec<(String, usize, usize, usize)> = vec![];
    update_global_lock(dir.path(), "new-name", &results2).unwrap();

    let loaded = load_global_lock(dir.path()).unwrap().unwrap();
    assert_eq!(loaded.name, "new-name");
    // Previous machine data should still be present
    assert!(loaded.machines.contains_key("web"));
}

#[test]
fn test_fj131_lock_roundtrip_preserves_all_status_types() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = make_lock();
    lock.resources.insert(
        "drifted-res".to_string(),
        ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Drifted,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:xxx".to_string(),
            details: HashMap::new(),
        },
    );
    lock.resources.insert(
        "unknown-res".to_string(),
        ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Unknown,
            applied_at: None,
            duration_seconds: None,
            hash: "".to_string(),
            details: HashMap::new(),
        },
    );
    save_lock(dir.path(), &lock).unwrap();

    let loaded = load_lock(dir.path(), "test").unwrap().unwrap();
    assert_eq!(
        loaded.resources["drifted-res"].status,
        ResourceStatus::Drifted
    );
    assert_eq!(
        loaded.resources["unknown-res"].status,
        ResourceStatus::Unknown
    );
    assert_eq!(
        loaded.resources["test-pkg"].status,
        ResourceStatus::Converged
    );
}

// --- FJ-132: State edge case tests ---

#[test]
fn test_fj132_save_lock_with_duration() {
    // Verify duration_seconds field persists through roundtrip
    let dir = tempfile::tempdir().unwrap();
    let mut lock = new_lock("m1", "host1");
    lock.resources.insert(
        "timed-res".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-01-01T00:00:00Z".to_string()),
            duration_seconds: Some(1.234),
            hash: "blake3:abc123".to_string(),
            details: HashMap::new(),
        },
    );
    save_lock(dir.path(), &lock).unwrap();
    let loaded = load_lock(dir.path(), "m1").unwrap().unwrap();
    assert_eq!(loaded.resources["timed-res"].duration_seconds, Some(1.234));
}

#[test]
fn test_fj132_save_lock_unicode_hostname() {
    // Machine names and hostnames may contain unusual characters
    let dir = tempfile::tempdir().unwrap();
    let lock = new_lock("edge-node-01", "räck-ünit");
    save_lock(dir.path(), &lock).unwrap();
    let loaded = load_lock(dir.path(), "edge-node-01").unwrap().unwrap();
    assert_eq!(loaded.hostname, "räck-ünit");
}

#[test]
fn test_fj132_lock_file_path_consistency() {
    // Verify path derivation is consistent
    let dir = Path::new("/tmp/forjar-state");
    let p1 = lock_file_path(dir, "web");
    let p2 = lock_file_path(dir, "web");
    assert_eq!(p1, p2);
    assert!(p1.ends_with("web/state.lock.yaml"));
}

#[test]
fn test_fj132_update_global_lock_preserves_existing_machines() {
    // Updating with new machines should keep old ones
    let dir = tempfile::tempdir().unwrap();
    let results1 = vec![("web".to_string(), 5usize, 5usize, 0usize)];
    update_global_lock(dir.path(), "test-config", &results1).unwrap();

    let results2 = vec![("db".to_string(), 3usize, 2usize, 1usize)];
    update_global_lock(dir.path(), "test-config", &results2).unwrap();

    let lock = load_global_lock(dir.path()).unwrap().unwrap();
    assert!(lock.machines.contains_key("web"), "web should be preserved");
    assert!(lock.machines.contains_key("db"), "db should be added");
    assert_eq!(lock.machines["web"].converged, 5);
    assert_eq!(lock.machines["db"].failed, 1);
}

#[test]
fn test_fj132_new_lock_fields_populated() {
    let lock = new_lock("prod-1", "prod-host");
    assert_eq!(lock.schema, "1.0");
    assert_eq!(lock.machine, "prod-1");
    assert_eq!(lock.hostname, "prod-host");
    assert_eq!(lock.blake3_version, "1.8");
    assert!(lock.generator.starts_with("forjar "));
    assert!(!lock.generated_at.is_empty());
    assert!(lock.resources.is_empty());
}

#[test]
fn test_fj132_global_lock_schema_version() {
    let lock = new_global_lock("my-infra");
    assert_eq!(lock.schema, "1.0");
    assert_eq!(lock.name, "my-infra");
    assert!(lock.machines.is_empty());
}

#[test]
fn test_fj132_save_lock_with_many_details() {
    // Resources can have arbitrary string->serde_yaml::Value details
    let dir = tempfile::tempdir().unwrap();
    let mut details = HashMap::new();
    details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String("/etc/app.conf".to_string()),
    );
    details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:deadbeef".to_string()),
    );
    details.insert(
        "owner".to_string(),
        serde_yaml_ng::Value::String("root".to_string()),
    );
    details.insert(
        "mode".to_string(),
        serde_yaml_ng::Value::String("0644".to_string()),
    );

    let mut lock = new_lock("m1", "h1");
    lock.resources.insert(
        "config".to_string(),
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:abc".to_string(),
            details,
        },
    );
    save_lock(dir.path(), &lock).unwrap();
    let loaded = load_lock(dir.path(), "m1").unwrap().unwrap();
    let d = &loaded.resources["config"].details;
    assert_eq!(d.len(), 4);
    assert_eq!(
        d["path"],
        serde_yaml_ng::Value::String("/etc/app.conf".to_string())
    );
}

#[test]
fn test_fj132_concurrent_save_different_machines() {
    // Two independent machine locks can be saved without conflict
    let dir = tempfile::tempdir().unwrap();
    let lock_a = new_lock("machine-a", "host-a");
    let lock_b = new_lock("machine-b", "host-b");
    save_lock(dir.path(), &lock_a).unwrap();
    save_lock(dir.path(), &lock_b).unwrap();

    let loaded_a = load_lock(dir.path(), "machine-a").unwrap().unwrap();
    let loaded_b = load_lock(dir.path(), "machine-b").unwrap().unwrap();
    assert_eq!(loaded_a.hostname, "host-a");
    assert_eq!(loaded_b.hostname, "host-b");
}

// --- FJ-036: State roundtrip and path construction tests ---

#[test]
fn test_fj036_save_and_load_lock_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let mut lock = make_lock();
    lock.machine = "roundtrip-box".to_string();
    lock.hostname = "rt-host".to_string();
    lock.schema = "1.0".to_string();
    lock.blake3_version = "1.8".to_string();
    lock.resources.insert(
        "extra-svc".to_string(),
        ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Failed,
            applied_at: Some("2026-02-25T12:00:00Z".to_string()),
            duration_seconds: Some(2.75),
            hash: "blake3:roundtrip".to_string(),
            details: HashMap::new(),
        },
    );
    save_lock(dir.path(), &lock).unwrap();

    let loaded = load_lock(dir.path(), "roundtrip-box").unwrap().unwrap();
    assert_eq!(loaded.machine, "roundtrip-box");
    assert_eq!(loaded.hostname, "rt-host");
    assert_eq!(loaded.schema, "1.0");
    assert_eq!(loaded.blake3_version, "1.8");
    assert_eq!(loaded.resources.len(), lock.resources.len());
    assert_eq!(loaded.resources["extra-svc"].status, ResourceStatus::Failed);
    assert_eq!(loaded.resources["extra-svc"].duration_seconds, Some(2.75));
    assert_eq!(loaded.resources["extra-svc"].hash, "blake3:roundtrip");
}

#[test]
fn test_fj036_load_lock_nonexistent_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let result = load_lock(dir.path(), "no-such-machine").unwrap();
    assert!(
        result.is_none(),
        "loading a lock for a nonexistent machine must return None"
    );
}

#[test]
fn test_fj036_lock_path_construction() {
    let state_dir = Path::new("/var/lib/forjar/state");
    let machine = "web-prod";
    let p = lock_file_path(state_dir, machine);
    assert_eq!(
        p,
        std::path::PathBuf::from("/var/lib/forjar/state/web-prod/state.lock.yaml"),
        "lock path must be state_dir/machine/state.lock.yaml"
    );
}

#[test]
fn test_fj036_save_lock_creates_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let deep_state = dir.path().join("nonexistent").join("deep").join("state");
    // The deep_state directory does not exist yet
    assert!(!deep_state.exists());

    let lock = make_lock();
    save_lock(&deep_state, &lock).unwrap();

    // save_lock should have created the machine subdirectory inside deep_state
    let expected_dir = deep_state.join("test");
    assert!(
        expected_dir.exists(),
        "save_lock must create the state directory hierarchy"
    );
    let expected_file = lock_file_path(&deep_state, "test");
    assert!(
        expected_file.exists(),
        "lock file must exist after save_lock"
    );
}
