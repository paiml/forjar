//! Coverage tests for validate_ownership_b.rs — FJ-897→FJ-921.

use super::validate_ownership_b::*;
use std::io::Write;

const BASE: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n";

fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(yaml.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// ── FJ-897: update safety ───────────────────────────────────────────

#[test]
fn update_safety_no_issues() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_update_safety(f.path(), false).is_ok());
}

#[test]
fn update_safety_service_with_triggers() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n    triggers: [cfg]\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: x\n"
    ));
    assert!(cmd_validate_check_resource_update_safety(f.path(), false).is_ok());
}

#[test]
fn update_safety_mount() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  mnt:\n    machine: m1\n    type: mount\n    device: /dev/sda1\n    mount_point: /data\n"
    ));
    assert!(cmd_validate_check_resource_update_safety(f.path(), false).is_ok());
}

#[test]
fn update_safety_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  mnt:\n    machine: m1\n    type: mount\n    device: /dev/sda1\n    mount_point: /data\n"
    ));
    assert!(cmd_validate_check_resource_update_safety(f.path(), true).is_ok());
}

// ── FJ-901: cross-machine consistency ───────────────────────────────

#[test]
fn cross_machine_no_issues() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_cross_machine_consistency(f.path(), false).is_ok());
}

#[test]
fn cross_machine_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_cross_machine_consistency(f.path(), true).is_ok());
}

// ── FJ-905: version pinning ────────────────────────────────────────

#[test]
fn version_pinning_pinned() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    version: '1.24'\n"
    ));
    assert!(cmd_validate_check_resource_version_pinning(f.path(), false).is_ok());
}

#[test]
fn version_pinning_unpinned() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"
    ));
    assert!(cmd_validate_check_resource_version_pinning(f.path(), false).is_ok());
}

#[test]
fn version_pinning_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"
    ));
    assert!(cmd_validate_check_resource_version_pinning(f.path(), true).is_ok());
}

// ── FJ-909: dependency completeness ─────────────────────────────────

#[test]
fn dep_completeness_valid() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"
    ));
    assert!(cmd_validate_check_resource_dependency_completeness(f.path(), false).is_ok());
}

#[test]
fn dep_completeness_missing() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [ghost]\n"
    ));
    assert!(cmd_validate_check_resource_dependency_completeness(f.path(), false).is_ok());
}

#[test]
fn dep_completeness_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [ghost]\n"
    ));
    assert!(cmd_validate_check_resource_dependency_completeness(f.path(), true).is_ok());
}

// ── FJ-913: state coverage ──────────────────────────────────────────

#[test]
fn state_coverage_has_state() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: present\n"
    ));
    assert!(cmd_validate_check_resource_state_coverage(f.path(), false).is_ok());
}

#[test]
fn state_coverage_missing() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"
    ));
    assert!(cmd_validate_check_resource_state_coverage(f.path(), false).is_ok());
}

#[test]
fn state_coverage_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"
    ));
    assert!(cmd_validate_check_resource_state_coverage(f.path(), true).is_ok());
}

// ── FJ-917: rollback safety ─────────────────────────────────────────

#[test]
fn rollback_safe() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_rollback_safety(f.path(), false).is_ok());
}

#[test]
fn rollback_risky() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    triggers: [svc]\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n"
    ));
    assert!(cmd_validate_check_resource_rollback_safety(f.path(), false).is_ok());
}

#[test]
fn rollback_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    triggers: [svc]\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n"
    ));
    assert!(cmd_validate_check_resource_rollback_safety(f.path(), true).is_ok());
}

// ── FJ-921: config maturity ─────────────────────────────────────────

#[test]
fn maturity_empty() {
    let f = write_cfg(&format!("{BASE}resources: {{}}\n"));
    assert!(cmd_validate_check_resource_config_maturity(f.path(), false).is_ok());
}

#[test]
fn maturity_high() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    tags: [web]\n    state: present\n    version: '1.0'\n    resource_group: frontend\n    depends_on: [base]\n  base:\n    machine: m1\n    type: package\n    name: base\n"
    ));
    assert!(cmd_validate_check_resource_config_maturity(f.path(), false).is_ok());
}

#[test]
fn maturity_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"
    ));
    assert!(cmd_validate_check_resource_config_maturity(f.path(), true).is_ok());
}
