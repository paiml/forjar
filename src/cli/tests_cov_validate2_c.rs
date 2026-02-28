//! Tests: Coverage for lock_repair and lock_normalize (part 3).

use super::lock_repair::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_yaml(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
        p
    }

    fn state_lock_yaml() -> &'static str {
        concat!(
            "schema: \"1\"\n",
            "machine: web\n",
            "hostname: web\n",
            "generated_at: \"2026-02-28T00:00:00Z\"\n",
            "generator: forjar\n",
            "blake3_version: \"1.8\"\n",
            "resources:\n",
            "  f:\n",
            "    type: file\n",
            "    status: converged\n",
            "    hash: \"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789\"\n",
        )
    }

    fn make_state_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "web.lock.yaml", state_lock_yaml());
        dir
    }

    fn make_state_dir_corrupted() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "web.lock.yaml", "NOT VALID YAML: {{{{");
        dir
    }

    // ========================================================================
    // lock_repair::cmd_lock_repair
    // ========================================================================

    #[test]
    fn test_lock_repair_valid_locks_text() {
        let dir = make_state_dir();
        let result = cmd_lock_repair(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_repair_valid_locks_json() {
        let dir = make_state_dir();
        let result = cmd_lock_repair(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_repair_corrupted_text() {
        let dir = make_state_dir_corrupted();
        let result = cmd_lock_repair(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_repair_corrupted_json() {
        let dir = make_state_dir_corrupted();
        let result = cmd_lock_repair(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_repair_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_repair(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_repair_empty_dir_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_repair(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_repair_no_flat_lock_file() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let result = cmd_lock_repair(dir.path(), false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // lock_repair::cmd_lock_normalize
    // ========================================================================

    #[test]
    fn test_lock_normalize_already_normal_text() {
        let dir = make_state_dir();
        let result = cmd_lock_normalize(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_normalize_already_normal_json() {
        let dir = make_state_dir();
        let result = cmd_lock_normalize(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_normalize_with_whitespace_diff_text() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let messy_lock = concat!(
            "schema:    \"1\"\n",
            "machine:   web\n",
            "hostname:  web\n",
            "generated_at:    \"2026-02-28T00:00:00Z\"\n",
            "generator:   forjar\n",
            "blake3_version:  \"1.8\"\n",
            "resources:\n",
            "  f:\n",
            "    type: file\n",
            "    status: converged\n",
            "    hash: \"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789\"\n",
        );
        write_yaml(dir.path(), "web.lock.yaml", messy_lock);
        let result = cmd_lock_normalize(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_normalize_with_whitespace_diff_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let messy_lock = concat!(
            "schema:    \"1\"\n",
            "machine:   web\n",
            "hostname:  web\n",
            "generated_at:    \"2026-02-28T00:00:00Z\"\n",
            "generator:   forjar\n",
            "blake3_version:  \"1.8\"\n",
            "resources:\n",
            "  f:\n",
            "    type: file\n",
            "    status: converged\n",
            "    hash: \"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789\"\n",
        );
        write_yaml(dir.path(), "web.lock.yaml", messy_lock);
        let result = cmd_lock_normalize(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_normalize_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_normalize(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_normalize_empty_dir_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_normalize(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_normalize_no_flat_lock() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let result = cmd_lock_normalize(dir.path(), false);
        assert!(result.is_ok());
    }
}
