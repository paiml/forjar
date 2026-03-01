//! Tests: Coverage for validate_resources, validate_quality (part 6).

#![allow(unused_imports)]
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
    // 10. validate_resources: cmd_validate_check_resource_limits
    // ========================================================================

    #[test]
    fn test_cov_resource_limits_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_limits(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_resource_limits_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_limits(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_resource_limits_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_limits(f.path(), true).is_ok());
    }

    // ========================================================================
    // 11. validate_resources: cmd_validate_check_unused
    // ========================================================================

    #[test]
    fn test_cov_check_unused_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_unused(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_check_unused_basic_plain() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_unused(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_check_unused_basic_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_unused(f.path(), true).is_ok());
    }

    /// Config with isolated resources (no deps) triggers unused detection.
    #[test]
    fn test_cov_check_unused_isolated() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  a:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/a\n",
            "    content: a\n",
            "  b:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/b\n",
            "    content: b\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_unused(f.path(), true).is_ok());
    }

    // ========================================================================
    // 12. validate_resources: cmd_validate_check_dependencies
    // ========================================================================

    #[test]
    fn test_cov_check_deps_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_dependencies(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_check_deps_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_dependencies(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_check_deps_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_dependencies(f.path(), true).is_ok());
    }

    // ========================================================================
    // 13. validate_resources: cmd_validate_check_permissions
    // ========================================================================

    #[test]
    fn test_cov_check_perms_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_permissions(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_check_perms_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_permissions(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_check_perms_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_permissions(f.path(), true).is_ok());
    }

    /// World-writable mode triggers permission issue.
    #[test]
    fn test_cov_check_perms_world_writable() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  bad:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/bad\n",
            "    content: x\n",
            "    mode: \"0777\"\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_permissions(f.path(), false).is_ok());
    }

    /// Root ownership on /tmp triggers permission issue.
    #[test]
    fn test_cov_check_perms_root_tmp() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  tmp-file:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/secret\n",
            "    content: x\n",
            "    owner: root\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_permissions(f.path(), true).is_ok());
    }

    // ========================================================================
    // 14. validate_resources: cmd_validate_check_machine_reachability
    // ========================================================================

    #[test]
    fn test_cov_machine_reach_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_machine_reachability(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_machine_reach_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_machine_reachability(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_machine_reach_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_machine_reachability(f.path(), true).is_ok());
    }

    /// Container addr is valid for reachability.
    #[test]
    fn test_cov_machine_reach_container() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  c:\n",
            "    hostname: c\n",
            "    addr: container\n",
            "resources: {}\n",
        );
        let f = write_temp_config(yaml);
        // parse_and_validate may reject 'container' as invalid addr
        let _ = cmd_validate_check_machine_reachability(f.path(), false);
    }

    // ========================================================================
    // 15. validate_resources: cmd_validate_check_owner_consistency
    // ========================================================================

    #[test]
    fn test_cov_owner_consistency_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_owner_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_owner_consistency_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_owner_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_owner_consistency_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_owner_consistency(f.path(), true).is_ok());
    }

    /// Mixed owners on same machine triggers inconsistency.
    #[test]
    fn test_cov_owner_consistency_mixed() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  a:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/a\n",
            "    content: a\n",
            "    owner: root\n",
            "  b:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/b\n",
            "    content: b\n",
            "    owner: www-data\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_owner_consistency(f.path(), true).is_ok());
    }

    // ========================================================================
    // 16. validate_resources: cmd_validate_check_service_deps
    // ========================================================================

    #[test]
    fn test_cov_service_deps_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_service_deps(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_service_deps_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_service_deps(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_service_deps_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_service_deps(f.path(), true).is_ok());
    }

    // ========================================================================
    // 17. validate_quality: cmd_validate_check_idempotency
    // ========================================================================

    #[test]
    fn test_cov_idempotency_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_idempotency(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_idempotency_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_idempotency(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_idempotency_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_idempotency(f.path(), true).is_ok());
    }

    // ========================================================================
    // 18. validate_quality: cmd_validate_check_drift_coverage
    // ========================================================================

    #[test]
    fn test_cov_drift_coverage_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_drift_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_drift_coverage_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_drift_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_drift_coverage_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_drift_coverage(f.path(), true).is_ok());
    }

    // ========================================================================
    // 19. validate_quality: cmd_validate_check_complexity
    // ========================================================================

    #[test]
    fn test_cov_complexity_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_complexity(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_complexity_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_complexity(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_complexity_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_complexity(f.path(), true).is_ok());
    }

    /// High fan-out triggers complexity warning.
    #[test]
    fn test_cov_complexity_high_fanout() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  d1:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/d1\n",
            "    content: d1\n",
            "  d2:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/d2\n",
            "    content: d2\n",
            "  d3:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/d3\n",
            "    content: d3\n",
            "  d4:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/d4\n",
            "    content: d4\n",
            "  d5:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/d5\n",
            "    content: d5\n",
            "  hub:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/hub\n",
            "    content: hub\n",
            "    depends_on: [d1, d2, d3, d4, d5]\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_complexity(f.path(), false).is_ok());
    }

    /// High fan-in triggers complexity warning.
    #[test]
    fn test_cov_complexity_high_fanin() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  core:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/core\n",
            "    content: core\n",
            "  a1:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/a1\n",
            "    content: a1\n",
            "    depends_on: [core]\n",
            "  a2:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/a2\n",
            "    content: a2\n",
            "    depends_on: [core]\n",
            "  a3:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/a3\n",
            "    content: a3\n",
            "    depends_on: [core]\n",
            "  a4:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/a4\n",
            "    content: a4\n",
            "    depends_on: [core]\n",
            "  a5:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/a5\n",
            "    content: a5\n",
            "    depends_on: [core]\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_complexity(f.path(), true).is_ok());
    }
}
