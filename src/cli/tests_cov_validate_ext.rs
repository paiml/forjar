//! Coverage tests: validate_ownership, validate_governance, validate_ordering_ext (FJ-1372).
//! Each test exercises a validation function with a minimal config (text + JSON).

#![allow(unused_imports)]
use super::validate_governance::*;
use super::validate_ordering_ext::*;
use super::validate_ownership::*;
use std::path::{Path, PathBuf};

fn write_cfg(dir: &Path, yaml: &str) -> PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn base_cfg() -> &'static str {
    r#"version: "1.0"
name: validate-ext
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

// ── validate_ownership ──────────────────────────────────────

#[test]
fn naming_convention_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_naming_convention(&f, false).is_ok());
}
#[test]
fn naming_convention_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_naming_convention(&f, true).is_ok());
}
#[test]
fn idempotency_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_idempotency(&f, false).is_ok());
}
#[test]
fn idempotency_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_idempotency(&f, true).is_ok());
}
#[test]
fn documentation_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_documentation(&f, false).is_ok());
}
#[test]
fn documentation_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_documentation(&f, true).is_ok());
}
#[test]
fn ownership_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_ownership(&f, false).is_ok());
}
#[test]
fn ownership_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_ownership(&f, true).is_ok());
}
#[test]
fn secret_exposure_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_secret_exposure(&f, false).is_ok());
}
#[test]
fn secret_exposure_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_secret_exposure(&f, true).is_ok());
}
#[test]
fn tag_standards_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_standards(&f, false).is_ok());
}
#[test]
fn tag_standards_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_standards(&f, true).is_ok());
}
#[test]
fn privilege_escalation_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_privilege_escalation(&f, false).is_ok());
}
#[test]
fn privilege_escalation_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_privilege_escalation(&f, true).is_ok());
}
#[test]
fn update_safety_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_update_safety(&f, false).is_ok());
}
#[test]
fn update_safety_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_update_safety(&f, true).is_ok());
}
#[test]
fn cross_machine_consistency_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_cross_machine_consistency(&f, false).is_ok());
}
#[test]
fn cross_machine_consistency_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_cross_machine_consistency(&f, true).is_ok());
}
#[test]
fn version_pinning_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_version_pinning(&f, false).is_ok());
}
#[test]
fn version_pinning_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_version_pinning(&f, true).is_ok());
}
#[test]
fn dependency_completeness_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_completeness(&f, false).is_ok());
}
#[test]
fn dependency_completeness_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_completeness(&f, true).is_ok());
}
#[test]
fn state_coverage_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_state_coverage(&f, false).is_ok());
}
#[test]
fn state_coverage_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_state_coverage(&f, true).is_ok());
}
#[test]
fn rollback_safety_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_rollback_safety(&f, false).is_ok());
}
#[test]
fn rollback_safety_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_rollback_safety(&f, true).is_ok());
}
#[test]
fn config_maturity_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_config_maturity(&f, false).is_ok());
}
#[test]
fn config_maturity_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_config_maturity(&f, true).is_ok());
}

// ── validate_governance ─────────────────────────────────────

#[test]
fn naming_pattern_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_naming_pattern(&f, false, "^[a-z]").is_ok());
}
#[test]
fn naming_pattern_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_naming_pattern(&f, true, "^[a-z]").is_ok());
}
#[test]
fn provider_support_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_provider_support(&f, false).is_ok());
}
#[test]
fn provider_support_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_provider_support(&f, true).is_ok());
}
#[test]
fn secret_refs_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_secret_refs(&f, false).is_ok());
}
#[test]
fn secret_refs_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_secret_refs(&f, true).is_ok());
}
#[test]
fn idempotency_hints_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_idempotency_hints(&f, false).is_ok());
}
#[test]
fn idempotency_hints_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_idempotency_hints(&f, true).is_ok());
}
#[test]
fn gov_dependency_depth_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_depth(&f, false, 10).is_ok());
}
#[test]
fn gov_dependency_depth_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_depth(&f, true, 10).is_ok());
}
#[test]
fn machine_affinity_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_machine_affinity(&f, false).is_ok());
}
#[test]
fn machine_affinity_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_machine_affinity(&f, true).is_ok());
}
#[test]
fn drift_risk_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_drift_risk(&f, false).is_ok());
}
#[test]
fn drift_risk_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_drift_risk(&f, true).is_ok());
}
#[test]
fn tag_coverage_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_coverage(&f, false).is_ok());
}
#[test]
fn tag_coverage_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_coverage(&f, true).is_ok());
}
#[test]
fn lifecycle_hooks_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_lifecycle_hooks(&f, false).is_ok());
}
#[test]
fn lifecycle_hooks_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_lifecycle_hooks(&f, true).is_ok());
}
#[test]
fn provider_version_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_provider_version(&f, false).is_ok());
}
#[test]
fn provider_version_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_provider_version(&f, true).is_ok());
}

// ── validate_ordering_ext ───────────────────────────────────

#[test]
fn naming_convention_strict_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_naming_convention_strict(&f, false).is_ok());
}
#[test]
fn naming_convention_strict_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_naming_convention_strict(&f, true).is_ok());
}
#[test]
fn idempotency_annotations_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_idempotency_annotations(&f, false).is_ok());
}
#[test]
fn idempotency_annotations_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_idempotency_annotations(&f, true).is_ok());
}
#[test]
fn content_size_limit_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_content_size_limit(&f, false).is_ok());
}
#[test]
fn content_size_limit_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_content_size_limit(&f, true).is_ok());
}
#[test]
fn dependency_fan_limit_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_fan_limit(&f, false).is_ok());
}
#[test]
fn dependency_fan_limit_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_fan_limit(&f, true).is_ok());
}
#[test]
fn gpu_backend_consistency_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_gpu_backend_consistency(&f, false).is_ok());
}
#[test]
fn gpu_backend_consistency_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_gpu_backend_consistency(&f, true).is_ok());
}
#[test]
fn when_condition_syntax_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_when_condition_syntax(&f, false).is_ok());
}
#[test]
fn when_condition_syntax_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_when_condition_syntax(&f, true).is_ok());
}
