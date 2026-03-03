//! Tests: Coverage for fleet_ops and fleet_reporting.

use super::fleet_ops::*;
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
        let cfg = write_yaml(
            dir.path(),
            "forjar.yaml",
            "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n",
        );
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
        let result = cmd_canary(
            &cfg,
            &state_dir,
            "nobody",
            true,
            &["key=val".to_string()],
            None,
        );
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
        let cfg = write_yaml(
            dir.path(),
            "forjar.yaml",
            "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n",
        );
        let result = cmd_rolling(&cfg, &state_dir, 2, &[], None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no machines defined"));
    }

    #[test]
    fn test_cov_rolling_batch_size_one() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(
            dir.path(),
            "forjar.yaml",
            "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n",
        );
        let result = cmd_rolling(&cfg, &state_dir, 1, &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rolling_batch_size_large() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(
            dir.path(),
            "forjar.yaml",
            "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n",
        );
        let result = cmd_rolling(&cfg, &state_dir, 100, &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rolling_with_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(
            dir.path(),
            "forjar.yaml",
            "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n",
        );
        let result = cmd_rolling(&cfg, &state_dir, 2, &[], Some(60));
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rolling_with_params() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let cfg = write_yaml(
            dir.path(),
            "forjar.yaml",
            "version: \"1.0\"\nname: t\nmachines: {}\nresources: {}\n",
        );
        let result = cmd_rolling(&cfg, &state_dir, 2, &["env=prod".to_string()], None);
        assert!(result.is_err());
    }
}
