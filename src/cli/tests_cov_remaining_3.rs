//! Tests: Coverage for remaining validate, lock, destroy, observe (part 3).

#![allow(unused_imports)]
use super::validate_quality::*;
use super::validate_resources::*;
use super::validate_structural::*;
use super::validate_compliance::*;
use super::lock_ops::*;
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

    fn empty_config() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources: {}\n",
        )
        .to_string()
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

    fn config_world_writable() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: insecure\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  insecure-file:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/world\n",
            "    content: open\n",
            "    owner: root\n",
            "    group: root\n",
            "    mode: \"0777\"\n",
        )
        .to_string()
    }

    fn config_root_tmp() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: root-tmp\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  tmp-file:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/root-owned\n",
            "    content: data\n",
            "    owner: root\n",
            "    group: root\n",
            "    mode: \"0644\"\n",
        )
        .to_string()
    }

    fn config_with_shebang() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: shebang\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  script:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/run.sh\n",
            "    content: \"#!/bin/sh\\necho hello\"\n",
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

    // ========================================================================
    // 21. validate_quality: cmd_validate_check_idempotency
    // ========================================================================

    #[test]
    fn test_cov_check_idempotency_empty() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_idempotency(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_idempotency_data_plain() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_idempotency(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_idempotency_data_json() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_idempotency(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 22. validate_quality: cmd_validate_check_drift_coverage
    // ========================================================================

    #[test]
    fn test_cov_check_drift_coverage_empty() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_drift_coverage(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_drift_coverage_data_plain() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_drift_coverage(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_drift_coverage_data_json() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_drift_coverage(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 23. validate_quality: cmd_validate_check_complexity
    // ========================================================================

    #[test]
    fn test_cov_check_complexity_empty() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_complexity(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_complexity_data_plain() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_complexity(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_complexity_data_json() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_complexity(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 24. validate_quality: cmd_validate_check_security
    // ========================================================================

    #[test]
    fn test_cov_check_security_empty() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_security(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_security_world_writable_plain() {
        let f = write_temp_config(&config_world_writable());
        let result = cmd_validate_check_security(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_security_world_writable_json() {
        let f = write_temp_config(&config_world_writable());
        let result = cmd_validate_check_security(f.path(), true);
        assert!(result.is_ok());
    }

    // ── root-owned in /tmp ──

    #[test]
    fn test_cov_check_security_root_tmp() {
        let f = write_temp_config(&config_root_tmp());
        let result = cmd_validate_check_security(f.path(), false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 25. validate_quality: cmd_validate_check_deprecation
    // ========================================================================

    #[test]
    fn test_cov_check_deprecation_empty() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_deprecation(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_deprecation_shebang_plain() {
        let f = write_temp_config(&config_with_shebang());
        let result = cmd_validate_check_deprecation(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_deprecation_shebang_json() {
        let f = write_temp_config(&config_with_shebang());
        let result = cmd_validate_check_deprecation(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 26. lock_ops: cmd_lock_compact
    // ========================================================================

    #[test]
    fn test_cov_lock_compact_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_compact(dir.path(), false, false);
        // Empty dir is valid state dir, but nothing to compact
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_compact_with_events_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        std::fs::write(
            machine_dir.join("events.jsonl"),
            "line1\nline2\nline3\n",
        )
        .unwrap();
        let result = cmd_lock_compact(dir.path(), false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_compact_with_events_apply_json() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        std::fs::write(
            machine_dir.join("events.jsonl"),
            "line1\nline2\nline3\n",
        )
        .unwrap();
        let result = cmd_lock_compact(dir.path(), true, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_compact_nonexistent() {
        let result = cmd_lock_compact(Path::new("/tmp/nonexistent-forjar-cov"), false, false);
        assert!(result.is_err());
    }

    // ========================================================================
    // 27. lock_ops: cmd_lock_verify
    // ========================================================================

    #[test]
    fn test_cov_lock_verify_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_verify(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_verify_with_lock_plain() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        write_yaml(dir.path(), "web/lock.yaml", state_lock_yaml());
        let result = cmd_lock_verify(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_verify_with_lock_json() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        write_yaml(dir.path(), "web/lock.yaml", state_lock_yaml());
        let result = cmd_lock_verify(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_verify_nonexistent() {
        let result = cmd_lock_verify(Path::new("/tmp/nonexistent-forjar-cov"), false);
        assert!(result.is_err());
    }

    // ========================================================================
    // 28. lock_ops: cmd_lock_export
    // ========================================================================

    #[test]
    fn test_cov_lock_export_yaml() {
        let dir = make_state_dir();
        let result = cmd_lock_export(dir.path(), "yaml", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_export_json_filtered() {
        let dir = make_state_dir();
        let result = cmd_lock_export(dir.path(), "json", Some("web"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_export_unknown_format() {
        let dir = make_state_dir();
        let result = cmd_lock_export(dir.path(), "xml", None);
        assert!(result.is_err());
    }

    // ========================================================================
    // 29. lock_ops: cmd_lock_gc
    // ========================================================================

    #[test]
    fn test_cov_lock_gc_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, basic_config()).unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_lock_gc(&cfg, &state, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_gc_with_orphans_plain() {
        let dir = make_state_dir();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, basic_config()).unwrap();
        // Lock has resources f and g, but basic_config has app-config and web-svc
        // So f and g are orphaned
        let result = cmd_lock_gc(&cfg, dir.path(), false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_gc_with_orphans_yes_json() {
        let dir = make_state_dir();
        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(&cfg, basic_config()).unwrap();
        let result = cmd_lock_gc(&cfg, dir.path(), true, true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 30. lock_ops: cmd_lock_diff
    // ========================================================================

    #[test]
    fn test_cov_lock_diff_empty_dirs() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let result = cmd_lock_diff(a.path(), b.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_diff_one_populated_plain() {
        let a = make_state_dir();
        let b = tempfile::tempdir().unwrap();
        let result = cmd_lock_diff(a.path(), b.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_lock_diff_one_populated_json() {
        let a = make_state_dir();
        let b = tempfile::tempdir().unwrap();
        let result = cmd_lock_diff(a.path(), b.path(), true);
        assert!(result.is_ok());
    }
}
