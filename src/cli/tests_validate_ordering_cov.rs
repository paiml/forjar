//! Coverage tests for validate_ordering_b.rs — FJ-953→FJ-985.

#![allow(unused_imports)]
use super::validate_ordering_b::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const BASE: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n";

    // FJ-961: dependency refs
    #[test]
    fn test_dep_refs_valid() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - b\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"));
        assert!(cmd_validate_check_resource_dependency_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_dep_refs_missing() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - ghost\n"));
        assert!(cmd_validate_check_resource_dependency_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_dep_refs_missing_trigger() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    triggers:\n      - ghost\n"));
        assert!(cmd_validate_check_resource_dependency_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_dep_refs_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - ghost\n"));
        assert!(cmd_validate_check_resource_dependency_refs(f.path(), true).is_ok());
    }

    // FJ-965: trigger refs
    #[test]
    fn test_trigger_refs_valid() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    triggers:\n      - b\n  b:\n    machine: m1\n    type: service\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_trigger_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_trigger_refs_invalid() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    triggers:\n      - nonexistent\n"));
        assert!(cmd_validate_check_resource_trigger_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_trigger_refs_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    triggers:\n      - ghost\n"));
        assert!(cmd_validate_check_resource_trigger_refs(f.path(), true).is_ok());
    }

    // FJ-969: param type safety
    #[test]
    fn test_param_type_safety_ok() {
        let f = write_cfg(&format!("{BASE}params:\n  http_port: 8080\n  app_path: /opt/app\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_param_type_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_param_type_safety_bad_port() {
        let f = write_cfg(&format!("{BASE}params:\n  http_port: not_a_number\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_param_type_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_param_type_safety_bad_path() {
        let f = write_cfg(&format!("{BASE}params:\n  data_dir: relative_path\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_param_type_safety(f.path(), false).is_ok());
    }

    #[test]
    fn test_param_type_safety_json() {
        let f = write_cfg(&format!("{BASE}params:\n  http_port: abc\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_param_type_safety(f.path(), true).is_ok());
    }

    // FJ-953: machine balance
    #[test]
    fn test_machine_balance_balanced() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n  m2:\n    hostname: m2\n    addr: 5.6.7.8\nresources:\n  a:\n    machine: m1\n    type: package\n    name: nginx\n  b:\n    machine: m2\n    type: package\n    name: curl\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_resource_machine_balance(f.path(), false).is_ok());
    }

    #[test]
    fn test_machine_balance_imbalanced() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n  m2:\n    hostname: m2\n    addr: 5.6.7.8\nresources:\n  a:\n    machine: m1\n    type: package\n    name: nginx\n  b:\n    machine: m1\n    type: package\n    name: curl\n  c:\n    machine: m1\n    type: package\n    name: vim\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_resource_machine_balance(f.path(), false).is_ok());
    }

    #[test]
    fn test_machine_balance_json() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  a:\n    machine: m1\n    type: package\n    name: nginx\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_resource_machine_balance(f.path(), true).is_ok());
    }

    // FJ-973: env consistency
    #[test]
    fn test_env_consistency_ok() {
        let f = write_cfg(&format!("{BASE}params:\n  port: 8080\nresources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"port={{{{port}}}}\"\n"));
        assert!(cmd_validate_check_resource_env_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_env_consistency_undeclared() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"port={{{{undeclared}}}}\"\n"));
        assert!(cmd_validate_check_resource_env_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_env_consistency_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"{{{{missing}}}}\"\n"));
        assert!(cmd_validate_check_resource_env_consistency(f.path(), true).is_ok());
    }

    // FJ-977: secret rotation
    #[test]
    fn test_secret_rotation_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  db-secret:\n    machine: m1\n    type: file\n    path: /etc/secret\n    content: s3cr3t\n    tags:\n      - rotate-90d\n"));
        assert!(cmd_validate_check_resource_secret_rotation(f.path(), false).is_ok());
    }

    #[test]
    fn test_secret_rotation_missing_tags() {
        let f = write_cfg(&format!("{BASE}resources:\n  db-secret:\n    machine: m1\n    type: file\n    path: /etc/secret\n    content: s3cr3t\n"));
        assert!(cmd_validate_check_resource_secret_rotation(f.path(), false).is_ok());
    }

    #[test]
    fn test_secret_rotation_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  api-token:\n    machine: m1\n    type: file\n    path: /etc/token\n    content: tok\n"));
        assert!(cmd_validate_check_resource_secret_rotation(f.path(), true).is_ok());
    }

    // FJ-981: lifecycle completeness
    #[test]
    fn test_lifecycle_complete() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: hi\n    tags:\n      - web\n"));
        assert!(cmd_validate_check_resource_lifecycle_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_lifecycle_incomplete() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_lifecycle_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_lifecycle_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_lifecycle_completeness(f.path(), true).is_ok());
    }

    // FJ-985: provider compatibility
    #[test]
    fn test_provider_compat_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_provider_compatibility(f.path(), false).is_ok());
    }

    #[test]
    fn test_provider_compat_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_provider_compatibility(f.path(), true).is_ok());
    }
}
