//! Tests: Coverage for lock_core, lock_audit, history, diff_cmd (part 2).

#![allow(unused_imports)]
use super::lock_core::*;
use super::lock_audit::*;
use super::history::*;
use super::diff_cmd::*;
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

    fn basic_config() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: test-project\n",
            "description: A test project\n",
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
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let events = concat!(
            "{\"ts\":\"2026-01-15T10:00:00Z\",\"event\":\"resource_converged\",",
            "\"machine\":\"web\",\"resource\":\"f\",\"duration_seconds\":0.5,",
            "\"hash\":\"abc123\"}\n",
            "{\"ts\":\"2026-02-01T12:00:00Z\",\"event\":\"resource_failed\",",
            "\"machine\":\"web\",\"resource\":\"g\",\"error\":\"timeout\"}\n",
        );
        write_yaml(dir.path(), "events/web.jsonl", events);
        dir
    }

    fn make_state_dir_with_snapshots() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "web.lock.yaml", state_lock_yaml());
        write_yaml(
            dir.path(),
            "snapshots/2026-02-01.yaml",
            state_lock_yaml(),
        );
        write_yaml(
            dir.path(),
            "snapshots/2026-02-15.yaml",
            state_lock_yaml(),
        );
        dir
    }

    // ========================================================================
    // 4. lock_core: output_verify_results & collect_verify_mismatches
    // ========================================================================

    #[test]
    fn test_lock_verify_matching_text() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_yaml(dir.path(), "forjar.yaml", &basic_config());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_lock(&config_path, &state_dir, None, None, false, false);
        assert!(result.is_ok());
        let result = cmd_lock(&config_path, &state_dir, None, None, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_verify_matching_json() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_yaml(dir.path(), "forjar.yaml", &basic_config());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_lock(&config_path, &state_dir, None, None, false, false);
        assert!(result.is_ok());
        let result = cmd_lock(&config_path, &state_dir, None, None, true, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_verify_no_existing_lock() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_yaml(dir.path(), "forjar.yaml", &basic_config());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_lock(&config_path, &state_dir, None, None, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lock_generate_and_verify_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_yaml(dir.path(), "forjar.yaml", &basic_config());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(cmd_lock(&config_path, &state_dir, None, None, false, false).is_ok());
        assert!(cmd_lock(&config_path, &state_dir, None, None, true, false).is_ok());
        assert!(cmd_lock(&config_path, &state_dir, None, None, true, true).is_ok());
    }

    // ========================================================================
    // 5. lock_audit::cmd_lock_restore
    // ========================================================================

    #[test]
    fn test_lock_restore_no_snapshots_dir_text() {
        let dir = make_state_dir();
        assert!(cmd_lock_restore(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_lock_restore_no_snapshots_dir_json() {
        let dir = make_state_dir();
        assert!(cmd_lock_restore(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_lock_restore_named_snapshot_text() {
        let dir = make_state_dir_with_snapshots();
        assert!(cmd_lock_restore(dir.path(), Some("2026-02-01"), false).is_ok());
    }

    #[test]
    fn test_lock_restore_named_snapshot_json() {
        let dir = make_state_dir_with_snapshots();
        assert!(cmd_lock_restore(dir.path(), Some("2026-02-01"), true).is_ok());
    }

    #[test]
    fn test_lock_restore_latest_snapshot_text() {
        let dir = make_state_dir_with_snapshots();
        assert!(cmd_lock_restore(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_lock_restore_latest_snapshot_json() {
        let dir = make_state_dir_with_snapshots();
        assert!(cmd_lock_restore(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_lock_restore_nonexistent_snapshot() {
        let dir = make_state_dir_with_snapshots();
        assert!(cmd_lock_restore(dir.path(), Some("1999-01-01"), false).is_err());
    }

    #[test]
    fn test_lock_restore_empty_snapshots_dir_text() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        std::fs::create_dir_all(dir.path().join("snapshots")).unwrap();
        assert!(cmd_lock_restore(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_lock_restore_empty_snapshots_dir_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        std::fs::create_dir_all(dir.path().join("snapshots")).unwrap();
        assert!(cmd_lock_restore(dir.path(), None, true).is_ok());
    }

    // ========================================================================
    // 6. history::cmd_history_resource
    // ========================================================================

    #[test]
    fn test_history_resource_no_events_dir_text() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_history_resource(dir.path(), "f", 10, false).is_ok());
    }

    #[test]
    fn test_history_resource_no_events_dir_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_history_resource(dir.path(), "f", 10, true).is_ok());
    }

    #[test]
    fn test_history_resource_with_events_text() {
        let dir = make_state_dir_with_events();
        assert!(cmd_history_resource(dir.path(), "f", 10, false).is_ok());
    }

    #[test]
    fn test_history_resource_with_events_json() {
        let dir = make_state_dir_with_events();
        assert!(cmd_history_resource(dir.path(), "f", 10, true).is_ok());
    }

    #[test]
    fn test_history_resource_no_matching_events_text() {
        let dir = make_state_dir_with_events();
        assert!(cmd_history_resource(dir.path(), "nonexistent", 10, false).is_ok());
    }

    #[test]
    fn test_history_resource_no_matching_events_json() {
        let dir = make_state_dir_with_events();
        assert!(cmd_history_resource(dir.path(), "nonexistent", 10, true).is_ok());
    }

    #[test]
    fn test_history_resource_limit_1() {
        let dir = make_state_dir_with_events();
        assert!(cmd_history_resource(dir.path(), "f", 1, false).is_ok());
    }

    #[test]
    fn test_history_resource_limit_1_json() {
        let dir = make_state_dir_with_events();
        assert!(cmd_history_resource(dir.path(), "f", 1, true).is_ok());
    }

    #[test]
    fn test_history_resource_empty_events_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("events")).unwrap();
        assert!(cmd_history_resource(dir.path(), "f", 10, false).is_ok());
    }

    #[test]
    fn test_history_resource_empty_events_dir_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("events")).unwrap();
        assert!(cmd_history_resource(dir.path(), "f", 10, true).is_ok());
    }

    // ========================================================================
    // 7. diff_cmd::cmd_env_diff
    // ========================================================================

    #[test]
    fn test_env_diff_missing_env1() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("prod")).unwrap();
        assert!(cmd_env_diff("staging", "prod", dir.path(), false).is_err());
    }

    #[test]
    fn test_env_diff_missing_env2() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("staging")).unwrap();
        assert!(cmd_env_diff("staging", "prod", dir.path(), false).is_err());
    }

    #[test]
    fn test_env_diff_identical_envs_text() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "staging/web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "prod/web/state.lock.yaml", state_lock_yaml());
        assert!(cmd_env_diff("staging", "prod", dir.path(), false).is_ok());
    }

    #[test]
    fn test_env_diff_identical_envs_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "staging/web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "prod/web/state.lock.yaml", state_lock_yaml());
        assert!(cmd_env_diff("staging", "prod", dir.path(), true).is_ok());
    }

    #[test]
    fn test_env_diff_different_hashes_text() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "staging/web/state.lock.yaml", state_lock_yaml());
        let alt_lock = concat!(
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
            "    hash: \"0000000000000000000000000000000000000000000000000000000000000000\"\n",
        );
        write_yaml(dir.path(), "prod/web/state.lock.yaml", alt_lock);
        assert!(cmd_env_diff("staging", "prod", dir.path(), false).is_ok());
    }

    #[test]
    fn test_env_diff_different_hashes_json() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "staging/web/state.lock.yaml", state_lock_yaml());
        let alt_lock = concat!(
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
            "    hash: \"0000000000000000000000000000000000000000000000000000000000000000\"\n",
        );
        write_yaml(dir.path(), "prod/web/state.lock.yaml", alt_lock);
        assert!(cmd_env_diff("staging", "prod", dir.path(), true).is_ok());
    }

    #[test]
    fn test_env_diff_one_env_has_extra_resource_text() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "staging/web/state.lock.yaml", state_lock_yaml());
        let empty_lock = concat!(
            "schema: \"1\"\n",
            "machine: web\n",
            "hostname: web\n",
            "generated_at: \"2026-02-28T00:00:00Z\"\n",
            "generator: forjar\n",
            "blake3_version: \"1.8\"\n",
            "resources: {}\n",
        );
        write_yaml(dir.path(), "prod/web/state.lock.yaml", empty_lock);
        assert!(cmd_env_diff("staging", "prod", dir.path(), false).is_ok());
    }

    #[test]
    fn test_env_diff_empty_workspace_dirs_text() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("staging")).unwrap();
        std::fs::create_dir_all(dir.path().join("prod")).unwrap();
        assert!(cmd_env_diff("staging", "prod", dir.path(), false).is_ok());
    }

    #[test]
    fn test_env_diff_empty_workspace_dirs_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("staging")).unwrap();
        std::fs::create_dir_all(dir.path().join("prod")).unwrap();
        assert!(cmd_env_diff("staging", "prod", dir.path(), true).is_ok());
    }

}
