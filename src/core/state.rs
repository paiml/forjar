//! FJ-013: Lock file management — load, save (atomic), path derivation.

use super::types::StateLock;
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
    let lock: StateLock = serde_yaml::from_str(&content)
        .map_err(|e| format!("invalid lock file {}: {}", path.display(), e))?;
    Ok(Some(lock))
}

/// Save a lock file atomically (write to temp, then rename).
pub fn save_lock(state_dir: &Path, lock: &StateLock) -> Result<(), String> {
    let path = lock_file_path(state_dir, &lock.machine);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create dir {}: {}", parent.display(), e))?;
    }

    let yaml = serde_yaml::to_string(lock)
        .map_err(|e| format!("serialize error: {}", e))?;

    // Atomic write: temp file + rename
    let tmp_path = path.with_extension("lock.yaml.tmp");
    std::fs::write(&tmp_path, &yaml)
        .map_err(|e| format!("cannot write {}: {}", tmp_path.display(), e))?;
    std::fs::rename(&tmp_path, &path)
        .map_err(|e| format!("cannot rename {} → {}: {}", tmp_path.display(), path.display(), e))?;

    Ok(())
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
        let tmp = dir
            .path()
            .join("test")
            .join("state.lock.yaml.tmp");
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
}
