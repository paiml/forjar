//! Coverage tests: validate_resilience, validate_maturity, validate_hygiene,
//! validate_config_quality, validate_compliance_ext, validate_scoring, validate_security (FJ-1372).

#![allow(unused_imports)]
use super::validate_compliance_ext::*;
use super::validate_config_quality::*;
use super::validate_hygiene::*;
use super::validate_maturity::*;
use super::validate_resilience::*;
use super::validate_scoring::*;
use super::validate_security::*;
use std::path::{Path, PathBuf};

fn write_cfg(dir: &Path, yaml: &str) -> PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn base_cfg() -> &'static str {
    r#"version: "1.0"
name: validate-ext2
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

// ── validate_resilience ─────────────────────────────────────

#[test]
fn lifecycle_hook_coverage_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_lifecycle_hook_coverage(&f, false).is_ok());
}
#[test]
fn lifecycle_hook_coverage_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_lifecycle_hook_coverage(&f, true).is_ok());
}
#[test]
fn secret_rotation_age_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_secret_rotation_age(&f, false).is_ok());
}
#[test]
fn secret_rotation_age_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_secret_rotation_age(&f, true).is_ok());
}
#[test]
fn dependency_chain_depth_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_chain_depth(&f, false).is_ok());
}
#[test]
fn dependency_chain_depth_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_chain_depth(&f, true).is_ok());
}

// ── validate_maturity ───────────────────────────────────────

#[test]
fn dependency_version_drift_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_version_drift(&f, false).is_ok());
}
#[test]
fn dependency_version_drift_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_version_drift(&f, true).is_ok());
}
#[test]
fn naming_length_limit_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_naming_length_limit(&f, false).is_ok());
}
#[test]
fn naming_length_limit_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_naming_length_limit(&f, true).is_ok());
}
#[test]
fn type_coverage_per_machine_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_type_coverage_per_machine(&f, false).is_ok());
}
#[test]
fn type_coverage_per_machine_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_type_coverage_per_machine(&f, true).is_ok());
}

// ── validate_hygiene ────────────────────────────────────────

#[test]
fn dependency_depth_variance_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_depth_variance(&f, false).is_ok());
}
#[test]
fn dependency_depth_variance_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_depth_variance(&f, true).is_ok());
}
#[test]
fn tag_key_naming_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_key_naming(&f, false).is_ok());
}
#[test]
fn tag_key_naming_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_key_naming(&f, true).is_ok());
}
#[test]
fn content_length_limit_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_content_length_limit(&f, false).is_ok());
}
#[test]
fn content_length_limit_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_content_length_limit(&f, true).is_ok());
}

// ── validate_config_quality ─────────────────────────────────

#[test]
fn dependency_isolation_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_isolation(&f, false).is_ok());
}
#[test]
fn dependency_isolation_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_isolation(&f, true).is_ok());
}
#[test]
fn tag_value_consistency_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_value_consistency(&f, false).is_ok());
}
#[test]
fn tag_value_consistency_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_value_consistency(&f, true).is_ok());
}
#[test]
fn machine_distribution_balance_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_machine_distribution_balance(&f, false).is_ok());
}
#[test]
fn machine_distribution_balance_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_machine_distribution_balance(&f, true).is_ok());
}

// ── validate_compliance_ext ─────────────────────────────────

#[test]
fn compliance_tags_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_compliance_tags(&f, false).is_ok());
}
#[test]
fn compliance_tags_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_compliance_tags(&f, true).is_ok());
}
#[test]
fn rollback_coverage_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_rollback_coverage(&f, false).is_ok());
}
#[test]
fn rollback_coverage_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_rollback_coverage(&f, true).is_ok());
}
#[test]
fn dependency_balance_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_balance(&f, false).is_ok());
}
#[test]
fn dependency_balance_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_balance(&f, true).is_ok());
}

// ── validate_scoring ────────────────────────────────────────

#[test]
fn dependency_ordering_consistency_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_ordering_consistency(&f, false).is_ok());
}
#[test]
fn dependency_ordering_consistency_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_ordering_consistency(&f, true).is_ok());
}
#[test]
fn tag_value_format_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_value_format(&f, false).is_ok());
}
#[test]
fn tag_value_format_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_value_format(&f, true).is_ok());
}
#[test]
fn provider_version_pinning_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_provider_version_pinning(&f, false).is_ok());
}
#[test]
fn provider_version_pinning_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_provider_version_pinning(&f, true).is_ok());
}

// ── validate_security ───────────────────────────────────────

