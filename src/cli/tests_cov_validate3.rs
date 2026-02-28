//! Tests: Coverage for validate_ordering (part 1 of 4).

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
    // validate_ordering: cmd_validate_check_resource_dependency_ordering
    // ======================================================================

    #[test]
    fn test_ordering_dep_ordering_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_dependency_ordering(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_dep_ordering_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_ordering(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_dep_ordering_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_ordering(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_tag_completeness
    // ======================================================================

    #[test]
    fn test_ordering_tag_completeness_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_tag_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_tag_completeness_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_tag_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_tag_completeness_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_tag_completeness(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_naming_standards
    // ======================================================================

    #[test]
    fn test_ordering_naming_standards_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_naming_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_naming_standards_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_naming_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_naming_standards_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_naming_standards(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_dependency_symmetry
    // ======================================================================

    #[test]
    fn test_ordering_dep_symmetry_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_dependency_symmetry(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_dep_symmetry_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_symmetry(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_dep_symmetry_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_symmetry(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_circular_alias
    // ======================================================================

    #[test]
    fn test_ordering_circular_alias_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_circular_alias(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_circular_alias_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_circular_alias(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_circular_alias_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_circular_alias(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_dependency_depth_limit
    // ======================================================================

    #[test]
    fn test_ordering_dep_depth_limit_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_dependency_depth_limit(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_dep_depth_limit_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_depth_limit(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_dep_depth_limit_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_depth_limit(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_unused_params
    // ======================================================================

    #[test]
    fn test_ordering_unused_params_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_unused_params(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_unused_params_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_unused_params(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_unused_params_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_unused_params(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_content_hash_consistency
    // ======================================================================

    #[test]
    fn test_ordering_content_hash_consistency_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_content_hash_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_content_hash_consistency_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_content_hash_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_content_hash_consistency_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_content_hash_consistency(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_dependency_refs
    // ======================================================================

    #[test]
    fn test_ordering_dep_refs_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_dependency_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_dep_refs_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_dep_refs_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_refs(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_trigger_refs
    // ======================================================================

    #[test]
    fn test_ordering_trigger_refs_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_trigger_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_trigger_refs_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_trigger_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_trigger_refs_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_trigger_refs(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_param_type_safety
    // ======================================================================

    #[test]
    fn test_ordering_param_type_safety_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_param_type_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_param_type_safety_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_param_type_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_param_type_safety_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_param_type_safety(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_machine_balance
    // ======================================================================

    #[test]
    fn test_ordering_machine_balance_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_machine_balance(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_machine_balance_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_machine_balance(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_machine_balance_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_machine_balance(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_env_consistency
    // ======================================================================

    #[test]
    fn test_ordering_env_consistency_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_env_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_env_consistency_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_env_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_env_consistency_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_env_consistency(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_secret_rotation
    // ======================================================================

    #[test]
    fn test_ordering_secret_rotation_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_secret_rotation(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_secret_rotation_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_secret_rotation(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_secret_rotation_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_secret_rotation(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_lifecycle_completeness
    // ======================================================================

    #[test]
    fn test_ordering_lifecycle_completeness_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_lifecycle_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_lifecycle_completeness_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_lifecycle_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_lifecycle_completeness_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_lifecycle_completeness(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_ordering: cmd_validate_check_resource_provider_compatibility
    // ======================================================================

    #[test]
    fn test_ordering_provider_compat_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_provider_compatibility(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_provider_compat_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_provider_compatibility(f.path(), false).is_ok());
    }

    #[test]
    fn test_ordering_provider_compat_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_provider_compatibility(f.path(), true).is_ok());
    }
}
