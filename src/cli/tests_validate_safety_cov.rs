//! Coverage tests for validate_safety.rs — FJ-757→FJ-793.

#![allow(unused_imports)]
use super::validate_safety::*;
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

    // FJ-757: circular deps
    #[test]
    fn test_circular_deps_none() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_circular_deps(f.path(), false).is_ok());
    }

    #[test]
    fn test_circular_deps_found() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - b\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n    depends_on:\n      - a\n"));
        assert!(cmd_validate_check_circular_deps(f.path(), false).is_err());
    }

    #[test]
    fn test_circular_deps_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_circular_deps(f.path(), true).is_ok());
    }

    // FJ-761: machine refs
    #[test]
    fn test_machine_refs_valid() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_machine_refs(f.path(), false).is_ok());
    }

    #[test]
    fn test_machine_refs_bad() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: bogus\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_machine_refs(f.path(), false).is_err());
    }

    #[test]
    fn test_machine_refs_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: bogus\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_machine_refs(f.path(), true).is_ok());
    }

    // FJ-765: provider consistency
    #[test]
    fn test_provider_consistency_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: package\n    name: nginx\n    provider: apt\n  b:\n    machine: m1\n    type: package\n    name: curl\n    provider: apt\n"));
        assert!(cmd_validate_check_provider_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_provider_consistency_mixed() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: package\n    name: nginx\n    provider: apt\n  b:\n    machine: m1\n    type: package\n    name: curl\n    provider: yum\n"));
        assert!(cmd_validate_check_provider_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_provider_consistency_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: package\n    name: nginx\n    provider: apt\n  b:\n    machine: m1\n    type: package\n    name: curl\n    provider: yum\n"));
        assert!(cmd_validate_check_provider_consistency(f.path(), true).is_ok());
    }

    // FJ-769: state values
    #[test]
    fn test_state_values_valid() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    state: file\n"));
        assert!(cmd_validate_check_state_values(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_values_invalid() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    state: bogus\n"));
        assert!(cmd_validate_check_state_values(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_values_service() {
        let f = write_cfg(&format!("{BASE}resources:\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n    state: running\n"));
        assert!(cmd_validate_check_state_values(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_values_service_bad() {
        let f = write_cfg(&format!("{BASE}resources:\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n    state: bogus\n"));
        assert!(cmd_validate_check_state_values(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_values_mount() {
        let f = write_cfg(&format!("{BASE}resources:\n  mnt:\n    machine: m1\n    type: mount\n    path: /mnt/data\n    content: /dev/sda1\n    state: mounted\n"));
        assert!(cmd_validate_check_state_values(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_values_docker() {
        let f = write_cfg(&format!("{BASE}resources:\n  dock:\n    machine: m1\n    type: docker\n    name: myapp\n    state: running\n"));
        assert!(cmd_validate_check_state_values(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_values_package() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: present\n"));
        assert!(cmd_validate_check_state_values(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_values_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    state: bogus\n"));
        assert!(cmd_validate_check_state_values(f.path(), true).is_ok());
    }

    // FJ-773: unused machines
    #[test]
    fn test_unused_machines_none() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_unused_machines(f.path(), false).is_ok());
    }

    #[test]
    fn test_unused_machines_found() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n  m2:\n    hostname: m2\n    addr: 5.6.7.8\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_unused_machines(f.path(), false).is_ok());
    }

    #[test]
    fn test_unused_machines_json() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n  m2:\n    hostname: m2\n    addr: 5.6.7.8\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_unused_machines(f.path(), true).is_ok());
    }

    // FJ-777: tag consistency
    #[test]
    fn test_tag_consistency_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - web-server\n"));
        assert!(cmd_validate_check_tag_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_tag_consistency_bad() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - WebServer\n"));
        assert!(cmd_validate_check_tag_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_tag_consistency_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - WebServer\n"));
        assert!(cmd_validate_check_tag_consistency(f.path(), true).is_ok());
    }

    // FJ-781: dependency exists
    #[test]
    fn test_dep_exists_valid() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - b\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"));
        assert!(cmd_validate_check_dependency_exists(f.path(), false).is_ok());
    }

    #[test]
    fn test_dep_exists_missing() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - ghost\n"));
        assert!(cmd_validate_check_dependency_exists(f.path(), false).is_err());
    }

    #[test]
    fn test_dep_exists_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - ghost\n"));
        assert!(cmd_validate_check_dependency_exists(f.path(), true).is_ok());
    }

    // FJ-785: path conflicts
    #[test]
    fn test_path_conflicts_none() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"));
        assert!(cmd_validate_check_path_conflicts_strict(f.path(), false).is_ok());
    }

    #[test]
    fn test_path_conflicts_found() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /etc/conf\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /etc/conf\n    content: b\n"));
        assert!(cmd_validate_check_path_conflicts_strict(f.path(), false).is_err());
    }

    #[test]
    fn test_path_conflicts_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /etc/conf\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /etc/conf\n    content: b\n"));
        assert!(cmd_validate_check_path_conflicts_strict(f.path(), true).is_ok());
    }

    // FJ-789: duplicate names
    #[test]
    fn test_duplicate_names_none() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_duplicate_names(f.path(), false).is_ok());
    }

    #[test]
    fn test_duplicate_names_found() {
        let f = write_cfg(&format!("{BASE}resources:\n  web/nginx:\n    machine: m1\n    type: package\n    name: nginx\n  db/nginx:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_duplicate_names(f.path(), false).is_ok());
    }

    #[test]
    fn test_duplicate_names_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  web/nginx:\n    machine: m1\n    type: package\n    name: nginx\n  db/nginx:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_duplicate_names(f.path(), true).is_ok());
    }

    // FJ-793: resource groups
    #[test]
    fn test_resource_groups_none() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_groups(f.path(), false).is_ok());
    }

    #[test]
    fn test_resource_groups_present() {
        let f = write_cfg(&format!("{BASE}resources:\n  web/nginx:\n    machine: m1\n    type: package\n    name: nginx\n  web/curl:\n    machine: m1\n    type: package\n    name: curl\n"));
        assert!(cmd_validate_check_resource_groups(f.path(), false).is_ok());
    }

    #[test]
    fn test_resource_groups_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  web/nginx:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_groups(f.path(), true).is_ok());
    }
}
