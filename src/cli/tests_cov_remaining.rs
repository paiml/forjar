//! Tests: Coverage for remaining validate, lock, destroy, observe (part 1).

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

    // ── Configs ──

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

    /// Config with secrets embedded (password, api_key, token).
    fn config_with_secrets() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: secret-test\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  db-cfg:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /etc/db.conf\n",
            "    content: \"password: s3cr3t\\napi_key: AKIAIOSFODNN7EXAMPLE\\ntoken: ghp_abc123\"\n",
        )
        .to_string()
    }

    /// Config with bad naming (uppercase, consecutive hyphens, leading hyphen).
    fn config_bad_naming() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: bad-name\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  MyResource:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/x\n",
            "    content: hi\n",
        )
        .to_string()
    }

    /// Config with overlapping paths on same machine.
    fn config_overlapping_paths() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: overlap-test\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  a:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/shared\n",
            "    content: a\n",
            "  b:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/shared\n",
            "    content: b\n",
        )
        .to_string()
    }

    /// Config with unresolved template params.
    fn config_unresolved_template() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: template-test\n",
            "params:\n",
            "  port: 8080\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  cfg:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/{{params.missing_var}}\n",
            "    content: \"host={{params.undefined_host}}\"\n",
        )
        .to_string()
    }

    /// Config with world-writable mode (CIS violation).
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

    // ========================================================================
    // 1. validate_structural: cmd_validate_check_templates
    // ========================================================================

    #[test]
    fn test_cov_check_templates_empty_config() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_templates(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_templates_unresolved_plain() {
        let f = write_temp_config(&config_unresolved_template());
        let result = cmd_validate_check_templates(f.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_check_templates_unresolved_json() {
        let f = write_temp_config(&config_unresolved_template());
        let result = cmd_validate_check_templates(f.path(), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_check_templates_machine_ref_valid() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  web:\n    hostname: web\n    addr: 10.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: web\n    path: /tmp/test\n    content: \"addr={{machine.web.addr}}\"\n";
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_templates(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_check_templates_machine_ref_invalid() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  web:\n    hostname: web\n    addr: 10.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: web\n    path: /tmp/test\n    content: \"addr={{machine.nonexistent.addr}}\"\n";
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_templates(f.path(), false).is_err());
    }

    #[test]
    fn test_cov_check_templates_multiple_fields() {
        let yaml = "version: \"1.0\"\nname: t\nparams:\n  app: myapp\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m\n    path: /opt/{{params.app}}/config\n    content: hello\n    owner: root\n";
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_templates(f.path(), false).is_ok());
    }

    // ========================================================================
    // 2. validate_structural: cmd_validate_check_secrets
    // ========================================================================

    #[test]
    fn test_cov_check_secrets_empty_config() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_secrets(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_secrets_with_secrets_plain() {
        let f = write_temp_config(&config_with_secrets());
        let result = cmd_validate_check_secrets(f.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_check_secrets_with_secrets_json() {
        let f = write_temp_config(&config_with_secrets());
        let result = cmd_validate_check_secrets(f.path(), true);
        assert!(result.is_err());
    }

    // ========================================================================
    // 3. validate_structural: cmd_validate_check_naming
    // ========================================================================

    #[test]
    fn test_cov_check_naming_empty_config() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_naming(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_naming_bad_plain() {
        let f = write_temp_config(&config_bad_naming());
        let result = cmd_validate_check_naming(f.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_check_naming_bad_json() {
        let f = write_temp_config(&config_bad_naming());
        let result = cmd_validate_check_naming(f.path(), true);
        assert!(result.is_err());
    }

    // ========================================================================
    // 4. validate_structural: cmd_validate_check_overlaps
    // ========================================================================

    #[test]
    fn test_cov_check_overlaps_empty_config() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_overlaps(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_overlaps_with_overlaps_plain() {
        let f = write_temp_config(&config_overlapping_paths());
        let result = cmd_validate_check_overlaps(f.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_check_overlaps_with_overlaps_json() {
        let f = write_temp_config(&config_overlapping_paths());
        let result = cmd_validate_check_overlaps(f.path(), true);
        assert!(result.is_err());
    }

    // ========================================================================
    // 5. validate_structural: cmd_validate_check_limits
    // ========================================================================

    #[test]
    fn test_cov_check_limits_empty_config() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_limits(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_limits_basic_plain() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_limits(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_limits_basic_json() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_limits(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 6. validate_structural: cmd_validate_check_circular_refs
    // ========================================================================

    #[test]
    fn test_cov_check_circular_refs_empty() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_circular_refs(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_circular_refs_data_plain() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_circular_refs(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_circular_refs_data_json() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_circular_refs(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 7. validate_structural: cmd_validate_check_naming_conventions
    // ========================================================================

    #[test]
    fn test_cov_check_naming_conventions_empty() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_naming_conventions(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_naming_conventions_bad_plain() {
        let f = write_temp_config(&config_bad_naming());
        let result = cmd_validate_check_naming_conventions(f.path(), false);
        assert!(result.is_ok()); // returns Ok, just prints warnings
    }

    #[test]
    fn test_cov_check_naming_conventions_bad_json() {
        let f = write_temp_config(&config_bad_naming());
        let result = cmd_validate_check_naming_conventions(f.path(), true);
        assert!(result.is_ok());
    }

    // ── Config with leading/trailing hyphen naming issues ──

    #[test]
    fn test_cov_check_naming_conventions_hyphen_edges() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  -leading:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/x\n",
            "    content: hi\n",
        );
        let f = write_temp_config(yaml);
        // May fail at parse, but we cover the code path
        let _ = cmd_validate_check_naming_conventions(f.path(), false);
    }

    // ========================================================================
    // 8. validate_compliance: cmd_validate_check_drift_risk
    // ========================================================================

    #[test]
    fn test_cov_check_drift_risk_empty() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_drift_risk(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_drift_risk_file_content_plain() {
        // file with content triggers drift risk
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_drift_risk(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_drift_risk_file_content_json() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_drift_risk(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 9. validate_compliance: cmd_validate_check_compliance (CIS)
    // ========================================================================

    #[test]
    fn test_cov_check_compliance_cis_empty() {
        let f = write_temp_config(&empty_config());
        let result = cmd_validate_check_compliance(f.path(), "CIS", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_compliance_cis_world_writable_plain() {
        let f = write_temp_config(&config_world_writable());
        let result = cmd_validate_check_compliance(f.path(), "CIS", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_compliance_cis_world_writable_json() {
        let f = write_temp_config(&config_world_writable());
        let result = cmd_validate_check_compliance(f.path(), "CIS", true);
        assert!(result.is_ok());
    }
}
