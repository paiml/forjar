//! Coverage tests: validate_audit, validate_topology, validate_security_ext,
//! validate_governance_ext (FJ-1372).

#![allow(unused_imports)]
use super::validate_audit::*;
use super::validate_governance_ext::*;
use super::validate_security_ext::*;
use super::validate_topology::*;
use std::path::{Path, PathBuf};

fn write_cfg(dir: &Path, yaml: &str) -> PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn base_cfg() -> &'static str {
    r#"version: "1.0"
name: validate-ext3
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m1
    path: /tmp/test.txt
    content: "hello"
    tags: [app]
"#
}

fn deps_cfg() -> &'static str {
    r#"version: "1.0"
name: validate-deps
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  base:
    type: file
    machine: m1
    path: /tmp/base.txt
    content: "base"
    tags: [infra]
  app:
    type: file
    machine: m1
    path: /tmp/app.txt
    content: "app"
    depends_on: [base]
    tags: [app]
"#
}

// ── validate_audit ──────────────────────────────────────────

#[test]
fn dependency_completeness_audit_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_completeness_audit(&f, false).is_ok());
}
#[test]
fn dependency_completeness_audit_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_completeness_audit(&f, true).is_ok());
}
#[test]
fn machine_coverage_gap_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_machine_coverage_gap(&f, false).is_ok());
}
#[test]
fn machine_coverage_gap_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_machine_coverage_gap(&f, true).is_ok());
}
#[test]
fn path_depth_limit_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_path_depth_limit(&f, false).is_ok());
}
#[test]
fn path_depth_limit_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_path_depth_limit(&f, true).is_ok());
}

// ── validate_topology ───────────────────────────────────────

#[test]
fn circular_dependency_depth_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_circular_dependency_depth(&f, false).is_ok());
}
#[test]
fn circular_dependency_depth_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_circular_dependency_depth(&f, true).is_ok());
}
#[test]
fn orphan_detection_deep_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_orphan_detection_deep(&f, false).is_ok());
}
#[test]
fn orphan_detection_deep_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_orphan_detection_deep(&f, true).is_ok());
}
#[test]
fn provider_diversity_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_provider_diversity(&f, false).is_ok());
}
#[test]
fn provider_diversity_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_provider_diversity(&f, true).is_ok());
}

// ── validate_security_ext ───────────────────────────────────

#[test]
fn dependency_symmetry_deep_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_symmetry_deep(&f, false).is_ok());
}
#[test]
fn dependency_symmetry_deep_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_symmetry_deep(&f, true).is_ok());
}
#[test]
fn tag_namespace_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_namespace(&f, false).is_ok());
}
#[test]
fn tag_namespace_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_namespace(&f, true).is_ok());
}
#[test]
fn machine_capacity_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_machine_capacity(&f, false).is_ok());
}
#[test]
fn machine_capacity_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_machine_capacity(&f, true).is_ok());
}

// ── validate_governance_ext ─────────────────────────────────

#[test]
fn dependency_fan_out_limit_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_fan_out_limit(&f, false).is_ok());
}
#[test]
fn dependency_fan_out_limit_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_fan_out_limit(&f, true).is_ok());
}
#[test]
fn tag_required_keys_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_required_keys(&f, false).is_ok());
}
#[test]
fn tag_required_keys_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_required_keys(&f, true).is_ok());
}
#[test]
fn content_drift_risk_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_content_drift_risk(&f, false).is_ok());
}
#[test]
fn content_drift_risk_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_content_drift_risk(&f, true).is_ok());
}
