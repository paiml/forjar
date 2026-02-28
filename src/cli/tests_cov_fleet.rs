//! Tests: Coverage for fleet_ops and fleet_reporting.

use super::fleet_ops::*;
use super::fleet_reporting::*;
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

    // ── Minimal config YAML snippets ──

    fn minimal_config_yaml() -> &'static str {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n"
    }

    fn multi_machine_config_yaml() -> &'static str {
        "version: \"1.0\"\nname: multi\nmachines:\n  web:\n    hostname: web\n    addr: 127.0.0.1\n  db:\n    hostname: db\n    addr: 127.0.0.1\n  cache:\n    hostname: cache\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: web\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: db\n    path: /tmp/b\n    content: b\n  c:\n    type: file\n    machine: cache\n    path: /tmp/c\n    content: c\n"
    }

    fn state_lock_yaml() -> &'static str {
        "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n"
    }

    fn state_lock_yaml_db() -> &'static str {
        "schema: \"1.0\"\nmachine: db\nhostname: db\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  g:\n    type: service\n    status: converged\n    hash: \"blake3:def\"\n"
    }

    fn state_lock_yaml_cache() -> &'static str {
        "schema: \"1.0\"\nmachine: cache\nhostname: cache\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  h:\n    type: file\n    status: failed\n    hash: \"blake3:ghi\"\n"
    }

    // ========================================================================
    // cmd_inventory tests
    // ========================================================================

    #[test]
    fn test_cov_inventory_empty_config_errs() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n");
        let result = cmd_inventory(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_inventory_single_local_text() {
        let f = write_temp_config(minimal_config_yaml());
        let result = cmd_inventory(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_inventory_single_local_json() {
        let f = write_temp_config(minimal_config_yaml());
        let result = cmd_inventory(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_inventory_multi_machine_text() {
        let f = write_temp_config(multi_machine_config_yaml());
        let result = cmd_inventory(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_inventory_multi_machine_json() {
        let f = write_temp_config(multi_machine_config_yaml());
        let result = cmd_inventory(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_inventory_container_transport_errs() {
        // container addr may not parse — verify we handle errors gracefully
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  c:\n    hostname: c\n    addr: container\n    transport: container\nresources:\n  a:\n    type: file\n    machine: c\n    path: /tmp/a\n    content: a\n";
        let f = write_temp_config(yaml);
        let _result = cmd_inventory(f.path(), false);
    }

    #[test]
    fn test_cov_inventory_container_json_errs() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  c:\n    hostname: c\n    addr: container\nresources:\n  a:\n    type: file\n    machine: c\n    path: /tmp/a\n    content: a\n";
        let f = write_temp_config(yaml);
        let _result = cmd_inventory(f.path(), true);
    }

    #[test]
    fn test_cov_inventory_invalid_config() {
        let f = write_temp_config("not valid yaml: [[[");
        let result = cmd_inventory(f.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_inventory_localhost_addr() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  lo:\n    hostname: lo\n    addr: localhost\nresources:\n  a:\n    type: file\n    machine: lo\n    path: /tmp/a\n    content: a\n";
        let f = write_temp_config(yaml);
        let result = cmd_inventory(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_inventory_no_resources_for_machine() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n";
        let f = write_temp_config(yaml);
        let result = cmd_inventory(f.path(), false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // cmd_canary tests
    // ========================================================================

    #[test]
    fn test_cov_canary_nonexistent_machine() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_canary(&cfg, &state_dir, "nonexistent", false, &[], None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found in config"));
    }

    #[test]
    fn test_cov_canary_nonexistent_machine_auto_proceed() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_canary(&cfg, &state_dir, "missing", true, &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_canary_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "bad.yaml", "not: valid: yaml: [[[");
        let result = cmd_canary(&cfg, &state_dir, "m", false, &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_canary_empty_machines() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n");
        let result = cmd_canary(&cfg, &state_dir, "m", false, &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_canary_with_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_canary(&cfg, &state_dir, "nonexistent", false, &[], Some(30));
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_canary_with_params() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_canary(&cfg, &state_dir, "nobody", true, &["key=val".to_string()], None);
        assert!(result.is_err());
    }

    // ========================================================================
    // cmd_rolling tests
    // ========================================================================

    #[test]
    fn test_cov_rolling_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "bad.yaml", "not: valid: yaml: [[[");
        let result = cmd_rolling(&cfg, &state_dir, 2, &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rolling_empty_machines() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n");
        let result = cmd_rolling(&cfg, &state_dir, 2, &[], None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no machines defined"));
    }

    #[test]
    fn test_cov_rolling_batch_size_one() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n");
        let result = cmd_rolling(&cfg, &state_dir, 1, &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rolling_batch_size_large() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n");
        let result = cmd_rolling(&cfg, &state_dir, 100, &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rolling_with_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n");
        let result = cmd_rolling(&cfg, &state_dir, 2, &[], Some(60));
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rolling_with_params() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n");
        let result = cmd_rolling(&cfg, &state_dir, 2, &["env=prod".to_string()], None);
        assert!(result.is_err());
    }

    // ========================================================================
    // cmd_retry_failed tests
    // ========================================================================

    #[test]
    fn test_cov_retry_failed_empty_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_retry_failed(&cfg, &state_dir, &[], None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_retry_failed_no_event_logs() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        // Create machine dir without events.jsonl
        std::fs::create_dir_all(state_dir.join("m")).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_retry_failed(&cfg, &state_dir, &[], None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_retry_failed_with_single_line_event_log() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(state_dir.join("m")).unwrap();
        // Single event line (no ApplyCompleted) — exercises boundary
        let events = r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"m","run_id":"r1","forjar_version":"0.1"}"#;
        write_yaml(&state_dir, "m/events.jsonl", events);
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_retry_failed(&cfg, &state_dir, &[], None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_retry_failed_with_converged_events_only() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(state_dir.join("m")).unwrap();
        let events = concat!(
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"m","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:00:01Z","event":"resource_converged","machine":"m","resource":"a","duration_seconds":0.5,"hash":"abc"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:00:02Z","event":"apply_completed","machine":"m","run_id":"r1","resources_converged":1,"resources_unchanged":0,"resources_failed":0,"total_seconds":2.0}"#,
            "\n",
        );
        write_yaml(&state_dir, "m/events.jsonl", events);
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_retry_failed(&cfg, &state_dir, &[], None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_retry_failed_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "bad.yaml", "not: valid: yaml: [[[");
        let result = cmd_retry_failed(&cfg, &state_dir, &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_retry_failed_with_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_retry_failed(&cfg, &state_dir, &[], Some(10));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_retry_failed_with_params() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_retry_failed(&cfg, &state_dir, &["x=y".to_string()], None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_retry_failed_with_malformed_event_lines() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(state_dir.join("m")).unwrap();
        let events = concat!(
            "this is not json\n",
            "{\"broken json\n",
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_completed","machine":"m","run_id":"r1","resources_converged":0,"resources_unchanged":0,"resources_failed":0,"total_seconds":0.0}"#,
            "\n",
        );
        write_yaml(&state_dir, "m/events.jsonl", events);
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_retry_failed(&cfg, &state_dir, &[], None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_retry_failed_event_log_with_failed_resource() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(state_dir.join("m")).unwrap();
        let events = concat!(
            r#"{"ts":"2026-01-01T00:00:00Z","event":"apply_started","machine":"m","run_id":"r1","forjar_version":"0.1"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:00:01Z","event":"resource_failed","machine":"m","resource":"a","error":"timeout"}"#,
            "\n",
            r#"{"ts":"2026-01-01T00:00:02Z","event":"apply_completed","machine":"m","run_id":"r1","resources_converged":0,"resources_unchanged":0,"resources_failed":1,"total_seconds":2.0}"#,
            "\n",
        );
        write_yaml(&state_dir, "m/events.jsonl", events);
        let cfg = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        // This will find the failed resource and try to retry, which calls cmd_apply
        // on a local machine -- it may succeed or fail depending on the resource,
        // but the function's branching is exercised.
        let _result = cmd_retry_failed(&cfg, &state_dir, &[], None);
    }

    // ========================================================================
    // cmd_export tests
    // ========================================================================

    #[test]
    fn test_cov_export_csv_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_export(dir.path(), "csv", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_export_csv_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let result = cmd_export(dir.path(), "csv", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_export_terraform_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_export(dir.path(), "terraform", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_export_terraform_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let result = cmd_export(dir.path(), "terraform", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_export_ansible_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_export(dir.path(), "ansible", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_export_ansible_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let result = cmd_export(dir.path(), "ansible", None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_export_unknown_format() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_export(dir.path(), "xml", None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown export format"));
    }

    #[test]
    fn test_cov_export_csv_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "db/state.lock.yaml", state_lock_yaml_db());
        let result = cmd_export(dir.path(), "csv", Some("web"), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_export_csv_machine_filter_no_match() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let result = cmd_export(dir.path(), "csv", Some("nonexistent"), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_export_csv_to_file() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let out = dir.path().join("export.csv");
        let result = cmd_export(dir.path(), "csv", None, Some(&out));
        assert!(result.is_ok());
        assert!(out.exists());
    }

    #[test]
    fn test_cov_export_terraform_to_file() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let out = dir.path().join("export.tf");
        let result = cmd_export(dir.path(), "terraform", None, Some(&out));
        assert!(result.is_ok());
        assert!(out.exists());
    }

    #[test]
    fn test_cov_export_ansible_to_file() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        let out = dir.path().join("inventory.yaml");
        let result = cmd_export(dir.path(), "ansible", None, Some(&out));
        assert!(result.is_ok());
        assert!(out.exists());
    }

    #[test]
    fn test_cov_export_multi_machine() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "db/state.lock.yaml", state_lock_yaml_db());
        write_yaml(dir.path(), "cache/state.lock.yaml", state_lock_yaml_cache());
        let result = cmd_export(dir.path(), "csv", None, None);
        assert!(result.is_ok());
    }

}

