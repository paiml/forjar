//! Coverage tests for validate_advanced.rs — FJ-797→FJ-825.

#![allow(unused_imports)]
use super::validate_advanced::*;
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

    // FJ-797: orphan resources
    #[test]
    fn test_orphan_resources_all_connected() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - b\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"));
        assert!(cmd_validate_check_orphan_resources(f.path(), false).is_ok());
    }

    #[test]
    fn test_orphan_resources_found() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: package\n    name: nginx\n  b:\n    machine: m1\n    type: package\n    name: curl\n"));
        assert!(cmd_validate_check_orphan_resources(f.path(), false).is_ok());
    }

    #[test]
    fn test_orphan_resources_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_orphan_resources(f.path(), true).is_ok());
    }

    // FJ-801: machine architecture
    #[test]
    fn test_machine_arch_valid() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n    arch: x86_64\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_machine_arch(f.path(), false).is_ok());
    }

    #[test]
    fn test_machine_arch_invalid() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n    arch: mips\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_machine_arch(f.path(), false).is_ok());
    }

    #[test]
    fn test_machine_arch_json() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n    arch: mips\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_machine_arch(f.path(), true).is_ok());
    }

    // FJ-805: health conflicts
    #[test]
    fn test_health_conflicts_none() {
        let f = write_cfg(&format!("{BASE}resources:\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n    state: running\n"));
        assert!(cmd_validate_check_resource_health_conflicts(f.path(), false).is_ok());
    }

    #[test]
    fn test_health_conflicts_type_mismatch() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: running\n"));
        assert!(cmd_validate_check_resource_health_conflicts(f.path(), false).is_ok());
    }

    #[test]
    fn test_health_conflicts_absent_service() {
        let f = write_cfg(&format!("{BASE}resources:\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n    state: absent\n"));
        assert!(cmd_validate_check_resource_health_conflicts(f.path(), false).is_ok());
    }

    #[test]
    fn test_health_conflicts_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: running\n"));
        assert!(cmd_validate_check_resource_health_conflicts(f.path(), true).is_ok());
    }

    // FJ-809: resource overlap
    #[test]
    fn test_resource_overlap_none() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"));
        assert!(cmd_validate_check_resource_overlap(f.path(), false).is_ok());
    }

    #[test]
    fn test_resource_overlap_found() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /etc/conf\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /etc/conf\n    content: b\n"));
        assert!(cmd_validate_check_resource_overlap(f.path(), false).is_ok());
    }

    #[test]
    fn test_resource_overlap_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /etc/conf\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /etc/conf\n    content: b\n"));
        assert!(cmd_validate_check_resource_overlap(f.path(), true).is_ok());
    }

    // FJ-813: resource tags
    #[test]
    fn test_resource_tags_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - web\n"));
        assert!(cmd_validate_check_resource_tags(f.path(), false).is_ok());
    }

    #[test]
    fn test_resource_tags_missing() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_resource_tags(f.path(), false).is_ok());
    }

    #[test]
    fn test_resource_tags_bad_case() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - Web Server\n"));
        assert!(cmd_validate_check_resource_tags(f.path(), false).is_ok());
    }

    #[test]
    fn test_resource_tags_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - Web\n"));
        assert!(cmd_validate_check_resource_tags(f.path(), true).is_ok());
    }

    // FJ-817: state consistency
    #[test]
    fn test_state_consistency_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: present\n"));
        assert!(cmd_validate_check_resource_state_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_consistency_bad() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: running\n"));
        assert!(cmd_validate_check_resource_state_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_consistency_service() {
        let f = write_cfg(&format!("{BASE}resources:\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n    state: running\n"));
        assert!(cmd_validate_check_resource_state_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_consistency_file() {
        let f = write_cfg(&format!("{BASE}resources:\n  f:\n    machine: m1\n    type: file\n    path: /a\n    content: hi\n    state: present\n"));
        assert!(cmd_validate_check_resource_state_consistency(f.path(), false).is_ok());
    }

    #[test]
    fn test_state_consistency_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: running\n"));
        assert!(cmd_validate_check_resource_state_consistency(f.path(), true).is_ok());
    }

    // FJ-821: dependencies complete
    #[test]
    fn test_deps_complete_valid() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - b\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"));
        assert!(cmd_validate_check_resource_dependencies_complete(f.path(), false).is_ok());
    }

    #[test]
    fn test_deps_complete_missing() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - ghost\n"));
        assert!(cmd_validate_check_resource_dependencies_complete(f.path(), false).is_ok());
    }

    #[test]
    fn test_deps_complete_json() {
        let f = write_cfg(&format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on:\n      - ghost\n"));
        assert!(cmd_validate_check_resource_dependencies_complete(f.path(), true).is_ok());
    }

    // FJ-825: machine connectivity
    #[test]
    fn test_connectivity_ok() {
        let f = write_cfg(&format!("{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"));
        assert!(cmd_validate_check_machine_connectivity(f.path(), false).is_ok());
    }

    #[test]
    fn test_connectivity_bad_addr() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: badaddr\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_machine_connectivity(f.path(), false).is_ok());
    }

    #[test]
    fn test_connectivity_localhost() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: localhost\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_machine_connectivity(f.path(), false).is_ok());
    }

    #[test]
    fn test_connectivity_json() {
        let yaml = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: badaddr\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n";
        let f = write_cfg(yaml);
        assert!(cmd_validate_check_machine_connectivity(f.path(), true).is_ok());
    }
}
