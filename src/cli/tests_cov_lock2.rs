//! Coverage tests for lock_ops.rs, lock_core.rs, lock_security.rs.

#![allow(unused_imports)]
use super::lock_ops::*;
use super::lock_core::*;
use super::lock_merge::cmd_lock_sign;
use super::lock_security::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
    }

    fn setup_state() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 2.5\n  mysql:\n    resource_type: Package\n    status: Failed\n    hash: def456\n");
        write_yaml(dir.path(), "web1/events.jsonl", "{\"ts\":\"2026-01-01T00:00:00Z\",\"event\":\"resource_started\",\"resource\":\"nginx\",\"machine\":\"web1\"}\n");
        dir
    }

    fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const CFG: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n    state: present\n    depends_on:\n      - pkg\n";

    // lock_ops
    #[test]
    fn test_lock_compact_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_lock_compact(d.path(), false, false).is_ok());
    }
    #[test]
    fn test_lock_compact_with_state() {
        let d = setup_state();
        assert!(cmd_lock_compact(d.path(), false, false).is_ok());
    }
    #[test]
    fn test_lock_compact_json() {
        let d = setup_state();
        assert!(cmd_lock_compact(d.path(), false, true).is_ok());
    }
    #[test]
    fn test_lock_verify_empty() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_lock_verify(d.path(), false);
    }
    #[test]
    fn test_lock_verify_with_state() {
        let d = setup_state();
        let _ = cmd_lock_verify(d.path(), false);
    }
    #[test]
    fn test_lock_verify_json() {
        let d = setup_state();
        let _ = cmd_lock_verify(d.path(), true);
    }
    #[test]
    fn test_lock_export_yaml() {
        let d = setup_state();
        assert!(cmd_lock_export(d.path(), "yaml", None).is_ok());
    }
    #[test]
    fn test_lock_export_json() {
        let d = setup_state();
        assert!(cmd_lock_export(d.path(), "json", None).is_ok());
    }
    #[test]
    fn test_lock_export_machine_filter() {
        let d = setup_state();
        assert!(cmd_lock_export(d.path(), "yaml", Some("web1")).is_ok());
    }
    #[test]
    fn test_lock_gc() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_lock_gc(f.path(), d.path(), false, false);
    }
    #[test]
    fn test_lock_gc_json() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_lock_gc(f.path(), d.path(), false, true);
    }
    #[test]
    fn test_lock_diff_same() {
        let d = setup_state();
        let p = d.path().join("web1/state.lock.yaml");
        let _ = cmd_lock_diff(&p, &p, false);
    }

    // lock_core
    #[test]
    fn test_lock_info_empty() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_lock_info(d.path(), false);
    }
    #[test]
    fn test_lock_info_with_state() {
        let d = setup_state();
        let _ = cmd_lock_info(d.path(), false);
    }
    #[test]
    fn test_lock_info_json() {
        let d = setup_state();
        let _ = cmd_lock_info(d.path(), true);
    }
    #[test]
    fn test_lock_validate_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_lock_validate(d.path(), false).is_ok());
    }
    #[test]
    fn test_lock_validate_with_state() {
        let d = setup_state();
        // setup_state creates minimal locks missing required fields — validation catches this
        let _ = cmd_lock_validate(d.path(), false);
    }
    #[test]
    fn test_lock_validate_json() {
        let d = setup_state();
        let _ = cmd_lock_validate(d.path(), true);
    }
    #[test]
    fn test_lock_integrity_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_lock_integrity(d.path(), false).is_ok());
    }
    #[test]
    fn test_lock_integrity_with_state() {
        let d = setup_state();
        let _ = cmd_lock_integrity(d.path(), false);
    }
    #[test]
    fn test_lock_integrity_json() {
        let d = setup_state();
        let _ = cmd_lock_integrity(d.path(), true);
    }
    #[test]
    fn test_lock_prune() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_lock_prune(f.path(), d.path(), false);
    }

    // lock_security
    #[test]
    fn test_lock_audit_trail_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_lock_audit_trail(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_lock_audit_trail_with_state() {
        let d = setup_state();
        assert!(cmd_lock_audit_trail(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_lock_audit_trail_json() {
        let d = setup_state();
        assert!(cmd_lock_audit_trail(d.path(), None, true).is_ok());
    }
    #[test]
    fn test_lock_backup() {
        let d = setup_state();
        assert!(cmd_lock_backup(d.path(), false).is_ok());
    }
    #[test]
    fn test_lock_backup_json() {
        let d = setup_state();
        assert!(cmd_lock_backup(d.path(), true).is_ok());
    }
    #[test]
    fn test_lock_verify_chain() {
        let d = setup_state();
        assert!(cmd_lock_verify_chain(d.path(), false).is_ok());
    }
    #[test]
    fn test_lock_verify_chain_json() {
        let d = setup_state();
        assert!(cmd_lock_verify_chain(d.path(), true).is_ok());
    }
    #[test]
    fn test_lock_stats_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_lock_stats(d.path(), false).is_ok());
    }
    #[test]
    fn test_lock_stats_with_state() {
        let d = setup_state();
        assert!(cmd_lock_stats(d.path(), false).is_ok());
    }
    #[test]
    fn test_lock_stats_json() {
        let d = setup_state();
        assert!(cmd_lock_stats(d.path(), true).is_ok());
    }
    #[test]
    fn test_lock_compact_all() {
        let d = setup_state();
        assert!(cmd_lock_compact_all(d.path(), false, false).is_ok());
    }
    #[test]
    fn test_lock_compact_all_json() {
        let d = setup_state();
        assert!(cmd_lock_compact_all(d.path(), false, true).is_ok());
    }

    // ── Full sign → verify-sig → verify-chain → rotate flow ──

    fn setup_valid_state() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let lock_yaml = concat!(
            "schema: '1.0'\n",
            "machine: srv1\n",
            "hostname: srv1\n",
            "generator: forjar 1.1.1\n",
            "generated_at: '2026-01-01T00:00:00Z'\n",
            "blake3_version: '1.5'\n",
            "resources:\n",
            "  app:\n",
            "    type: file\n",
            "    status: converged\n",
            "    hash: blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
            "    applied_at: '2026-01-01T00:00:00Z'\n",
            "    duration_seconds: 0.5\n",
        );
        write_yaml(dir.path(), "srv1/state.lock.yaml", lock_yaml);
        dir
    }

    #[test]
    fn test_sign_then_verify_sig() {
        let d = setup_valid_state();
        assert!(cmd_lock_sign(d.path(), "mykey", false).is_ok());
        assert!(cmd_lock_verify_sig(d.path(), "mykey", false).is_ok());
    }

    #[test]
    fn test_sign_then_verify_sig_json() {
        let d = setup_valid_state();
        assert!(cmd_lock_sign(d.path(), "k", true).is_ok());
        assert!(cmd_lock_verify_sig(d.path(), "k", true).is_ok());
    }

    #[test]
    fn test_verify_sig_wrong_key_fails() {
        let d = setup_valid_state();
        assert!(cmd_lock_sign(d.path(), "right", false).is_ok());
        assert!(cmd_lock_verify_sig(d.path(), "wrong", false).is_err());
    }

    #[test]
    fn test_sign_then_verify_chain() {
        let d = setup_valid_state();
        assert!(cmd_lock_sign(d.path(), "k", false).is_ok());
        assert!(cmd_lock_verify_chain(d.path(), false).is_ok());
    }

    #[test]
    fn test_sign_then_verify_chain_json() {
        let d = setup_valid_state();
        assert!(cmd_lock_sign(d.path(), "k", false).is_ok());
        assert!(cmd_lock_verify_chain(d.path(), true).is_ok());
    }

    #[test]
    fn test_rotate_keys_valid() {
        let d = setup_valid_state();
        assert!(cmd_lock_sign(d.path(), "old", false).is_ok());
        assert!(cmd_lock_rotate_keys(d.path(), "old", "new", false).is_ok());
        assert!(cmd_lock_verify_sig(d.path(), "new", false).is_ok());
    }

    #[test]
    fn test_rotate_keys_valid_json() {
        let d = setup_valid_state();
        assert!(cmd_lock_sign(d.path(), "old", false).is_ok());
        assert!(cmd_lock_rotate_keys(d.path(), "old", "new", true).is_ok());
    }

    #[test]
    fn test_rotate_keys_wrong_old_key_fails() {
        let d = setup_valid_state();
        assert!(cmd_lock_sign(d.path(), "real", false).is_ok());
        assert!(cmd_lock_rotate_keys(d.path(), "fake", "new", false).is_err());
    }

    #[test]
    fn test_compact_all_yes() {
        let d = setup_valid_state();
        assert!(cmd_lock_compact_all(d.path(), true, false).is_ok());
    }

    #[test]
    fn test_compact_all_yes_json() {
        let d = setup_valid_state();
        assert!(cmd_lock_compact_all(d.path(), true, true).is_ok());
    }

    // ── lock_audit with valid blake3-prefixed hashes ──

    #[test]
    fn test_lock_audit_valid_hashes() {
        let d = setup_valid_state();
        assert!(super::super::lock_audit::cmd_lock_audit(d.path(), false).is_ok());
    }

    #[test]
    fn test_lock_audit_valid_hashes_json() {
        let d = setup_valid_state();
        assert!(super::super::lock_audit::cmd_lock_audit(d.path(), true).is_ok());
    }

    // ── show strip_defaults ──

    #[test]
    fn test_show_strip_defaults() {
        use super::super::show::strip_defaults;
        let mut val = serde_json::json!({
            "name": "test",
            "path": null,
            "enabled": false,
            "tags": [],
            "extra": {},
            "nested": {
                "a": null,
                "b": "keep"
            }
        });
        strip_defaults(&mut val);
        let obj = val.as_object().unwrap();
        assert!(obj.contains_key("name"));
        assert!(!obj.contains_key("path"));
        assert!(!obj.contains_key("enabled"));
        assert!(!obj.contains_key("tags"));
        assert!(!obj.contains_key("extra"));
        assert!(obj.contains_key("nested"));
        let nested = obj["nested"].as_object().unwrap();
        assert!(!nested.contains_key("a"));
        assert!(nested.contains_key("b"));
    }

    // ── fleet_reporting: export json ──

    #[test]
    fn test_export_json_format() {
        let d = setup_state();
        let _ = super::super::fleet_reporting::cmd_export(d.path(), "json", None, None);
    }

    // ── lock_audit: history with valid data ──

    #[test]
    fn test_lock_history_with_data() {
        let d = setup_valid_state();
        assert!(super::super::lock_audit::cmd_lock_history(d.path(), false, 10).is_ok());
    }

    #[test]
    fn test_lock_history_with_data_json() {
        let d = setup_valid_state();
        assert!(super::super::lock_audit::cmd_lock_history(d.path(), true, 5).is_ok());
    }

    #[test]
    fn test_lock_audit_trail_machine_filter() {
        let d = setup_valid_state();
        write_yaml(d.path(), "srv1/srv1.events.jsonl",
            "{\"timestamp\":\"2026-01-01\",\"resource\":\"app\",\"action\":\"apply\"}\n");
        assert!(cmd_lock_audit_trail(d.path(), Some("srv1"), false).is_ok());
    }

    #[test]
    fn test_lock_audit_trail_machine_filter_json() {
        let d = setup_valid_state();
        write_yaml(d.path(), "srv1/srv1.events.jsonl",
            "{\"timestamp\":\"2026-01-01\",\"resource\":\"app\",\"action\":\"apply\"}\n");
        assert!(cmd_lock_audit_trail(d.path(), Some("srv1"), true).is_ok());
    }

    #[test]
    fn test_lock_verify_hmac() {
        let d = setup_valid_state();
        assert!(super::super::lock_audit::cmd_lock_verify_hmac(d.path(), false).is_ok());
    }

    #[test]
    fn test_lock_verify_hmac_json() {
        let d = setup_valid_state();
        assert!(super::super::lock_audit::cmd_lock_verify_hmac(d.path(), true).is_ok());
    }

    #[test]
    fn test_lock_verify_schema() {
        let d = setup_valid_state();
        assert!(super::super::lock_audit::cmd_lock_verify_schema(d.path(), false).is_ok());
    }

    #[test]
    fn test_lock_verify_schema_json() {
        let d = setup_valid_state();
        assert!(super::super::lock_audit::cmd_lock_verify_schema(d.path(), true).is_ok());
    }

    // ── lock_security: backup with data ──

    #[test]
    fn test_lock_backup_with_data() {
        let d = setup_valid_state();
        assert!(cmd_lock_backup(d.path(), false).is_ok());
    }

    #[test]
    fn test_lock_backup_with_data_json() {
        let d = setup_valid_state();
        assert!(cmd_lock_backup(d.path(), true).is_ok());
    }

    // ── lock_security: stats with data ──

    #[test]
    fn test_lock_stats_with_data() {
        let d = setup_valid_state();
        assert!(cmd_lock_stats(d.path(), false).is_ok());
    }

    #[test]
    fn test_lock_stats_with_data_json() {
        let d = setup_valid_state();
        assert!(cmd_lock_stats(d.path(), true).is_ok());
    }

    // ── lock_core: validate + integrity with valid data ──

    fn setup_schema_state() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let lock_yaml = concat!(
            "schema: '1.0'\n",
            "machine: m1\n",
            "hostname: m1\n",
            "generator: forjar 1.1.1\n",
            "generated_at: '2026-01-01T00:00:00Z'\n",
            "blake3_version: '1.5'\n",
            "resources:\n",
            "  pkg:\n",
            "    type: package\n",
            "    status: converged\n",
            "    hash: blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n",
        );
        write_yaml(dir.path(), "m1/state.lock.yaml", lock_yaml);
        dir
    }

    #[test]
    fn test_lock_validate_valid_schema() {
        let d = setup_schema_state();
        assert!(cmd_lock_validate(d.path(), false).is_ok());
    }

    #[test]
    fn test_lock_validate_valid_schema_json() {
        let d = setup_schema_state();
        assert!(cmd_lock_validate(d.path(), true).is_ok());
    }

    #[test]
    fn test_lock_integrity_valid() {
        let d = setup_schema_state();
        assert!(cmd_lock_integrity(d.path(), false).is_ok());
    }

    #[test]
    fn test_lock_integrity_valid_json() {
        let d = setup_schema_state();
        assert!(cmd_lock_integrity(d.path(), true).is_ok());
    }
}
