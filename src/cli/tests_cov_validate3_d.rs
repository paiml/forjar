//! Tests: Coverage for validate_governance and validate_paths (part 4 of 4).

#![allow(unused_imports)]
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
    // validate_governance: cmd_validate_check_resource_dependency_depth
    // ======================================================================

    #[test]
    fn test_governance_dep_depth_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_dependency_depth(f.path(), false, 5).is_ok());
    }

    #[test]
    fn test_governance_dep_depth_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_depth(f.path(), false, 5).is_ok());
    }

    #[test]
    fn test_governance_dep_depth_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_dependency_depth(f.path(), true, 5).is_ok());
    }

    // ======================================================================
    // validate_governance: cmd_validate_check_resource_machine_affinity
    // ======================================================================

    #[test]
    fn test_governance_machine_affinity_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_machine_affinity(f.path(), false).is_ok());
    }

    #[test]
    fn test_governance_machine_affinity_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_machine_affinity(f.path(), false).is_ok());
    }

    #[test]
    fn test_governance_machine_affinity_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_machine_affinity(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_governance: cmd_validate_check_resource_drift_risk
    // ======================================================================

    #[test]
    fn test_governance_drift_risk_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_drift_risk(f.path(), false).is_ok());
    }

    #[test]
    fn test_governance_drift_risk_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_drift_risk(f.path(), false).is_ok());
    }

    #[test]
    fn test_governance_drift_risk_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_drift_risk(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_governance: cmd_validate_check_resource_tag_coverage
    // ======================================================================

    #[test]
    fn test_governance_tag_coverage_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_tag_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_governance_tag_coverage_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_tag_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_governance_tag_coverage_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_tag_coverage(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_governance: cmd_validate_check_resource_lifecycle_hooks
    // ======================================================================

    #[test]
    fn test_governance_lifecycle_hooks_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_lifecycle_hooks(f.path(), false).is_ok());
    }

    #[test]
    fn test_governance_lifecycle_hooks_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_lifecycle_hooks(f.path(), false).is_ok());
    }

    #[test]
    fn test_governance_lifecycle_hooks_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_lifecycle_hooks(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_governance: cmd_validate_check_resource_provider_version
    // ======================================================================

    #[test]
    fn test_governance_provider_version_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_provider_version(f.path(), false).is_ok());
    }

    #[test]
    fn test_governance_provider_version_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_provider_version(f.path(), false).is_ok());
    }

    #[test]
    fn test_governance_provider_version_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_provider_version(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_paths: cmd_validate_check_path_conflicts
    // ======================================================================

    #[test]
    fn test_paths_path_conflicts_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_path_conflicts(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_path_conflicts_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_path_conflicts(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_path_conflicts_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_path_conflicts(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_paths: cmd_validate_check_template_vars
    // ======================================================================

    #[test]
    fn test_paths_template_vars_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_template_vars(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_template_vars_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_template_vars(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_template_vars_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_template_vars(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_paths: cmd_validate_check_mode_consistency
    // ======================================================================

    #[test]
    fn test_paths_mode_consistency_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_mode_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_mode_consistency_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_mode_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_mode_consistency_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_mode_consistency(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_paths: cmd_validate_check_group_consistency
    // ======================================================================

    #[test]
    fn test_paths_group_consistency_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_group_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_group_consistency_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_group_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_group_consistency_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_group_consistency(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_paths: cmd_validate_check_mount_points
    // ======================================================================

    #[test]
    fn test_paths_mount_points_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_mount_points(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_mount_points_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_mount_points(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_mount_points_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_mount_points(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_paths: cmd_validate_check_cron_syntax
    // ======================================================================

    #[test]
    fn test_paths_cron_syntax_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_cron_syntax(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_cron_syntax_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_cron_syntax(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_cron_syntax_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_cron_syntax(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_paths: cmd_validate_check_env_refs
    // ======================================================================

    #[test]
    fn test_paths_env_refs_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_env_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_env_refs_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_env_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_env_refs_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_env_refs(f.path(), true).is_ok());
    }

    // ======================================================================
    // validate_paths: cmd_validate_check_resource_names
    // ======================================================================

    #[test]
    fn test_paths_resource_names_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_names(f.path(), false, "kebab-case").is_ok());
    }

    #[test]
    fn test_paths_resource_names_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_names(f.path(), false, "kebab-case").is_ok());
    }

    #[test]
    fn test_paths_resource_names_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_names(f.path(), true, "kebab-case").is_ok());
    }

    // ======================================================================
    // validate_paths: cmd_validate_check_resource_count
    // ======================================================================

    #[test]
    fn test_paths_resource_count_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_resource_count(f.path(), false, 100).is_ok());
    }

    #[test]
    fn test_paths_resource_count_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_count(f.path(), false, 100).is_ok());
    }

    #[test]
    fn test_paths_resource_count_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_resource_count(f.path(), true, 100).is_ok());
    }

    // ======================================================================
    // validate_paths: cmd_validate_check_duplicate_paths
    // ======================================================================

    #[test]
    fn test_paths_duplicate_paths_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_duplicate_paths(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_duplicate_paths_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_duplicate_paths(f.path(), false).is_ok());
    }

    #[test]
    fn test_paths_duplicate_paths_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_duplicate_paths(f.path(), true).is_ok());
    }
}
