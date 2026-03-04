//! Tests: Coverage for validate_core, validate_resources, lock_repair, lock_core, lock_audit, history, diff_cmd.

#![allow(unused_imports)]
#![allow(dead_code)]
use super::diff_cmd::*;
use super::history::*;
use super::lock_audit::*;
use super::lock_core::*;
use super::lock_repair::*;
use super::validate_core::*;
use super::validate_resources::*;
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

    /// Config with unused params and template references.
    fn config_with_params() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: param-test\n",
            "description: Param test\n",
            "params:\n",
            "  port: 8080\n",
            "  unused_flag: true\n",
            "machines:\n",
            "  web:\n",
            "    hostname: web\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  cfg:\n",
            "    type: file\n",
            "    machine: web\n",
            "    path: /etc/app.conf\n",
            "    content: \"port={{params.port}}\"\n",
        )
        .to_string()
    }

    /// Config with an unknown machine reference.
    fn config_bad_machine_ref() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: bad-machine\n",
            "description: bad\n",
            "machines:\n",
            "  web:\n",
            "    hostname: web\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  cfg:\n",
            "    type: file\n",
            "    machine: ghost\n",
            "    path: /tmp/x\n",
            "    content: hi\n",
        )
        .to_string()
    }

    /// Config with an unknown dependency reference.
    fn config_bad_dep_ref() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: bad-dep\n",
            "description: bad\n",
            "machines:\n",
            "  web:\n",
            "    hostname: web\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  cfg:\n",
            "    type: file\n",
            "    machine: web\n",
            "    path: /tmp/x\n",
            "    content: hi\n",
            "    depends_on: [nonexistent]\n",
        )
        .to_string()
    }

    /// Config with unresolved content param.
    fn config_unresolved_param() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: unresolved\n",
            "description: test\n",
            "params:\n",
            "  port: 8080\n",
            "machines:\n",
            "  web:\n",
            "    hostname: web\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  cfg:\n",
            "    type: file\n",
            "    machine: web\n",
            "    path: /tmp/x\n",
            "    content: \"host={{params.missing_host}}\"\n",
        )
        .to_string()
    }

    /// Config with multiple isolated resources (all unused in dependency chains).
    fn config_many_unused() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: unused-test\n",
            "machines:\n",
            "  web:\n",
            "    hostname: web\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  a:\n",
            "    type: file\n",
            "    machine: web\n",
            "    path: /tmp/a\n",
            "    content: a\n",
            "  b:\n",
            "    type: file\n",
            "    machine: web\n",
            "    path: /tmp/b\n",
            "    content: b\n",
            "  c:\n",
            "    type: file\n",
            "    machine: web\n",
            "    path: /tmp/c\n",
            "    content: c\n",
        )
        .to_string()
    }

    /// A valid state lock YAML string.
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

    /// Create a temp state dir with both nested (discover_machines) and flat lock files.
    fn make_state_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        // Nested structure: {machine}/state.lock.yaml (for discover_machines)
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        // Flat structure: {machine}.lock.yaml (for lock_repair / lock_normalize)
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        dir
    }

    /// Create a state dir with a corrupted flat lock file.
    fn make_state_dir_corrupted() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "web/state.lock.yaml", "NOT VALID YAML: {{{{");
        dir
    }

    /// Create a state dir with events subdirectory containing JSONL files.
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

    /// Create a state dir with snapshots subdirectory.
    fn make_state_dir_with_snapshots() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        write_yaml(dir.path(), "snapshots/2026-02-01.yaml", state_lock_yaml());
        write_yaml(dir.path(), "snapshots/2026-02-15.yaml", state_lock_yaml());
        dir
    }

    // ========================================================================
    // 1. validate_core::cmd_validate_exhaustive
    // ========================================================================

    #[test]
    fn test_validate_exhaustive_clean_config_text() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_exhaustive(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_exhaustive_clean_config_json() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_exhaustive(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_exhaustive_with_params_text() {
        let f = write_temp_config(&config_with_params());
        // unused_flag param => issues found
        let result = cmd_validate_exhaustive(f.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_exhaustive_with_params_json() {
        let f = write_temp_config(&config_with_params());
        let result = cmd_validate_exhaustive(f.path(), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_exhaustive_bad_machine_ref_text() {
        let f = write_temp_config(&config_bad_machine_ref());
        let result = cmd_validate_exhaustive(f.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_exhaustive_bad_machine_ref_json() {
        let f = write_temp_config(&config_bad_machine_ref());
        let result = cmd_validate_exhaustive(f.path(), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_exhaustive_bad_dep_ref_text() {
        let f = write_temp_config(&config_bad_dep_ref());
        let result = cmd_validate_exhaustive(f.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_exhaustive_unresolved_param_text() {
        let f = write_temp_config(&config_unresolved_param());
        let result = cmd_validate_exhaustive(f.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_exhaustive_unresolved_param_json() {
        let f = write_temp_config(&config_unresolved_param());
        let result = cmd_validate_exhaustive(f.path(), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_exhaustive_invalid_yaml() {
        let f = write_temp_config("NOT VALID");
        let result = cmd_validate_exhaustive(f.path(), false);
        assert!(result.is_err());
    }

    // ========================================================================
    // 2. validate_resources::cmd_validate_check_unused
    // ========================================================================

    #[test]
    fn test_validate_check_unused_with_deps_text() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_unused(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_check_unused_with_deps_json() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_unused(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_check_unused_many_unused_text() {
        let f = write_temp_config(&config_many_unused());
        let result = cmd_validate_check_unused(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_check_unused_many_unused_json() {
        let f = write_temp_config(&config_many_unused());
        let result = cmd_validate_check_unused(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_check_unused_single_resource_text() {
        let config = concat!(
            "version: \"1.0\"\n",
            "name: single\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  only:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/only\n",
            "    content: x\n",
        );
        let f = write_temp_config(config);
        let result = cmd_validate_check_unused(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_check_unused_single_resource_json() {
        let config = concat!(
            "version: \"1.0\"\n",
            "name: single\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  only:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/only\n",
            "    content: x\n",
        );
        let f = write_temp_config(config);
        let result = cmd_validate_check_unused(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_check_unused_invalid_yaml() {
        let f = write_temp_config("{{broken");
        let result = cmd_validate_check_unused(f.path(), false);
        assert!(result.is_err());
    }
}
