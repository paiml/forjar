//! Coverage tests for lock_ops.rs, lock_core.rs, lock_security.rs.

#![allow(unused_imports)]
use super::lock_ops::*;
use super::lock_core::*;
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
}
