//! Tests: Coverage for validate_safety and validate_advanced (part 3 of 4).

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
    // validate_safety: cmd_validate_check_circular_deps
    // ======================================================================

    #[test]
    fn test_safety_circular_deps_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_circular_deps(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_circular_deps_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_circular_deps(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_circular_deps_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_circular_deps(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_safety: cmd_validate_check_machine_refs
    // ======================================================================

    #[test]
    fn test_safety_machine_refs_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_machine_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_machine_refs_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_machine_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_machine_refs_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_machine_refs(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_safety: cmd_validate_check_provider_consistency
    // ======================================================================

    #[test]
    fn test_safety_provider_consistency_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_provider_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_provider_consistency_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_provider_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_provider_consistency_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_provider_consistency(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_safety: cmd_validate_check_state_values
    // ======================================================================

    #[test]
    fn test_safety_state_values_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_state_values(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_state_values_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_state_values(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_state_values_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_state_values(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_safety: cmd_validate_check_unused_machines
    // ======================================================================

    #[test]
    fn test_safety_unused_machines_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_unused_machines(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_unused_machines_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_unused_machines(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_unused_machines_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_unused_machines(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_safety: cmd_validate_check_tag_consistency
    // ======================================================================

    #[test]
    fn test_safety_tag_consistency_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_tag_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_tag_consistency_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_tag_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_tag_consistency_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_tag_consistency(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_safety: cmd_validate_check_dependency_exists
    // ======================================================================

    #[test]
    fn test_safety_dependency_exists_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_dependency_exists(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_dependency_exists_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_dependency_exists(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_dependency_exists_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_dependency_exists(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_safety: cmd_validate_check_path_conflicts_strict
    // ======================================================================

    #[test]
    fn test_safety_path_conflicts_strict_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_path_conflicts_strict(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_path_conflicts_strict_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_path_conflicts_strict(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_path_conflicts_strict_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_path_conflicts_strict(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_safety: cmd_validate_check_duplicate_names
    // ======================================================================

    #[test]
    fn test_safety_duplicate_names_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_duplicate_names(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_duplicate_names_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_duplicate_names(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_duplicate_names_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_duplicate_names(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_safety: cmd_validate_check_resource_groups
    // ======================================================================

    #[test]
    fn test_safety_resource_groups_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_groups(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_resource_groups_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_groups(f.path(), false).is_ok());
    }

    #[test]
    fn test_safety_resource_groups_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_groups(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_advanced: cmd_validate_check_orphan_resources
    // ======================================================================

    #[test]
    fn test_advanced_orphan_resources_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_orphan_resources(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_orphan_resources_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_orphan_resources(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_orphan_resources_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_orphan_resources(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_advanced: cmd_validate_check_machine_arch
    // ======================================================================

    #[test]
    fn test_advanced_machine_arch_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_machine_arch(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_machine_arch_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_machine_arch(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_machine_arch_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_machine_arch(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_advanced: cmd_validate_check_resource_health_conflicts
    // ======================================================================

    #[test]
    fn test_advanced_health_conflicts_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_health_conflicts(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_health_conflicts_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_health_conflicts(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_health_conflicts_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_health_conflicts(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_advanced: cmd_validate_check_resource_overlap
    // ======================================================================

    #[test]
    fn test_advanced_resource_overlap_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_overlap(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_resource_overlap_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_overlap(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_resource_overlap_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_overlap(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_advanced: cmd_validate_check_resource_tags
    // ======================================================================

    #[test]
    fn test_advanced_resource_tags_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_tags(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_resource_tags_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_tags(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_resource_tags_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_tags(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_advanced: cmd_validate_check_resource_state_consistency
    // ======================================================================

    #[test]
    fn test_advanced_state_consistency_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_state_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_state_consistency_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_state_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_state_consistency_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_state_consistency(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_advanced: cmd_validate_check_resource_dependencies_complete
    // ======================================================================

    #[test]
    fn test_advanced_deps_complete_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_dependencies_complete(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_deps_complete_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependencies_complete(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_deps_complete_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependencies_complete(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_advanced: cmd_validate_check_machine_connectivity
    // ======================================================================

    #[test]
    fn test_advanced_machine_connectivity_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_machine_connectivity(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_machine_connectivity_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_machine_connectivity(f.path(), false).is_ok());
    }

    #[test]
    fn test_advanced_machine_connectivity_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_machine_connectivity(f.path(), true).is_ok());
    }
}
