//! Tests: Coverage for remaining validate, lock, destroy, observe (part 4).

use super::lock_core::*;
use super::lock_security::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn write_yaml(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
        p
    }

    fn basic_config() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: test-project\n",
            "machines:\n",
            "  web:\n",
            "    hostname: web\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  app-config:\n",
            "    type: file\n",
            "    machine: web\n",
            "    path: /etc/app.conf\n",
            "    content: \"port=8080\"\n",
            "    owner: root\n",
            "    group: root\n",
            "    mode: \"0644\"\n",
            "  web-svc:\n",
            "    type: service\n",
            "    machine: web\n",
            "    name: nginx\n",
            "    depends_on: [app-config]\n",
        )
        .to_string()
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
            "  g:\n",
            "    type: service\n",
            "    status: drifted\n",
            "    hash: \"1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef\"\n",
        )
    }

    fn make_state_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "web.lock.yaml", state_lock_yaml());
        dir
    }

    fn make_state_dir_with_events() -> tempfile::TempDir {
        let dir = make_state_dir();
        let events = concat!(
            "{\"timestamp\":\"2026-01-01T00:00:00Z\",\"resource\":\"f\",\"action\":\"apply\"}\n",
            "{\"timestamp\":\"2026-02-01T00:00:00Z\",\"resource\":\"g\",\"action\":\"converge\"}\n",
        );
        write_yaml(dir.path(), "web/events.jsonl", events);
        write_yaml(dir.path(), "web.events.jsonl", events);
        dir
    }

    // ========================================================================
    // 31. lock_core: cmd_lock
    // ========================================================================

    #[test]
    fn test_cov_lock_generate_text() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", &basic_config());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_lock(&cfg, &state_dir, None, None, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_generate_json() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", &basic_config());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_lock(&cfg, &state_dir, None, None, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_generate_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", "NOT VALID");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_lock(&cfg, &state_dir, None, None, false, false);
        assert!(result.is_err());
    }

    // ========================================================================
    // 32. lock_core: cmd_lock_info
    // ========================================================================

    #[test]
    fn test_cov_lock_info_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_info(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_info_with_data_plain() {
        let dir = make_state_dir();
        let result = cmd_lock_info(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_info_with_data_json() {
        let dir = make_state_dir();
        let result = cmd_lock_info(dir.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 33. lock_core: cmd_lock_prune
    // ========================================================================

    #[test]
    fn test_cov_lock_prune_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", &basic_config());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_lock_prune(&cfg, &state, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_prune_with_stale_dry() {
        let dir = make_state_dir();
        let cfg = write_yaml(dir.path(), "forjar.yaml", &basic_config());
        let result = cmd_lock_prune(&cfg, dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_prune_with_stale_yes() {
        let dir = make_state_dir();
        let cfg = write_yaml(dir.path(), "forjar.yaml", &basic_config());
        let result = cmd_lock_prune(&cfg, dir.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 34. lock_core: cmd_lock_validate
    // ========================================================================

    #[test]
    fn test_cov_lock_validate_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_validate(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_validate_with_data_plain() {
        let dir = make_state_dir();
        let result = cmd_lock_validate(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_validate_with_data_json() {
        let dir = make_state_dir();
        let result = cmd_lock_validate(dir.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 35. lock_core: cmd_lock_integrity
    // ========================================================================

    #[test]
    fn test_cov_lock_integrity_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_integrity(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_integrity_with_data_plain() {
        let dir = make_state_dir();
        let result = cmd_lock_integrity(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_integrity_with_data_json() {
        let dir = make_state_dir();
        let result = cmd_lock_integrity(dir.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 36. lock_security: cmd_lock_verify_sig
    // ========================================================================

    #[test]
    fn test_cov_lock_verify_sig_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_verify_sig(dir.path(), "key", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_verify_sig_data_plain() {
        let dir = make_state_dir();
        let result = cmd_lock_verify_sig(dir.path(), "test-key", false);
        // Sig won't match, but function still runs
        let _ = result;
    }

    #[test]
    fn test_cov_lock_verify_sig_data_json() {
        let dir = make_state_dir();
        let result = cmd_lock_verify_sig(dir.path(), "test-key", true);
        let _ = result;
    }

    // ========================================================================
    // 37. lock_security: cmd_lock_compact_all
    // ========================================================================

    #[test]
    fn test_cov_lock_compact_all_empty_plain() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_compact_all(dir.path(), false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_compact_all_empty_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_compact_all(dir.path(), false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_compact_all_yes_json() {
        let dir = make_state_dir();
        let result = cmd_lock_compact_all(dir.path(), true, true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 38. lock_security: cmd_lock_audit_trail
    // ========================================================================

    #[test]
    fn test_cov_lock_audit_trail_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_audit_trail(dir.path(), None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_audit_trail_with_events_plain() {
        let dir = make_state_dir_with_events();
        let result = cmd_lock_audit_trail(dir.path(), None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_audit_trail_with_events_json() {
        let dir = make_state_dir_with_events();
        let result = cmd_lock_audit_trail(dir.path(), Some("web"), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 39. lock_security: cmd_lock_rotate_keys
    // ========================================================================

    #[test]
    fn test_cov_lock_rotate_keys_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_rotate_keys(dir.path(), "old", "new", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_rotate_keys_data_plain() {
        let dir = make_state_dir();
        let result = cmd_lock_rotate_keys(dir.path(), "old-key", "new-key", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_rotate_keys_data_json() {
        let dir = make_state_dir();
        let result = cmd_lock_rotate_keys(dir.path(), "old", "new", true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 40. lock_security: cmd_lock_backup
    // ========================================================================

    #[test]
    fn test_cov_lock_backup_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_backup(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_backup_with_data_plain() {
        let dir = make_state_dir();
        let result = cmd_lock_backup(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_backup_with_data_json() {
        let dir = make_state_dir();
        let result = cmd_lock_backup(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_backup_nonexistent() {
        let result = cmd_lock_backup(Path::new("/tmp/nonexistent-forjar-cov-backup"), false);
        assert!(result.is_err());
    }

    // ========================================================================
    // 41. lock_security: cmd_lock_verify_chain
    // ========================================================================

    #[test]
    fn test_cov_lock_verify_chain_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_verify_chain(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_verify_chain_data_plain() {
        let dir = make_state_dir();
        let result = cmd_lock_verify_chain(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_verify_chain_data_json() {
        let dir = make_state_dir();
        let result = cmd_lock_verify_chain(dir.path(), true);
        assert!(result.is_ok());
    }

    // ── with signature file ──

    #[test]
    fn test_cov_lock_verify_chain_with_sig() {
        let dir = make_state_dir();
        // Create a signature file for the lock
        let lock_content = std::fs::read_to_string(dir.path().join("web.lock.yaml")).unwrap();
        let hash = crate::tripwire::hasher::hash_string(&lock_content);
        write_yaml(dir.path(), "web.lock.yaml.sig", &hash);
        let result = cmd_lock_verify_chain(dir.path(), false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 42. lock_security: cmd_lock_stats
    // ========================================================================

    #[test]
    fn test_cov_lock_stats_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_stats(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_stats_data_plain() {
        let dir = make_state_dir();
        let result = cmd_lock_stats(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_stats_data_json() {
        let dir = make_state_dir();
        let result = cmd_lock_stats(dir.path(), true);
        assert!(result.is_ok());
    }
}
