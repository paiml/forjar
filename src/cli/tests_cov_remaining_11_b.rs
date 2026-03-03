//! Coverage tests: lint, validate_policy, print_helpers gaps (FJ-1372).

#![allow(unused_imports)]
use super::lint::*;
use super::print_helpers::*;
use super::validate_policy::*;
use crate::core::types;
use std::path::{Path, PathBuf};

fn write_cfg(dir: &Path, yaml: &str) -> PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

#[test]
fn validate_policy_file_require_tags() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
"#,
    );
    let policy = d.path().join("policy.yaml");
    std::fs::write(
        &policy,
        r#"rules:
  - name: tags-required
    check: require_tags
"#,
    )
    .unwrap();
    let result = cmd_validate_policy_file(&f, &policy, false);
    assert!(result.is_err());
}

#[test]
fn validate_policy_file_require_depends_on() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
"#,
    );
    let policy = d.path().join("policy.yaml");
    std::fs::write(
        &policy,
        r#"rules:
  - name: deps-required
    check: require_depends_on
"#,
    )
    .unwrap();
    let result = cmd_validate_policy_file(&f, &policy, false);
    assert!(result.is_err());
}

#[test]
fn validate_policy_file_package_exempt_from_deps() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
    );
    let policy = d.path().join("policy.yaml");
    std::fs::write(
        &policy,
        r#"rules:
  - name: deps-required
    check: require_depends_on
"#,
    )
    .unwrap();
    // Package type is exempt from require_depends_on
    assert!(cmd_validate_policy_file(&f, &policy, false).is_ok());
}

#[test]
fn validate_policy_file_unknown_check() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines: {}
resources: {}
"#,
    );
    let policy = d.path().join("policy.yaml");
    std::fs::write(
        &policy,
        r#"rules:
  - name: bogus
    check: nonexistent_check
"#,
    )
    .unwrap();
    let result = cmd_validate_policy_file(&f, &policy, false);
    assert!(result.is_err());
}

#[test]
fn validate_policy_file_passes() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    owner: noah
    tags: [app]
    depends_on: []
"#,
    );
    let policy = d.path().join("policy.yaml");
    std::fs::write(
        &policy,
        r#"rules:
  - name: no-root
    check: no_root_owner
"#,
    )
    .unwrap();
    assert!(cmd_validate_policy_file(&f, &policy, false).is_ok());
}

#[test]
fn validate_policy_file_invalid_yaml() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines: {}
resources: {}
"#,
    );
    let policy = d.path().join("policy.yaml");
    std::fs::write(&policy, "not: [valid: yaml: {{").unwrap();
    let result = cmd_validate_policy_file(&f, &policy, false);
    assert!(result.is_err());
}

// ── validate: strict-deps ───────────────────────────────────

#[test]
fn validate_strict_deps_pass() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
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
  app:
    type: file
    machine: m1
    path: /tmp/app.txt
    content: "app"
    depends_on: [base]
"#,
    );
    assert!(cmd_validate_strict_deps(&f, false).is_ok());
}

#[test]
fn validate_strict_deps_violation() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  app:
    type: file
    machine: m1
    path: /tmp/app.txt
    content: "app"
    depends_on: [base]
  base:
    type: file
    machine: m1
    path: /tmp/base.txt
    content: "base"
"#,
    );
    let result = cmd_validate_strict_deps(&f, false);
    assert!(result.is_err());
}

#[test]
fn validate_strict_deps_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
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
"#,
    );
    assert!(cmd_validate_strict_deps(&f, true).is_ok());
}

// ── print_helpers ───────────────────────────────────────────

#[test]
fn print_content_diff_create_mode() {
    print_content_diff("line1\nline2\nline3\n", &types::PlanAction::Create, None);
}

#[test]
fn print_content_diff_update_no_old() {
    print_content_diff("new content\n", &types::PlanAction::Update, None);
}

#[test]
fn print_content_diff_noop() {
    print_content_diff("same\n", &types::PlanAction::NoOp, None);
}

#[test]
fn print_unified_diff_added_lines() {
    print_unified_diff("line1\n", "line1\nline2\nline3\n");
}

#[test]
fn print_unified_diff_removed_lines() {
    print_unified_diff("line1\nline2\n", "line1\n");
}

#[test]
fn print_unified_diff_identical() {
    print_unified_diff("same\n", "same\n");
}

#[test]
fn print_plan_with_filter() {
    let plan = types::ExecutionPlan {
        name: "test".to_string(),
        changes: vec![
            types::PlannedChange {
                resource_id: "cfg".to_string(),
                resource_type: types::ResourceType::File,
                machine: "m1".to_string(),
                action: types::PlanAction::Create,
                description: "create file".to_string(),
            },
            types::PlannedChange {
                resource_id: "pkg".to_string(),
                resource_type: types::ResourceType::Package,
                machine: "m2".to_string(),
                action: types::PlanAction::Update,
                description: "update package".to_string(),
            },
            types::PlannedChange {
                resource_id: "old".to_string(),
                resource_type: types::ResourceType::File,
                machine: "m1".to_string(),
                action: types::PlanAction::Destroy,
                description: "destroy old".to_string(),
            },
            types::PlannedChange {
                resource_id: "same".to_string(),
                resource_type: types::ResourceType::File,
                machine: "m1".to_string(),
                action: types::PlanAction::NoOp,
                description: "no change".to_string(),
            },
        ],
        to_create: 1,
        to_update: 1,
        to_destroy: 1,
        unchanged: 1,
        execution_order: vec![
            "cfg".to_string(),
            "pkg".to_string(),
            "old".to_string(),
            "same".to_string(),
        ],
    };
    // With machine filter
    print_plan(&plan, Some("m1"), None);
    // Without machine filter
    print_plan(&plan, None, None);
}
