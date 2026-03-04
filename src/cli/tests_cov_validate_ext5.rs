//! Coverage tests for validate_ordering_ext.rs, validate_paths_b.rs, validate_ownership_b.rs.

#![allow(unused_imports)]
use super::validate_ordering_ext::*;
use super::validate_paths_b::*;
use super::validate_ownership_b::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const CFG: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - web\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n    state: present\n    depends_on:\n      - pkg\n    tags:\n      - web\n";

    // validate_ordering_ext
    #[test]
    fn test_naming_convention_strict() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_naming_convention_strict(f.path(), false).is_ok());
    }
    #[test]
    fn test_naming_convention_strict_json() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_naming_convention_strict(f.path(), true).is_ok());
    }
    #[test]
    fn test_idempotency_annotations() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_idempotency_annotations(f.path(), false).is_ok());
    }
    #[test]
    fn test_idempotency_annotations_json() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_idempotency_annotations(f.path(), true).is_ok());
    }
    #[test]
    fn test_content_size_limit() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_content_size_limit(f.path(), false).is_ok());
    }
    #[test]
    fn test_dependency_fan_limit() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_dependency_fan_limit(f.path(), false).is_ok());
    }
    #[test]
    fn test_gpu_backend_consistency() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_gpu_backend_consistency(f.path(), false).is_ok());
    }
    #[test]
    fn test_when_condition_syntax() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_when_condition_syntax(f.path(), false).is_ok());
    }

    // validate_paths_b
    #[test]
    fn test_cron_syntax() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_cron_syntax(f.path(), false).is_ok());
    }
    #[test]
    fn test_cron_syntax_json() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_cron_syntax(f.path(), true).is_ok());
    }
    #[test]
    fn test_env_refs() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_env_refs(f.path(), false).is_ok());
    }
    #[test]
    fn test_resource_names_match() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_names(f.path(), false, "^[a-z]").is_ok());
    }
    #[test]
    fn test_resource_count() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_count(f.path(), false, 100).is_ok());
    }
    #[test]
    fn test_duplicate_paths() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_duplicate_paths(f.path(), false).is_ok());
    }

    // validate_ownership_b
    #[test]
    fn test_update_safety() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_update_safety(f.path(), false).is_ok());
    }
    #[test]
    fn test_update_safety_json() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_update_safety(f.path(), true).is_ok());
    }
    #[test]
    fn test_cross_machine_consistency() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_cross_machine_consistency(f.path(), false).is_ok());
    }
    #[test]
    fn test_version_pinning() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_version_pinning(f.path(), false).is_ok());
    }
    #[test]
    fn test_dep_completeness() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_dependency_completeness(f.path(), false).is_ok());
    }
    #[test]
    fn test_state_coverage() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_state_coverage(f.path(), false).is_ok());
    }
    #[test]
    fn test_rollback_safety() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_rollback_safety(f.path(), false).is_ok());
    }
    #[test]
    fn test_config_maturity() {
        let f = write_cfg(CFG);
        assert!(cmd_validate_check_resource_config_maturity(f.path(), false).is_ok());
    }
}
