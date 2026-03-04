//! Coverage tests for validate_governance.rs — FJ-829→FJ-865.

#![allow(unused_imports)]
use super::validate_governance::*;
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

    // FJ-829: naming pattern
    #[test]
    fn test_naming_pattern_match() {
        let f = write_cfg(&format!("{BASE}resources:\n  web-nginx:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_naming_pattern(f.path(), false, "web").is_ok());
    }

    #[test]
    fn test_naming_pattern_no_match() {
        let f = write_cfg(&format!("{BASE}resources:\n  db-postgres:\n    machine: m1\n    type: package\n    name: pg\n"));
        assert!(cmd_validate_check_resource_naming_pattern(f.path(), false, "web").is_ok());
    }

    #[test]
    fn test_naming_pattern_prefix() {
        let f = write_cfg(&format!("{BASE}resources:\n  web-nginx:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_naming_pattern(f.path(), false, "^web").is_ok());
    }

    #[test]
    fn test_naming_pattern_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  db-pg:\n    machine: m1\n    type: package\n    name: pg\n"));
        assert!(cmd_validate_check_resource_naming_pattern(f.path(), true, "web").is_ok());
    }

    // FJ-833: provider support
    #[test]
    fn test_provider_support_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    provider: apt\n"));
        assert!(cmd_validate_check_resource_provider_support(f.path(), false).is_ok());
    }

    #[test]
    fn test_provider_support_bad_pkg_file() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    provider: file\n"));
        assert!(cmd_validate_check_resource_provider_support(f.path(), false).is_ok());
    }

    #[test]
    fn test_provider_support_bad_svc_file() {
        let f = write_cfg(&format!("{BASE}resources:\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n    provider: file\n"));
        assert!(cmd_validate_check_resource_provider_support(f.path(), false).is_ok());
    }

    #[test]
    fn test_provider_support_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    provider: file\n"));
        assert!(cmd_validate_check_resource_provider_support(f.path(), true).is_ok());
    }

    // FJ-837: secret refs
    #[test]
    fn test_secret_refs_none() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: normal content\n"));
        assert!(cmd_validate_check_resource_secret_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_secret_refs_found() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: \"key={{{{secret.db_pass}}}}\"\n"));
        assert!(cmd_validate_check_resource_secret_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_secret_refs_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: \"db=${{{{secret.pass}}}}\"\n"));
        assert!(cmd_validate_check_resource_secret_refs(f.path(), true).is_ok());
    }

    // FJ-841: idempotency hints
    #[test]
    fn test_idempotency_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: hi\n    state: present\n"));
        assert!(cmd_validate_check_resource_idempotency_hints(f.path(), false).is_ok());
    }

    #[test]
    fn test_idempotency_missing_state() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: hi\n"));
        assert!(cmd_validate_check_resource_idempotency_hints(f.path(), false).is_ok());
    }

    #[test]
    fn test_idempotency_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: hi\n"));
        assert!(cmd_validate_check_resource_idempotency_hints(f.path(), true).is_ok());
    }

    // FJ-849: machine affinity
    #[test]
    fn test_affinity_valid() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_machine_affinity(f.path(), false).is_ok());
    }

    #[test]
    fn test_affinity_invalid_machine() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: ghost\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_machine_affinity(f.path(), false).is_ok());
    }

    #[test]
    fn test_affinity_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: ghost\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_machine_affinity(f.path(), true).is_ok());
    }

    // FJ-853: drift risk
    #[test]
    fn test_drift_risk_file() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: hi\n"));
        assert!(cmd_validate_check_resource_drift_risk(f.path(), false).is_ok());
    }

    #[test]
    fn test_drift_risk_service() {
        let f = write_cfg(&format!("{BASE}resources:\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n    state: running\n"));
        assert!(cmd_validate_check_resource_drift_risk(f.path(), false).is_ok());
    }

    #[test]
    fn test_drift_risk_with_deps() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: hi\n    depends_on:\n      - pkg\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_drift_risk(f.path(), false).is_ok());
    }

    #[test]
    fn test_drift_risk_empty() {
        let f = write_cfg(&format!("{BASE}resources: {{}}\n"));
        assert!(cmd_validate_check_resource_drift_risk(f.path(), false).is_ok());
    }

    #[test]
    fn test_drift_risk_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"));
        assert!(cmd_validate_check_resource_drift_risk(f.path(), true).is_ok());
    }

    // FJ-857: tag coverage
    #[test]
    fn test_tag_coverage_all_tagged() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - web\n"));
        assert!(cmd_validate_check_resource_tag_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_tag_coverage_missing() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_tag_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_tag_coverage_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_tag_coverage(f.path(), true).is_ok());
    }

    // FJ-861: lifecycle hooks
    #[test]
    fn test_lifecycle_hooks_valid() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - b\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"));
        assert!(cmd_validate_check_resource_lifecycle_hooks(f.path(), false).is_ok());
    }

    #[test]
    fn test_lifecycle_hooks_missing_dep() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - ghost\n"));
        assert!(cmd_validate_check_resource_lifecycle_hooks(f.path(), false).is_ok());
    }

    #[test]
    fn test_lifecycle_hooks_missing_trigger() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    triggers:\n      - ghost\n"));
        assert!(cmd_validate_check_resource_lifecycle_hooks(f.path(), false).is_ok());
    }

    #[test]
    fn test_lifecycle_hooks_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - ghost\n"));
        assert!(cmd_validate_check_resource_lifecycle_hooks(f.path(), true).is_ok());
    }

    // FJ-865: provider version
    #[test]
    fn test_provider_version_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    provider: apt@1.0\n"));
        assert!(cmd_validate_check_resource_provider_version(f.path(), false).is_ok());
    }

    #[test]
    fn test_provider_version_empty() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    provider: \"apt@\"\n"));
        assert!(cmd_validate_check_resource_provider_version(f.path(), false).is_ok());
    }

    #[test]
    fn test_provider_version_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    provider: \"apt@\"\n"));
        assert!(cmd_validate_check_resource_provider_version(f.path(), true).is_ok());
    }
}
