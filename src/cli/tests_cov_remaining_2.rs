//! Tests: Coverage for remaining validate, lock, destroy, observe (part 2).

use super::validate_compliance::*;
use super::validate_resources::*;
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

    fn empty_config() -> &'static str {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n"
    }

    fn basic_config() -> &'static str {
        "version: \"1.0\"\nname: test-project\nmachines:\n  web:\n    hostname: web\n    addr: 127.0.0.1\nresources:\n  app-config:\n    type: file\n    machine: web\n    path: /etc/app.conf\n    content: \"port=8080\"\n    owner: root\n    group: root\n    mode: \"0644\"\n  web-svc:\n    type: service\n    machine: web\n    name: nginx\n    depends_on: [app-config]\n"
    }

    fn config_world_writable() -> &'static str {
        "version: \"1.0\"\nname: insecure\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  insecure-file:\n    type: file\n    machine: m\n    path: /tmp/world\n    content: open\n    owner: root\n    group: root\n    mode: \"0777\"\n"
    }

    fn config_root_tmp() -> &'static str {
        "version: \"1.0\"\nname: root-tmp\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  tmp-file:\n    type: file\n    machine: m\n    path: /tmp/root-owned\n    content: data\n    owner: root\n    group: root\n    mode: \"0644\"\n"
    }

    fn config_hipaa_violation() -> &'static str {
        "version: \"1.0\"\nname: hipaa-test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  phi-file:\n    type: file\n    machine: m\n    path: /etc/phi.conf\n    content: \"patient_data\"\n    mode: \"0644\"\n"
    }

    fn config_soc2_violation() -> &'static str {
        "version: \"1.0\"\nname: soc2-test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  orphan-file:\n    type: file\n    machine: m\n    path: /etc/app.conf\n    content: \"data\"\n"
    }

    fn config_non_portable() -> &'static str {
        "version: \"1.0\"\nname: linux-only\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  proc-file:\n    type: file\n    machine: m\n    path: /proc/sys/net\n    content: \"1\"\n  apt-pkg:\n    type: package\n    machine: m\n    provider: apt\n    packages: [curl]\n"
    }

    fn config_non_idempotent() -> &'static str {
        "version: \"1.0\"\nname: non-idem\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  dyn:\n    type: file\n    machine: m\n    path: /tmp/dyn\n    content: \"timestamp=$(date)\"\n"
    }

    fn config_isolated_resources() -> &'static str {
        "version: \"1.0\"\nname: isolated\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  x:\n    type: file\n    machine: m\n    path: /tmp/x\n    content: x\n  y:\n    type: file\n    machine: m\n    path: /tmp/y\n    content: y\n  z:\n    type: file\n    machine: m\n    path: /tmp/z\n    content: z\n"
    }

    fn config_missing_dep() -> &'static str {
        "version: \"1.0\"\nname: missing-dep\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  svc:\n    type: service\n    machine: m\n    name: nginx\n    depends_on: [nonexistent-resource]\n"
    }

    fn config_mixed_owners() -> &'static str {
        "version: \"1.0\"\nname: mixed\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  file-a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    owner: root\n  file-b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    owner: nobody\n"
    }

    fn config_bad_machine_addr() -> &'static str {
        "version: \"1.0\"\nname: bad-addr\nmachines:\n  m:\n    hostname: m\n    addr: invalid-no-dot\nresources:\n  f:\n    type: file\n    machine: m\n    path: /tmp/f\n    content: hi\n"
    }

    // ── CIS root in /tmp ──

    #[test]
    fn test_cov_check_compliance_cis_root_tmp() {
        let f = write_temp_config(config_root_tmp());
        let result = cmd_validate_check_compliance(f.path(), "CIS", false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 10. validate_compliance: cmd_validate_check_compliance (SOC2)
    // ========================================================================

    #[test]
    fn test_cov_check_compliance_soc2_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_compliance(f.path(), "SOC2", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_compliance_soc2_violation_plain() {
        let f = write_temp_config(config_soc2_violation());
        let result = cmd_validate_check_compliance(f.path(), "SOC2", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_compliance_soc2_violation_json() {
        let f = write_temp_config(config_soc2_violation());
        let result = cmd_validate_check_compliance(f.path(), "SOC2", true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 11. validate_compliance: cmd_validate_check_compliance (HIPAA)
    // ========================================================================

    #[test]
    fn test_cov_check_compliance_hipaa_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_compliance(f.path(), "HIPAA", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_compliance_hipaa_violation_plain() {
        let f = write_temp_config(config_hipaa_violation());
        let result = cmd_validate_check_compliance(f.path(), "HIPAA", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_compliance_hipaa_violation_json() {
        let f = write_temp_config(config_hipaa_violation());
        let result = cmd_validate_check_compliance(f.path(), "HIPAA", true);
        assert!(result.is_ok());
    }

    // ── Unknown policy ──

    #[test]
    fn test_cov_check_compliance_unknown_policy() {
        let f = write_temp_config(basic_config());
        let result = cmd_validate_check_compliance(f.path(), "UNKNOWN", false);
        assert!(result.is_err());
    }

    // ========================================================================
    // 12. validate_compliance: cmd_validate_check_portability
    // ========================================================================

    #[test]
    fn test_cov_check_portability_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_portability(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_portability_non_portable_plain() {
        let f = write_temp_config(config_non_portable());
        let result = cmd_validate_check_portability(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_portability_non_portable_json() {
        let f = write_temp_config(config_non_portable());
        let result = cmd_validate_check_portability(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 13. validate_compliance: cmd_validate_check_idempotency_deep
    // ========================================================================

    #[test]
    fn test_cov_check_idempotency_deep_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_idempotency_deep(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_idempotency_deep_dynamic_plain() {
        let f = write_temp_config(config_non_idempotent());
        let result = cmd_validate_check_idempotency_deep(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_idempotency_deep_dynamic_json() {
        let f = write_temp_config(config_non_idempotent());
        let result = cmd_validate_check_idempotency_deep(f.path(), true);
        assert!(result.is_ok());
    }

    // ── file content without mode triggers suspect ──

    #[test]
    fn test_cov_check_idempotency_deep_no_mode() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: m\n    path: /tmp/f\n    content: data\n";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_idempotency_deep(f.path(), false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 14. validate_resources: cmd_validate_check_resource_limits
    // ========================================================================

    #[test]
    fn test_cov_check_resource_limits_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_resource_limits(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_resource_limits_data_plain() {
        let f = write_temp_config(basic_config());
        let result = cmd_validate_check_resource_limits(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_resource_limits_data_json() {
        let f = write_temp_config(basic_config());
        let result = cmd_validate_check_resource_limits(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 15. validate_resources: cmd_validate_check_unused
    // ========================================================================

    #[test]
    fn test_cov_check_unused_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_unused(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_unused_isolated_plain() {
        let f = write_temp_config(config_isolated_resources());
        let result = cmd_validate_check_unused(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_unused_isolated_json() {
        let f = write_temp_config(config_isolated_resources());
        let result = cmd_validate_check_unused(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 16. validate_resources: cmd_validate_check_dependencies
    // ========================================================================

    #[test]
    fn test_cov_check_dependencies_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_dependencies(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_dependencies_missing_plain() {
        let f = write_temp_config(config_missing_dep());
        let result = cmd_validate_check_dependencies(f.path(), false);
        // Missing dep reference may cause parse_and_validate to fail
        let _ = result;
    }

    #[test]
    fn test_cov_check_dependencies_missing_json() {
        let f = write_temp_config(config_missing_dep());
        let result = cmd_validate_check_dependencies(f.path(), true);
        let _ = result;
    }

    // ========================================================================
    // 17. validate_resources: cmd_validate_check_permissions
    // ========================================================================

    #[test]
    fn test_cov_check_permissions_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_permissions(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_permissions_world_writable_plain() {
        let f = write_temp_config(config_world_writable());
        let result = cmd_validate_check_permissions(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_permissions_world_writable_json() {
        let f = write_temp_config(config_world_writable());
        let result = cmd_validate_check_permissions(f.path(), true);
        assert!(result.is_ok());
    }

    // ── root on non-system path ──

    #[test]
    fn test_cov_check_permissions_root_non_system() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: m\n    path: /home/user/data\n    content: hi\n    owner: root\n    mode: \"0644\"\n";
        let f = write_temp_config(yaml);
        let result = cmd_validate_check_permissions(f.path(), false);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 18. validate_resources: cmd_validate_check_machine_reachability
    // ========================================================================

    #[test]
    fn test_cov_check_machine_reachability_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_machine_reachability(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_machine_reachability_bad_addr_plain() {
        let f = write_temp_config(config_bad_machine_addr());
        let result = cmd_validate_check_machine_reachability(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_machine_reachability_bad_addr_json() {
        let f = write_temp_config(config_bad_machine_addr());
        let result = cmd_validate_check_machine_reachability(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 19. validate_resources: cmd_validate_check_owner_consistency
    // ========================================================================

    #[test]
    fn test_cov_check_owner_consistency_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_owner_consistency(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_owner_consistency_mixed_plain() {
        let f = write_temp_config(config_mixed_owners());
        let result = cmd_validate_check_owner_consistency(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_owner_consistency_mixed_json() {
        let f = write_temp_config(config_mixed_owners());
        let result = cmd_validate_check_owner_consistency(f.path(), true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 20. validate_resources: cmd_validate_check_service_deps
    // ========================================================================

    #[test]
    fn test_cov_check_service_deps_empty() {
        let f = write_temp_config(empty_config());
        let result = cmd_validate_check_service_deps(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_service_deps_missing_plain() {
        let f = write_temp_config(config_missing_dep());
        let result = cmd_validate_check_service_deps(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_check_service_deps_missing_json() {
        let f = write_temp_config(config_missing_dep());
        let result = cmd_validate_check_service_deps(f.path(), true);
        assert!(result.is_ok());
    }
}
