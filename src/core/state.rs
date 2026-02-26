//! FJ-013: Lock file management — load, save (atomic), path derivation.

use super::types::{ApplyResult, GlobalLock, MachineSummary, StateLock};
use provable_contracts_macros::contract;
use std::path::{Path, PathBuf};

/// Derive the lock file path for a machine within the state directory.
pub fn lock_file_path(state_dir: &Path, machine: &str) -> PathBuf {
    state_dir.join(machine).join("state.lock.yaml")
}

/// Load a lock file for a machine. Returns None if the file doesn't exist.
pub fn load_lock(state_dir: &Path, machine: &str) -> Result<Option<StateLock>, String> {
    let path = lock_file_path(state_dir, machine);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let lock: StateLock = serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("invalid lock file {}: {}", path.display(), e))?;
    Ok(Some(lock))
}

/// Save a lock file atomically (write to temp, then rename).
#[contract("execution-safety-v1", equation = "atomic_write")]
pub fn save_lock(state_dir: &Path, lock: &StateLock) -> Result<(), String> {
    let path = lock_file_path(state_dir, &lock.machine);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create dir {}: {}", parent.display(), e))?;
    }

    let yaml = serde_yaml_ng::to_string(lock).map_err(|e| format!("serialize error: {}", e))?;

    // Atomic write: temp file + rename
    let tmp_path = path.with_extension("lock.yaml.tmp");
    std::fs::write(&tmp_path, &yaml)
        .map_err(|e| format!("cannot write {}: {}", tmp_path.display(), e))?;
    std::fs::rename(&tmp_path, &path).map_err(|e| {
        format!(
            "cannot rename {} → {}: {}",
            tmp_path.display(),
            path.display(),
            e
        )
    })?;

    Ok(())
}

/// Path to the global lock file.
pub fn global_lock_path(state_dir: &Path) -> PathBuf {
    state_dir.join("forjar.lock.yaml")
}

/// Load the global lock file. Returns None if it doesn't exist.
pub fn load_global_lock(state_dir: &Path) -> Result<Option<GlobalLock>, String> {
    let path = global_lock_path(state_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let lock: GlobalLock = serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("invalid global lock {}: {}", path.display(), e))?;
    Ok(Some(lock))
}

/// Save the global lock file atomically.
pub fn save_global_lock(state_dir: &Path, lock: &GlobalLock) -> Result<(), String> {
    std::fs::create_dir_all(state_dir)
        .map_err(|e| format!("cannot create dir {}: {}", state_dir.display(), e))?;

    let path = global_lock_path(state_dir);
    let yaml = serde_yaml_ng::to_string(lock).map_err(|e| format!("serialize error: {}", e))?;

    let tmp_path = path.with_extension("lock.yaml.tmp");
    std::fs::write(&tmp_path, &yaml)
        .map_err(|e| format!("cannot write {}: {}", tmp_path.display(), e))?;
    std::fs::rename(&tmp_path, &path).map_err(|e| {
        format!(
            "cannot rename {} → {}: {}",
            tmp_path.display(),
            path.display(),
            e
        )
    })?;

    Ok(())
}

/// Create a new GlobalLock with machine summaries.
pub fn new_global_lock(name: &str) -> GlobalLock {
    use crate::tripwire::eventlog::now_iso8601;
    GlobalLock {
        schema: "1.0".to_string(),
        name: name.to_string(),
        last_apply: now_iso8601(),
        generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        machines: indexmap::IndexMap::new(),
    }
}

/// Update global lock with results from an apply.
pub fn update_global_lock(
    state_dir: &Path,
    config_name: &str,
    machine_results: &[(String, usize, usize, usize)], // (name, total, converged, failed)
) -> Result<(), String> {
    use crate::tripwire::eventlog::now_iso8601;
    let mut lock = load_global_lock(state_dir)?.unwrap_or_else(|| new_global_lock(config_name));
    lock.name = config_name.to_string();
    lock.last_apply = now_iso8601();
    lock.generator = format!("forjar {}", env!("CARGO_PKG_VERSION"));

    for (name, total, converged, failed) in machine_results {
        lock.machines.insert(
            name.clone(),
            MachineSummary {
                resources: *total,
                converged: *converged,
                failed: *failed,
                last_apply: now_iso8601(),
            },
        );
    }

    save_global_lock(state_dir, &lock)
}

/// Create a new empty StateLock for a machine.
pub fn new_lock(machine: &str, hostname: &str) -> StateLock {
    use crate::tripwire::eventlog::now_iso8601;
    StateLock {
        schema: "1.0".to_string(),
        machine: machine.to_string(),
        hostname: hostname.to_string(),
        generated_at: now_iso8601(),
        generator: format!("forjar {}", env!("CARGO_PKG_VERSION")),
        blake3_version: "1.8".to_string(),
        resources: indexmap::IndexMap::new(),
    }
}

/// FJ-262: Save per-machine apply report to `state/<machine>/last-apply.yaml`.
pub fn save_apply_report(state_dir: &Path, result: &ApplyResult) -> Result<(), String> {
    let dir = state_dir.join(&result.machine);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("cannot create dir {}: {}", dir.display(), e))?;
    let path = dir.join("last-apply.yaml");
    let yaml =
        serde_yaml_ng::to_string(result).map_err(|e| format!("serialize report error: {}", e))?;
    std::fs::write(&path, &yaml)
        .map_err(|e| format!("cannot write {}: {}", path.display(), e))?;
    Ok(())
}

/// FJ-262: Load last apply report for a machine.
pub fn load_apply_report(state_dir: &Path, machine: &str) -> Result<Option<String>, String> {
    let path = state_dir.join(machine).join("last-apply.yaml");
    if !path.exists() {
        return Ok(None);
    }
    std::fs::read_to_string(&path)
        .map(Some)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{ResourceLock, ResourceStatus, ResourceType};
    use proptest::prelude::*;
    use std::collections::HashMap;

    fn make_lock() -> StateLock {
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "test-pkg".to_string(),
            ResourceLock {
                resource_type: ResourceType::Package,
                status: ResourceStatus::Converged,
                applied_at: Some("2026-02-16T14:00:00Z".to_string()),
                duration_seconds: Some(1.5),
                hash: "blake3:abc123".to_string(),
                details: HashMap::new(),
            },
        );
        StateLock {
            schema: "1.0".to_string(),
            machine: "test".to_string(),
            hostname: "test-box".to_string(),
            generated_at: "2026-02-16T14:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        }
    }

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
    fn test_fj013_global_lock_corrupted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("forjar.lock.yaml"), "{{broken yaml").unwrap();
        let result = load_global_lock(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid global lock"));
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
        // Resources can have arbitrary string→serde_yaml::Value details
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
            PathBuf::from("/var/lib/forjar/state/web-prod/state.lock.yaml"),
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
}