#[test]
fn secret_scope_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_secret_scope(&f, false).is_ok());
}
#[test]
fn secret_scope_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_secret_scope(&f, true).is_ok());
}
#[test]
fn deprecation_usage_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_deprecation_usage(&f, false).is_ok());
}
#[test]
fn deprecation_usage_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_deprecation_usage(&f, true).is_ok());
}
#[test]
fn when_condition_coverage_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_when_condition_coverage(&f, false).is_ok());
}
#[test]
fn when_condition_coverage_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_when_condition_coverage(&f, true).is_ok());
}

fn pkg_cfg() -> &'static str {
    r#"version: "1.0"
name: validate-pkg
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  web:
    type: package
    machine: m1
    packages: [nginx]
    tags: [web]
  db:
    type: package
    machine: m1
    packages: [postgresql]
    version: "15"
    tags: [db]
  cfg:
    type: file
    machine: m1
    path: /tmp/app.conf
    content: "key=value"
    tags: [app]
"#
}

// ── deeper coverage: package resources & edge cases ─────────

#[test]
fn version_drift_with_packages_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_dependency_version_drift(&f, false).is_ok());
}
#[test]
fn version_drift_with_packages_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_dependency_version_drift(&f, true).is_ok());
}
#[test]
fn naming_length_with_packages_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_naming_length_limit(&f, false).is_ok());
}
#[test]
fn type_coverage_with_packages_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_type_coverage_per_machine(&f, false).is_ok());
}
#[test]
fn type_coverage_with_packages_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_type_coverage_per_machine(&f, true).is_ok());
}
#[test]
fn content_length_with_packages_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_content_length_limit(&f, false).is_ok());
}
#[test]
fn dependency_depth_variance_single_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_dependency_depth_variance(&f, true).is_ok());
}
#[test]
fn scoring_version_pinning_with_pkg_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_provider_version_pinning(&f, false).is_ok());
}
#[test]
fn scoring_version_pinning_with_pkg_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), pkg_cfg());
    assert!(cmd_validate_check_resource_provider_version_pinning(&f, true).is_ok());
}
#[test]
fn scoring_dep_ordering_with_deps_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_ordering_consistency(&f, true).is_ok());
}
#[test]
fn security_when_condition_with_deps() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_when_condition_coverage(&f, false).is_ok());
}
#[test]
fn compliance_tags_with_deps() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_compliance_tags(&f, false).is_ok());
}
#[test]
fn rollback_coverage_with_deps() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_rollback_coverage(&f, false).is_ok());
}

// ── error paths: file not found ─────────────────────────────

#[test]
fn resilience_hook_coverage_missing() {
    let p = std::path::PathBuf::from("/tmp/nonexistent_ext2.yaml");
    assert!(cmd_validate_check_resource_lifecycle_hook_coverage(&p, false).is_err());
}
#[test]
fn resilience_secret_rotation_missing() {
    let p = std::path::PathBuf::from("/tmp/nonexistent_ext2.yaml");
    assert!(cmd_validate_check_resource_secret_rotation_age(&p, false).is_err());
}
#[test]
fn maturity_version_drift_missing() {
    let p = std::path::PathBuf::from("/tmp/nonexistent_ext2.yaml");
    assert!(cmd_validate_check_resource_dependency_version_drift(&p, false).is_err());
}
#[test]
fn maturity_naming_length_missing() {
    let p = std::path::PathBuf::from("/tmp/nonexistent_ext2.yaml");
    assert!(cmd_validate_check_resource_naming_length_limit(&p, false).is_err());
}
#[test]
fn hygiene_depth_variance_missing() {
    let p = std::path::PathBuf::from("/tmp/nonexistent_ext2.yaml");
    assert!(cmd_validate_check_resource_dependency_depth_variance(&p, false).is_err());
}
#[test]
fn hygiene_tag_key_naming_missing() {
    let p = std::path::PathBuf::from("/tmp/nonexistent_ext2.yaml");
    assert!(cmd_validate_check_resource_tag_key_naming(&p, false).is_err());
}
#[test]
fn config_quality_dep_isolation_missing() {
    let p = std::path::PathBuf::from("/tmp/nonexistent_ext2.yaml");
    assert!(cmd_validate_check_resource_dependency_isolation(&p, false).is_err());
}
#[test]
fn scoring_dep_ordering_missing() {
    let p = std::path::PathBuf::from("/tmp/nonexistent_ext2.yaml");
    assert!(cmd_validate_check_resource_dependency_ordering_consistency(&p, false).is_err());
}
#[test]
fn scoring_tag_format_missing() {
    let p = std::path::PathBuf::from("/tmp/nonexistent_ext2.yaml");
    assert!(cmd_validate_check_resource_tag_value_format(&p, false).is_err());
}
// Remaining error-path tests moved to tests_cov_validate_ext3.rs
