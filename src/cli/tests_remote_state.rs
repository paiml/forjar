//! Tests: FJ-1425 remote state backend.

#![allow(unused_imports)]
use super::remote_state::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_backend_put_get() {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(dir.path());
        backend.put("test/key.txt", b"hello").unwrap();
        let data = backend.get("test/key.txt").unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn test_local_backend_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(dir.path());
        assert!(!backend.exists("missing").unwrap());
        backend.put("present", b"data").unwrap();
        assert!(backend.exists("present").unwrap());
    }

    #[test]
    fn test_local_backend_delete() {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(dir.path());
        backend.put("del.txt", b"data").unwrap();
        assert!(backend.exists("del.txt").unwrap());
        backend.delete("del.txt").unwrap();
        assert!(!backend.exists("del.txt").unwrap());
    }

    #[test]
    fn test_local_backend_list() {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(dir.path());
        backend.put("sub/a.txt", b"a").unwrap();
        backend.put("sub/b.txt", b"b").unwrap();
        let keys = backend.list("sub").unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_local_backend_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(dir.path());
        let keys = backend.list("nonexistent").unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_local_backend_name() {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(dir.path());
        assert_eq!(backend.name(), "local");
    }

    #[test]
    fn test_cmd_state_backend() {
        let dir = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(dir.path());
        backend.put("machine/state.lock.yaml", b"resources: {}\n").unwrap();
        let result = cmd_state_backend(dir.path(), None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_backend_report_serde() {
        let report = StateBackendReport {
            backend: "local".to_string(),
            keys: vec!["a.txt".to_string()],
            total: 1,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"backend\":\"local\""));
    }
}
