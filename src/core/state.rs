//! FJ-013: Lock file management — load, save (atomic), path derivation.

use super::types::{GlobalLock, MachineSummary, StateLock};
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
    let mut lock = load_global_lock(state_dir)?
        .unwrap_or_else(|| new_global_lock(config_name));
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
}
