//! Coverage tests for validate_ordering_ext.rs — naming, idempotency, size, fan, GPU, when.

use super::validate_ordering_ext::*;
use std::io::Write;

const BASE: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n";

fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(yaml.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// ── cmd_validate_check_resource_naming_convention_strict ──────────────

#[test]
fn naming_strict_valid() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  my-app:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_naming_convention_strict(f.path(), false).is_ok());
}

#[test]
fn naming_strict_violation() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  MyApp:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_naming_convention_strict(f.path(), false).is_ok());
}

#[test]
fn naming_strict_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  MyApp:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_naming_convention_strict(f.path(), true).is_ok());
}

#[test]
fn naming_strict_all_valid_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  my_app:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_naming_convention_strict(f.path(), true).is_ok());
}

// ── cmd_validate_check_resource_idempotency_annotations ──────────────

#[test]
fn idempotency_text() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: curl\n"
    ));
    assert!(cmd_validate_check_resource_idempotency_annotations(f.path(), false).is_ok());
}

#[test]
fn idempotency_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: curl\n"
    ));
    assert!(cmd_validate_check_resource_idempotency_annotations(f.path(), true).is_ok());
}

#[test]
fn idempotency_empty_resources() {
    let f = write_cfg(&format!("{BASE}resources: {{}}\n"));
    assert!(cmd_validate_check_resource_idempotency_annotations(f.path(), false).is_ok());
}

// ── cmd_validate_check_resource_content_size_limit ────────────────────

#[test]
fn content_size_within_limit() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: small\n"
    ));
    assert!(cmd_validate_check_resource_content_size_limit(f.path(), false).is_ok());
}

#[test]
fn content_size_over_limit() {
    let big = "x".repeat(20000);
    let f = write_cfg(&format!(
        "{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"{big}\"\n"
    ));
    assert!(cmd_validate_check_resource_content_size_limit(f.path(), false).is_ok());
}

#[test]
fn content_size_json() {
    let big = "x".repeat(20000);
    let f = write_cfg(&format!(
        "{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"{big}\"\n"
    ));
    assert!(cmd_validate_check_resource_content_size_limit(f.path(), true).is_ok());
}

#[test]
fn content_size_no_content() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n"
    ));
    assert!(cmd_validate_check_resource_content_size_limit(f.path(), false).is_ok());
}

// ── cmd_validate_check_resource_dependency_fan_limit ──────────────────

#[test]
fn fan_limit_within() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"
    ));
    assert!(cmd_validate_check_resource_dependency_fan_limit(f.path(), false).is_ok());
}

#[test]
fn fan_limit_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_dependency_fan_limit(f.path(), true).is_ok());
}

#[test]
fn fan_limit_no_deps() {
    let f = write_cfg(&format!("{BASE}resources: {{}}\n"));
    assert!(cmd_validate_check_resource_dependency_fan_limit(f.path(), false).is_ok());
}

// ── cmd_validate_check_resource_gpu_backend_consistency ───────────────

#[test]
fn gpu_no_resources() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: curl\n"
    ));
    assert!(cmd_validate_check_resource_gpu_backend_consistency(f.path(), false).is_ok());
}

#[test]
fn gpu_consistent() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: package\n    name: cuda-toolkit\n    gpu_backend: nvidia\n  b:\n    machine: m1\n    type: package\n    name: cuDNN\n    gpu_backend: nvidia\n"
    ));
    assert!(cmd_validate_check_resource_gpu_backend_consistency(f.path(), false).is_ok());
}

#[test]
fn gpu_inconsistent() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: package\n    name: cuda\n    gpu_backend: nvidia\n  b:\n    machine: m1\n    type: package\n    name: rocm\n    gpu_backend: rocm\n"
    ));
    assert!(cmd_validate_check_resource_gpu_backend_consistency(f.path(), false).is_ok());
}

#[test]
fn gpu_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: package\n    name: cuda\n    gpu_backend: nvidia\n"
    ));
    assert!(cmd_validate_check_resource_gpu_backend_consistency(f.path(), true).is_ok());
}

// ── cmd_validate_check_resource_when_condition_syntax ─────────────────

#[test]
fn when_no_conditions() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: curl\n"
    ));
    assert!(cmd_validate_check_resource_when_condition_syntax(f.path(), false).is_ok());
}

#[test]
fn when_valid_condition() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: curl\n    when: \"{{{{arch}}}} == aarch64\"\n"
    ));
    assert!(cmd_validate_check_resource_when_condition_syntax(f.path(), false).is_ok());
}

#[test]
fn when_empty_condition() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: curl\n    when: \"\"\n"
    ));
    assert!(cmd_validate_check_resource_when_condition_syntax(f.path(), false).is_ok());
}

#[test]
fn when_unclosed_template() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: curl\n    when: \"{{{{arch\"\n"
    ));
    assert!(cmd_validate_check_resource_when_condition_syntax(f.path(), false).is_ok());
}

#[test]
fn when_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  pkg:\n    machine: m1\n    type: package\n    name: curl\n    when: \"\"\n"
    ));
    assert!(cmd_validate_check_resource_when_condition_syntax(f.path(), true).is_ok());
}
