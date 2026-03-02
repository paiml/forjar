//! Tests: Coverage for lock_security, destroy, observe (part 8).

#![allow(unused_imports)]
use super::destroy::*;
use super::lock_core::*;
use super::lock_ops::*;
use super::lock_security::*;
use super::observe::*;
use super::validate_compliance::*;
use super::validate_structural::*;
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
    // 32. lock_security: cmd_lock_verify_sig
    // ========================================================================

    #[test]
    fn test_cov_lock_verify_sig_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_verify_sig(td.path(), "testkey", false).is_ok());
    }

    #[test]
    fn test_cov_lock_verify_sig_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_verify_sig(td.path(), "testkey", true).is_ok());
    }

    // ========================================================================
    // 33. lock_security: cmd_lock_compact_all
    // ========================================================================

    #[test]
    fn test_cov_lock_compact_all_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_compact_all(td.path(), false, false).is_ok());
    }

    #[test]
    fn test_cov_lock_compact_all_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_compact_all(td.path(), false, true).is_ok());
    }

    #[test]
    fn test_cov_lock_compact_all_yes() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_compact_all(td.path(), true, false).is_ok());
    }

    // ========================================================================
    // 34. lock_security: cmd_lock_audit_trail
    // ========================================================================

    #[test]
    fn test_cov_lock_audit_trail_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_audit_trail(td.path(), None, false).is_ok());
    }

    #[test]
    fn test_cov_lock_audit_trail_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_audit_trail(td.path(), None, true).is_ok());
    }

    #[test]
    fn test_cov_lock_audit_trail_with_filter() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_audit_trail(td.path(), Some("web"), false).is_ok());
    }

    // ========================================================================
    // 35. lock_security: cmd_lock_rotate_keys
    // ========================================================================

    #[test]
    fn test_cov_lock_rotate_keys_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_rotate_keys(td.path(), "old", "new", false).is_ok());
    }

    #[test]
    fn test_cov_lock_rotate_keys_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_rotate_keys(td.path(), "old", "new", true).is_ok());
    }

    // ========================================================================
    // 36. lock_security: cmd_lock_backup
    // ========================================================================

    #[test]
    fn test_cov_lock_backup_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_backup(td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_backup_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_backup(td.path(), true).is_ok());
    }

    #[test]
    fn test_cov_lock_backup_no_state() {
        let td = tempfile::tempdir().unwrap();
        let missing = td.path().join("nope");
        let result = cmd_lock_backup(&missing, false);
        assert!(result.is_err());
    }

    /// Backup with actual lock and event files.
    #[test]
    fn test_cov_lock_backup_with_files() {
        let td = tempfile::tempdir().unwrap();
        std::fs::write(
            td.path().join("web.lock.yaml"),
            "schema: \"1\"\nmachine: web\nhostname: web\nresources: {}\n",
        )
        .unwrap();
        std::fs::write(td.path().join("web.events.jsonl"), "{\"ts\":\"now\"}\n").unwrap();
        assert!(cmd_lock_backup(td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_backup_with_files_json() {
        let td = tempfile::tempdir().unwrap();
        std::fs::write(
            td.path().join("web.lock.yaml"),
            "schema: \"1\"\nmachine: web\nhostname: web\nresources: {}\n",
        )
        .unwrap();
        assert!(cmd_lock_backup(td.path(), true).is_ok());
    }

    // ========================================================================
    // 37. lock_security: cmd_lock_verify_chain
    // ========================================================================

    #[test]
    fn test_cov_lock_verify_chain_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_verify_chain(td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_verify_chain_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_verify_chain(td.path(), true).is_ok());
    }

    /// Verify chain with mismatched signature.
    #[test]
    fn test_cov_lock_verify_chain_mismatch() {
        let td = tempfile::tempdir().unwrap();
        // Create state dir with a machine subdir containing state.lock.yaml
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        std::fs::write(
            mdir.join("state.lock.yaml"),
            "schema: \"1\"\nmachine: web\nhostname: web\nresources: {}\n",
        )
        .unwrap();
        // Create the flat lock file that verify_chain looks for
        let lock_content = "schema: \"1\"\nmachine: web\nhostname: web\nresources: {}\n";
        std::fs::write(td.path().join("web.lock.yaml"), lock_content).unwrap();
        std::fs::write(td.path().join("web.lock.yaml.sig"), "wrong-hash").unwrap();
        assert!(cmd_lock_verify_chain(td.path(), false).is_ok());
    }

    // ========================================================================
    // 38. lock_security: cmd_lock_stats
    // ========================================================================

    #[test]
    fn test_cov_lock_stats_empty() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_stats(td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_stats_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_lock_stats(td.path(), true).is_ok());
    }

    /// Stats with actual lock files.
    #[test]
    fn test_cov_lock_stats_with_files() {
        let td = tempfile::tempdir().unwrap();
        // Create a machine dir with state.lock.yaml for discover_machines
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        std::fs::write(
            mdir.join("state.lock.yaml"),
            "schema: \"1\"\nmachine: web\nhostname: web\nresources: {}\n",
        )
        .unwrap();
        // Also create the flat lock file that stats reads
        std::fs::write(
            td.path().join("web.lock.yaml"),
            "schema: \"1\"\nmachine: web\nhostname: web\nresources:\n  a:\n    type: File\n    status: Converged\n    hash: abc123\n",
        )
        .unwrap();
        assert!(cmd_lock_stats(td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_stats_with_files_json() {
        let td = tempfile::tempdir().unwrap();
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        std::fs::write(
            mdir.join("state.lock.yaml"),
            "schema: \"1\"\nmachine: web\nhostname: web\nresources: {}\n",
        )
        .unwrap();
        std::fs::write(
            td.path().join("web.lock.yaml"),
            "schema: \"1\"\nmachine: web\nhostname: web\nresources: {}\n",
        )
        .unwrap();
        assert!(cmd_lock_stats(td.path(), true).is_ok());
    }

    // ========================================================================
    // 39-40. destroy: cmd_destroy, compute_rollback_changes
    // ========================================================================

    #[test]
    fn test_cov_destroy_no_yes() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&basic_config());
        let result = cmd_destroy(f.path(), td.path(), None, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--yes"));
    }

    #[test]
    fn test_cov_destroy_empty_config() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&empty_config());
        assert!(cmd_destroy(f.path(), td.path(), None, true, false).is_ok());
    }

    #[test]
    fn test_cov_destroy_empty_verbose() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&empty_config());
        assert!(cmd_destroy(f.path(), td.path(), None, true, true).is_ok());
    }

    #[test]
    fn test_cov_destroy_nonexistent_machine_filter() {
        let td = tempfile::tempdir().unwrap();
        let f = write_temp_config(&basic_config());
        assert!(cmd_destroy(f.path(), td.path(), Some("nope"), true, false).is_ok());
    }

    #[test]
    fn test_cov_compute_rollback_no_changes() {
        let yaml = &basic_config();
        let config: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let changes = compute_rollback_changes(&config, &config, 1);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_cov_compute_rollback_added_resource() {
        let prev_yaml = &empty_config();
        let cur_yaml = &basic_config();
        let prev: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(prev_yaml).unwrap();
        let cur: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(cur_yaml).unwrap();
        let changes = compute_rollback_changes(&prev, &cur, 1);
        // current has resources that prev doesn't, so we get "-" entries
        assert!(!changes.is_empty());
    }

    #[test]
    fn test_cov_compute_rollback_removed_resource() {
        let prev_yaml = &basic_config();
        let cur_yaml = &empty_config();
        let prev: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(prev_yaml).unwrap();
        let cur: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(cur_yaml).unwrap();
        let changes = compute_rollback_changes(&prev, &cur, 2);
        // prev has resources that current doesn't, so we get "+" entries
        assert!(!changes.is_empty());
    }

    // ========================================================================
    // 41-42. observe: cmd_anomaly, cmd_trace
    // ========================================================================

    #[test]
    fn test_cov_anomaly_empty_dir() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_anomaly(td.path(), None, 1, false).is_ok());
    }

    #[test]
    fn test_cov_anomaly_empty_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_anomaly(td.path(), None, 1, true).is_ok());
    }

    #[test]
    fn test_cov_anomaly_with_filter() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_anomaly(td.path(), Some("web"), 1, false).is_ok());
    }

    #[test]
    fn test_cov_anomaly_with_events() {
        let td = tempfile::tempdir().unwrap();
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        // Write valid event JSONL with various event types
        let events = concat!(
            "{\"timestamp\":\"2025-01-01T00:00:00Z\",\"event\":{\"ResourceConverged\":{\"resource\":\"a\",\"hash\":\"h1\",\"duration_ms\":100}}}\n",
            "{\"timestamp\":\"2025-01-01T00:00:01Z\",\"event\":{\"ResourceFailed\":{\"resource\":\"a\",\"error\":\"err\"}}}\n",
            "{\"timestamp\":\"2025-01-01T00:00:02Z\",\"event\":{\"DriftDetected\":{\"resource\":\"a\",\"field\":\"content\",\"expected\":\"x\",\"actual\":\"y\"}}}\n",
            "{\"timestamp\":\"2025-01-01T00:00:03Z\",\"event\":{\"ResourceConverged\":{\"resource\":\"b\",\"hash\":\"h2\",\"duration_ms\":50}}}\n",
            "\n",
        );
        std::fs::write(mdir.join("events.jsonl"), events).unwrap();
        assert!(cmd_anomaly(td.path(), None, 1, false).is_ok());
    }

    #[test]
    fn test_cov_anomaly_with_events_json() {
        let td = tempfile::tempdir().unwrap();
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        let events = concat!(
            "{\"timestamp\":\"2025-01-01T00:00:00Z\",\"event\":{\"ResourceConverged\":{\"resource\":\"a\",\"hash\":\"h1\",\"duration_ms\":100}}}\n",
            "{\"timestamp\":\"2025-01-01T00:00:01Z\",\"event\":{\"ResourceFailed\":{\"resource\":\"a\",\"error\":\"err\"}}}\n",
        );
        std::fs::write(mdir.join("events.jsonl"), events).unwrap();
        assert!(cmd_anomaly(td.path(), None, 1, true).is_ok());
    }

    /// Anomaly with high min_events filters everything out.
    #[test]
    fn test_cov_anomaly_high_min_events() {
        let td = tempfile::tempdir().unwrap();
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        let events =
            "{\"timestamp\":\"2025-01-01T00:00:00Z\",\"event\":{\"ResourceConverged\":{\"resource\":\"a\",\"hash\":\"h1\",\"duration_ms\":100}}}\n";
        std::fs::write(mdir.join("events.jsonl"), events).unwrap();
        assert!(cmd_anomaly(td.path(), None, 999, false).is_ok());
    }

    #[test]
    fn test_cov_trace_empty_dir() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_trace(td.path(), None, false).is_ok());
    }

    #[test]
    fn test_cov_trace_empty_json() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_trace(td.path(), None, true).is_ok());
    }

    #[test]
    fn test_cov_trace_with_filter() {
        let td = tempfile::tempdir().unwrap();
        assert!(cmd_trace(td.path(), Some("web"), false).is_ok());
    }

    // ========================================================================
    // 43. observe: handle_watch_change (non-blocking test)
    // ========================================================================

    #[test]
    fn test_cov_handle_watch_change_bad_config() {
        let td = tempfile::tempdir().unwrap();
        let bad = td.path().join("bad.yaml");
        std::fs::write(&bad, "not valid yaml: [[[").unwrap();
        // Should print error but not panic
        handle_watch_change(&bad, td.path(), false);
    }

    // ========================================================================
    // 44. validate_compliance: SOC2 and HIPAA policies
    // ========================================================================

    #[test]
    fn test_cov_compliance_soc2_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_compliance(f.path(), "SOC2", false).is_ok());
    }

    /// SOC2: file without owner triggers violation.
    #[test]
    fn test_cov_compliance_soc2_no_owner() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  cfg:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/cfg\n",
            "    content: x\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_compliance(f.path(), "SOC2", false).is_ok());
    }

    #[test]
    fn test_cov_compliance_soc2_json() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  cfg:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/cfg\n",
            "    content: x\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_compliance(f.path(), "SOC2", true).is_ok());
    }
}
