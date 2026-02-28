//! Tests: Coverage for validate_quality, lock_ops, lock_core (part 7).

use super::validate_resources::*;
use super::validate_quality::*;
use super::lock_ops::*;
use super::lock_core::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
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

    // ========================================================================
    // 20. validate_quality: cmd_validate_check_security
    // ========================================================================

    #[test]
    fn test_cov_security_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_security(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_security_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_security(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_security_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_security(f.path(), true).is_ok());
    }

    /// World-writable mode triggers security warning.
    #[test]
    fn test_cov_security_world_writable() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  insecure:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/insecure\n",
            "    content: x\n",
            "    owner: root\n",
            "    group: root\n",
            "    mode: \"0777\"\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_security(f.path(), false).is_ok());
    }

    /// Mode ending in 6 triggers world-writable path.
    #[test]
    fn test_cov_security_mode_6() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  rw:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /var/tmp/rw\n",
            "    content: x\n",
            "    owner: root\n",
            "    group: root\n",
            "    mode: \"0666\"\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_security(f.path(), true).is_ok());
    }

    // ========================================================================
    // 21. validate_quality: cmd_validate_check_deprecation
    // ========================================================================

    #[test]
    fn test_cov_deprecation_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_deprecation(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_deprecation_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_deprecation(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_deprecation_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_deprecation(f.path(), true).is_ok());
    }

    /// Content with shebang triggers deprecation warning.
    #[test]
    fn test_cov_deprecation_shebang() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
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
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_deprecation(f.path(), false).is_ok());
    }

    // ========================================================================
    // 22-26. lock_ops: cmd_lock_compact, cmd_lock_verify, cmd_lock_export,
    //        cmd_lock_gc, cmd_lock_diff
    // ========================================================================

    #[test]
    fn test_cov_lock_compact_no_state() {
        let td = tempfile::tempdir().unwrap();
        let missing = td.path().join("nope");
        let result = cmd_lock_compact(&missing, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_lock_compact_empty_dir() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_compact(td.path(), false, false).is_ok());
    }

    #[test]
    fn test_cov_lock_compact_empty_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_compact(td.path(), false, true).is_ok());
    }

    #[test]
    fn test_cov_lock_compact_with_events() {
        let td = tempfile::tempdir().unwrap();
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        std::fs::write(
            mdir.join("events.jsonl"),
            "{\"ts\":\"a\"}\n{\"ts\":\"b\"}\n{\"ts\":\"c\"}\n",
        )
        .unwrap();
        // dry run
        assert!(cmd_lock_compact(td.path(), false, false).is_ok());
        // actual compact
        assert!(cmd_lock_compact(td.path(), true, false).is_ok());
    }

    #[test]
    fn test_cov_lock_compact_with_events_json() {
        let td = tempfile::tempdir().unwrap();
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        std::fs::write(
            mdir.join("events.jsonl"),
            "{\"ts\":\"a\"}\n{\"ts\":\"b\"}\n",
        )
        .unwrap();
        assert!(cmd_lock_compact(td.path(), true, true).is_ok());
    }

    #[test]
    fn test_cov_lock_verify_no_state() {
        let td = tempfile::tempdir().unwrap();
        let missing = td.path().join("nope");
        let result = cmd_lock_verify(&missing, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_lock_verify_empty_dir() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_verify(td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_verify_empty_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_verify(td.path(), true).is_ok());
    }

    #[test]
    fn test_cov_lock_export_json_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_export(td.path(), "json", None).is_ok());
    }

    #[test]
    fn test_cov_lock_export_csv_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_export(td.path(), "csv", None).is_ok());
    }

    #[test]
    fn test_cov_lock_export_yaml_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_export(td.path(), "yaml", None).is_ok());
    }

    #[test]
    fn test_cov_lock_export_unknown_format() {
        let td = tempfile::tempdir().unwrap();
        let result = cmd_lock_export(td.path(), "xml", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_lock_gc_empty() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&empty_config());
        assert!(cmd_lock_gc(f.path(), td.path(), false, false).is_ok());
    }

    #[test]
    fn test_cov_lock_gc_json() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&empty_config());
        assert!(cmd_lock_gc(f.path(), td.path(), false, true).is_ok());
    }

    #[test]
    fn test_cov_lock_diff_empty_dirs() {
        let from = tempfile::tempdir().unwrap();
        let to = tempfile::tempdir().unwrap();
        assert!(cmd_lock_diff(from.path(), to.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_diff_json() {
        let from = tempfile::tempdir().unwrap();
        let to = tempfile::tempdir().unwrap();
        assert!(cmd_lock_diff(from.path(), to.path(), true).is_ok());
    }

    // ========================================================================
    // 27-31. lock_core: cmd_lock, cmd_lock_info, cmd_lock_prune,
    //        cmd_lock_validate, cmd_lock_integrity
    // ========================================================================

    #[test]
    fn test_cov_lock_cmd_basic() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&basic_config());
        assert!(cmd_lock(f.path(), td.path(), None, None, false, false).is_ok());
    }

    #[test]
    fn test_cov_lock_cmd_json() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&basic_config());
        assert!(cmd_lock(f.path(), td.path(), None, None, false, true).is_ok());
    }

    #[test]
    fn test_cov_lock_cmd_empty() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&empty_config());
        assert!(cmd_lock(f.path(), td.path(), None, None, false, false).is_ok());
    }

    #[test]
    fn test_cov_lock_info_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_info(td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_info_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_info(td.path(), true).is_ok());
    }

    #[test]
    fn test_cov_lock_prune_empty() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&empty_config());
        assert!(cmd_lock_prune(f.path(), td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_validate_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_validate(td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_validate_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_validate(td.path(), true).is_ok());
    }

    #[test]
    fn test_cov_lock_integrity_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_integrity(td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_integrity_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_integrity(td.path(), true).is_ok());
    }

    /// Lock then verify should succeed.
    #[test]
    fn test_cov_lock_verify_after_lock() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&basic_config());
        cmd_lock(f.path(), td.path(), None, None, false, false).unwrap();
        assert!(cmd_lock(f.path(), td.path(), None, None, true, false).is_ok());
    }

    /// Lock then verify with JSON.
    #[test]
    fn test_cov_lock_verify_after_lock_json() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&basic_config());
        cmd_lock(f.path(), td.path(), None, None, false, false).unwrap();
        assert!(cmd_lock(f.path(), td.path(), None, None, true, true).is_ok());
    }
}
