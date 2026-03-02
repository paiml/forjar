//! Coverage tests: validate_safety, validate_advanced, validate_ordering (FJ-1372).
//! Each test exercises a validation function with a minimal config.

#![allow(unused_imports)]
use super::validate_advanced::*;
use super::validate_ordering::*;
use super::validate_safety::*;
use std::path::{Path, PathBuf};

fn write_cfg(dir: &Path, yaml: &str) -> PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn base_cfg() -> &'static str {
    r#"version: "1.0"
name: validate-test
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

// ── validate_safety: circular deps ──────────────────────────
#[test]
fn circular_deps_none_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_circular_deps(&f, false).is_ok());
}
#[test]
fn circular_deps_none_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_circular_deps(&f, true).is_ok());
}

// ── validate_safety: machine refs ───────────────────────────
#[test]
fn machine_refs_valid_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_machine_refs(&f, false).is_ok());
}
#[test]
fn machine_refs_valid_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_machine_refs(&f, true).is_ok());
}

// ── validate_safety: provider consistency ───────────────────
#[test]
fn provider_consistency_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_provider_consistency(&f, false).is_ok());
}
#[test]
fn provider_consistency_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_provider_consistency(&f, true).is_ok());
}

// ── validate_safety: state values ───────────────────────────
#[test]
fn state_values_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_state_values(&f, false).is_ok());
}
#[test]
fn state_values_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_state_values(&f, true).is_ok());
}

// ── validate_safety: unused machines ────────────────────────
#[test]
fn unused_machines_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_unused_machines(&f, false).is_ok());
}
#[test]
fn unused_machines_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_unused_machines(&f, true).is_ok());
}

// ── validate_safety: tag consistency ────────────────────────
#[test]
fn tag_consistency_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_tag_consistency(&f, false).is_ok());
}
#[test]
fn tag_consistency_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_tag_consistency(&f, true).is_ok());
}

// ── validate_safety: dependency exists ──────────────────────
#[test]
fn dependency_exists_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_dependency_exists(&f, false).is_ok());
}
#[test]
fn dependency_exists_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_dependency_exists(&f, true).is_ok());
}

// ── validate_safety: path conflicts ─────────────────────────
#[test]
fn path_conflicts_strict_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_path_conflicts_strict(&f, false).is_ok());
}
#[test]
fn path_conflicts_strict_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_path_conflicts_strict(&f, true).is_ok());
}

// ── validate_safety: duplicate names ────────────────────────
#[test]
fn duplicate_names_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_duplicate_names(&f, false).is_ok());
}
#[test]
fn duplicate_names_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_duplicate_names(&f, true).is_ok());
}

// ── validate_safety: resource groups ────────────────────────
#[test]
fn resource_groups_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_groups(&f, false).is_ok());
}
#[test]
fn resource_groups_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_groups(&f, true).is_ok());
}

// ── validate_advanced: orphan resources ─────────────────────
#[test]
fn orphan_resources_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_orphan_resources(&f, false).is_ok());
}
#[test]
fn orphan_resources_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_orphan_resources(&f, true).is_ok());
}

// ── validate_advanced: machine arch ─────────────────────────
#[test]
fn machine_arch_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_machine_arch(&f, false).is_ok());
}
#[test]
fn machine_arch_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_machine_arch(&f, true).is_ok());
}

// ── validate_advanced: health conflicts ─────────────────────
#[test]
fn health_conflicts_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_health_conflicts(&f, false).is_ok());
}
#[test]
fn health_conflicts_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_health_conflicts(&f, true).is_ok());
}

// ── validate_advanced: resource overlap ─────────────────────
#[test]
fn resource_overlap_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_overlap(&f, false).is_ok());
}
#[test]
fn resource_overlap_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_overlap(&f, true).is_ok());
}

// ── validate_advanced: resource tags ────────────────────────
#[test]
fn resource_tags_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tags(&f, false).is_ok());
}
#[test]
fn resource_tags_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tags(&f, true).is_ok());
}

// ── validate_advanced: state consistency ────────────────────
#[test]
fn state_consistency_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_state_consistency(&f, false).is_ok());
}
#[test]
fn state_consistency_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_state_consistency(&f, true).is_ok());
}

// ── validate_advanced: deps complete ────────────────────────
#[test]
fn deps_complete_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependencies_complete(&f, false).is_ok());
}
#[test]
fn deps_complete_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependencies_complete(&f, true).is_ok());
}

// ── validate_advanced: machine connectivity ─────────────────
#[test]
fn machine_connectivity_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_machine_connectivity(&f, false).is_ok());
}
#[test]
fn machine_connectivity_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_machine_connectivity(&f, true).is_ok());
}

// ── validate_ordering ───────────────────────────────────────
#[test]
fn dependency_ordering_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_ordering(&f, false).is_ok());
}
#[test]
fn dependency_ordering_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_ordering(&f, true).is_ok());
}
#[test]
fn tag_completeness_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_completeness(&f, false).is_ok());
}
#[test]
fn tag_completeness_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_tag_completeness(&f, true).is_ok());
}
#[test]
fn naming_standards_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_naming_standards(&f, false).is_ok());
}
#[test]
fn naming_standards_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_naming_standards(&f, true).is_ok());
}
#[test]
fn dependency_symmetry_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_symmetry(&f, false).is_ok());
}
#[test]
fn dependency_symmetry_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_symmetry(&f, true).is_ok());
}
#[test]
fn circular_alias_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_circular_alias(&f, false).is_ok());
}
#[test]
fn circular_alias_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_circular_alias(&f, true).is_ok());
}
#[test]
fn depth_limit_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_depth_limit(&f, false).is_ok());
}
#[test]
fn depth_limit_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_depth_limit(&f, true).is_ok());
}
#[test]
fn unused_params_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_unused_params(&f, false).is_ok());
}
#[test]
fn unused_params_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_unused_params(&f, true).is_ok());
}
#[test]
fn content_hash_consistency_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_content_hash_consistency(&f, false).is_ok());
}
#[test]
fn dependency_refs_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), deps_cfg());
    assert!(cmd_validate_check_resource_dependency_refs(&f, false).is_ok());
}
#[test]
fn trigger_refs_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_trigger_refs(&f, false).is_ok());
}
#[test]
fn param_type_safety_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_param_type_safety(&f, false).is_ok());
}
#[test]
fn machine_balance_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_machine_balance(&f, false).is_ok());
}
#[test]
fn machine_balance_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_machine_balance(&f, true).is_ok());
}
#[test]
fn env_consistency_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_env_consistency(&f, false).is_ok());
}
#[test]
fn secret_rotation_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_secret_rotation(&f, false).is_ok());
}
#[test]
fn lifecycle_completeness_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_lifecycle_completeness(&f, false).is_ok());
}
#[test]
fn provider_compatibility_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), base_cfg());
    assert!(cmd_validate_check_resource_provider_compatibility(&f, false).is_ok());
}
