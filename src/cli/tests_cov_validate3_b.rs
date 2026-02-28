//! Tests: Coverage for validate_ownership (part 2 of 4).

use super::validate_ordering::*;
use super::validate_ownership::*;
use super::validate_safety::*;
use super::validate_advanced::*;
use super::validate_governance::*;
use super::validate_paths::*;
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
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n".to_string()
    }

    fn basic_config() -> String {
        concat!(
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
            "    content: hello\n",
            "    owner: root\n",
            "    group: root\n",
            "    mode: \"0644\"\n",
            "    tags: [app]\n",
            "  b:\n",
            "    type: service\n",
            "    machine: m\n",
            "    name: nginx\n",
            "    depends_on: [a]\n",
            "  c:\n",
            "    type: package\n",
            "    machine: m\n",
            "    provider: apt\n",
            "    packages: [curl]\n",
        )
        .to_string()
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_naming_convention
    // ======================================================================

    #[test]
    fn test_ownership_naming_convention_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_naming_convention(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_naming_convention_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_naming_convention(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_naming_convention_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_naming_convention(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_idempotency
    // ======================================================================

    #[test]
    fn test_ownership_idempotency_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_idempotency(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_idempotency_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_idempotency(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_idempotency_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_idempotency(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_documentation
    // ======================================================================

    #[test]
    fn test_ownership_documentation_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_documentation(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_documentation_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_documentation(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_documentation_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_documentation(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_ownership
    // ======================================================================

    #[test]
    fn test_ownership_ownership_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_ownership(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_ownership_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_ownership(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_ownership_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_ownership(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_secret_exposure
    // ======================================================================

    #[test]
    fn test_ownership_secret_exposure_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_secret_exposure(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_secret_exposure_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_secret_exposure(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_secret_exposure_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_secret_exposure(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_tag_standards
    // ======================================================================

    #[test]
    fn test_ownership_tag_standards_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_tag_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_tag_standards_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_tag_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_tag_standards_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_tag_standards(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_privilege_escalation
    // ======================================================================

    #[test]
    fn test_ownership_privilege_escalation_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_privilege_escalation_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_privilege_escalation_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_update_safety
    // ======================================================================

    #[test]
    fn test_ownership_update_safety_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_update_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_update_safety_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_update_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_update_safety_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_update_safety(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_cross_machine_consistency
    // ======================================================================

    #[test]
    fn test_ownership_cross_machine_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_cross_machine_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_cross_machine_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_cross_machine_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_cross_machine_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_cross_machine_consistency(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_version_pinning
    // ======================================================================

    #[test]
    fn test_ownership_version_pinning_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_version_pinning(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_version_pinning_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_version_pinning(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_version_pinning_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_version_pinning(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_dependency_completeness
    // ======================================================================

    #[test]
    fn test_ownership_dep_completeness_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_dependency_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_dep_completeness_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_dep_completeness_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_completeness(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_state_coverage
    // ======================================================================

    #[test]
    fn test_ownership_state_coverage_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_state_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_state_coverage_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_state_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_state_coverage_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_state_coverage(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_rollback_safety
    // ======================================================================

    #[test]
    fn test_ownership_rollback_safety_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_rollback_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_rollback_safety_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_rollback_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_rollback_safety_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_rollback_safety(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ownership: cmd_validate_check_resource_config_maturity
    // ======================================================================

    #[test]
    fn test_ownership_config_maturity_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_config_maturity(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_config_maturity_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_config_maturity(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_config_maturity_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_config_maturity(f.path(), true).is_ok());
    }
}
