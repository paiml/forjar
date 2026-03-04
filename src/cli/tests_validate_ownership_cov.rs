//! Coverage tests for validate_ownership.rs — FJ-869→FJ-893.

#![allow(unused_imports)]
use super::validate_ownership::*;
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

    // FJ-869: naming convention
    #[test]
    fn test_naming_convention_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  web-nginx:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_naming_convention(f.path(), false).is_ok());
    }

    #[test]
    fn test_naming_convention_uppercase() {
        let f = write_cfg(&format!("{BASE}resources:\n  WebNginx:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_naming_convention(f.path(), false).is_ok());
    }

    #[test]
    fn test_naming_convention_spaces() {
        let f = write_cfg(&format!("{BASE}resources:\n  \"web nginx\":\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_naming_convention(f.path(), false).is_ok());
    }

    #[test]
    fn test_naming_convention_leading_hyphen() {
        let f = write_cfg(&format!("{BASE}resources:\n  -nginx:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_naming_convention(f.path(), false).is_ok());
    }

    #[test]
    fn test_naming_convention_double_underscore() {
        let f = write_cfg(&format!("{BASE}resources:\n  web__nginx:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_naming_convention(f.path(), false).is_ok());
    }

    #[test]
    fn test_naming_convention_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  WebNginx:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_naming_convention(f.path(), true).is_ok());
    }

    // FJ-873: idempotency
    #[test]
    fn test_idempotency_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: static\n"));
        assert!(cmd_validate_check_resource_idempotency(f.path(), false).is_ok());
    }

    #[test]
    fn test_idempotency_dynamic_content() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"ts=$(date)\"\n"));
        assert!(cmd_validate_check_resource_idempotency(f.path(), false).is_ok());
    }

    #[test]
    fn test_idempotency_absent_with_triggers() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: hi\n    state: absent\n    triggers:\n      - svc\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_idempotency(f.path(), false).is_ok());
    }

    #[test]
    fn test_idempotency_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"ts=$(date)\"\n"));
        assert!(cmd_validate_check_resource_idempotency(f.path(), true).is_ok());
    }

    // FJ-877: documentation
    #[test]
    fn test_documentation_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: hi\n    tags:\n      - web\n"));
        assert!(cmd_validate_check_resource_documentation(f.path(), false).is_ok());
    }

    #[test]
    fn test_documentation_missing() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_documentation(f.path(), false).is_ok());
    }

    #[test]
    fn test_documentation_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_documentation(f.path(), true).is_ok());
    }

    // FJ-881: ownership
    #[test]
    fn test_ownership_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - team-web\n"));
        assert!(cmd_validate_check_resource_ownership(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_missing() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_ownership(f.path(), false).is_ok());
    }

    #[test]
    fn test_ownership_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_ownership(f.path(), true).is_ok());
    }

    // FJ-885: secret exposure
    #[test]
    fn test_secret_exposure_none() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: normal text\n"));
        assert!(cmd_validate_check_resource_secret_exposure(f.path(), false).is_ok());
    }

    #[test]
    fn test_secret_exposure_password() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"db_password=hunter2\"\n"));
        assert!(cmd_validate_check_resource_secret_exposure(f.path(), false).is_ok());
    }

    #[test]
    fn test_secret_exposure_api_key() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"api_key=abc123\"\n"));
        assert!(cmd_validate_check_resource_secret_exposure(f.path(), false).is_ok());
    }

    #[test]
    fn test_secret_exposure_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"password=x\"\n"));
        assert!(cmd_validate_check_resource_secret_exposure(f.path(), true).is_ok());
    }

    // FJ-889: tag standards
    #[test]
    fn test_tag_standards_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - web\n      - production\n"));
        assert!(cmd_validate_check_resource_tag_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_tag_standards_uppercase() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - Web\n"));
        assert!(cmd_validate_check_resource_tag_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_tag_standards_spaces() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - \"web server\"\n"));
        assert!(cmd_validate_check_resource_tag_standards(f.path(), false).is_ok());
    }

    #[test]
    fn test_tag_standards_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - Web\n"));
        assert!(cmd_validate_check_resource_tag_standards(f.path(), true).is_ok());
    }

    // FJ-893: privilege escalation
    #[test]
    fn test_priv_esc_none() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/app.conf\n    content: normal config\n"));
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), false).is_ok());
    }

    #[test]
    fn test_priv_esc_chmod_setuid() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /usr/bin/app\n    content: \"chmod +s /usr/bin/app\"\n"));
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), false).is_ok());
    }

    #[test]
    fn test_priv_esc_sudoers_path() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/sudoers.d/app\n    content: \"app ALL=(ALL) ALL\"\n"));
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), false).is_ok());
    }

    #[test]
    fn test_priv_esc_nopasswd() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/sudoers.d/app\n    content: \"app ALL=(ALL) NOPASSWD: ALL\"\n"));
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), false).is_ok());
    }

    #[test]
    fn test_priv_esc_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/sudoers.d/app\n    content: \"NOPASSWD\"\n"));
        assert!(cmd_validate_check_resource_privilege_escalation(f.path(), true).is_ok());
    }
}
