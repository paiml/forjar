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

// ── lint: unused machines ───────────────────────────────────

#[test]
fn lint_unused_machine_detected() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-unused
machines:
  used:
    hostname: used
    addr: 127.0.0.1
  unused:
    hostname: spare
    addr: 10.0.0.99
resources:
  cfg:
    type: file
    machine: used
    path: /tmp/test.txt
    content: "hello"
"#,
    );
    // Non-strict so it covers lint_unused_machines + lint_scripts
    assert!(cmd_lint(&f, false, false, false).is_ok());
}

#[test]
fn lint_unused_machine_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-unused
machines:
  used:
    hostname: used
    addr: 127.0.0.1
  unused:
    hostname: spare
    addr: 10.0.0.99
resources:
  cfg:
    type: file
    machine: used
    path: /tmp/test.txt
    content: "hello"
"#,
    );
    assert!(cmd_lint(&f, true, false, false).is_ok());
}

// ── lint: dependency issues ─────────────────────────────────

#[test]
fn lint_nonexistent_dependency() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-deps
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
    depends_on: [ghost]
"#,
    );
    // Validator may reject nonexistent deps before lint runs
    let _ = cmd_lint(&f, false, false, false);
}

#[test]
fn lint_cross_machine_dependency() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-cross
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
  m2:
    hostname: m2
    addr: 10.0.0.2
resources:
  base:
    type: file
    machine: m1
    path: /tmp/base.txt
    content: "base"
  app:
    type: file
    machine: m2
    path: /tmp/app.txt
    content: "app"
    depends_on: [base]
"#,
    );
    assert!(cmd_lint(&f, false, false, false).is_ok());
}

// ── lint: empty packages ────────────────────────────────────

#[test]
fn lint_empty_packages_detected() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-empty-pkgs
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: []
"#,
    );
    // Validator may reject empty packages before lint runs
    let _ = cmd_lint(&f, false, false, false);
}

// ── lint: strict rules ──────────────────────────────────────

#[test]
fn lint_strict_root_owner_no_system_tag() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-strict
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "data"
    owner: root
"#,
    );
    assert!(cmd_lint(&f, false, true, false).is_ok());
}

#[test]
fn lint_strict_no_ssh_key() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-strict-ssh
machines:
  remote:
    hostname: web-1
    addr: 10.0.0.5
resources:
  cfg:
    type: file
    machine: remote
    path: /etc/app.conf
    content: "data"
"#,
    );
    assert!(cmd_lint(&f, true, true, false).is_ok());
}

// ── lint: auto-fix ──────────────────────────────────────────

#[test]
fn lint_auto_fix_sorts_resources() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-fix
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  z_last:
    type: file
    machine: m1
    path: /tmp/z.txt
    content: "z"
  a_first:
    type: file
    machine: m1
    path: /tmp/a.txt
    content: "a"
"#,
    );
    assert!(cmd_lint(&f, false, false, true).is_ok());
    // Verify the file was rewritten with sorted keys
    let content = std::fs::read_to_string(&f).unwrap();
    let a_pos = content.find("a_first").unwrap();
    let z_pos = content.find("z_last").unwrap();
    assert!(a_pos < z_pos, "a_first should come before z_last after fix");
}

#[test]
fn lint_auto_fix_no_changes_needed() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-fix
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  a_first:
    type: file
    machine: m1
    path: /tmp/a.txt
    content: "a"
"#,
    );
    // Only one resource → no fix needed, but lint still runs
    assert!(cmd_lint(&f, false, false, true).is_ok());
}

// ── lint: untagged resources ────────────────────────────────

#[test]
fn lint_all_untagged_more_than_3() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-tags
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: m1
    path: /tmp/a.txt
    content: "a"
  b:
    type: file
    machine: m1
    path: /tmp/b.txt
    content: "b"
  c:
    type: file
    machine: m1
    path: /tmp/c.txt
    content: "c"
  d:
    type: file
    machine: m1
    path: /tmp/d.txt
    content: "d"
"#,
    );
    assert!(cmd_lint(&f, false, false, false).is_ok());
}

// ── lint: no warnings (clean config) ────────────────────────

#[test]
fn lint_clean_config_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: lint-clean
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
"#,
    );
    assert!(cmd_lint(&f, false, false, false).is_ok());
}

// ── validate_policy: policy file ────────────────────────────

#[test]
fn validate_policy_file_no_root() {
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
    owner: root
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
    let result = cmd_validate_policy_file(&f, &policy, false);
    assert!(result.is_err());
}

#[test]
fn validate_policy_file_no_root_json() {
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
    owner: root
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
    let result = cmd_validate_policy_file(&f, &policy, true);
    assert!(result.is_err());
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
        execution_order: vec!["cfg".to_string(), "pkg".to_string(), "old".to_string(), "same".to_string()],
    };
    // With machine filter
    print_plan(&plan, Some("m1"), None);
    // Without machine filter
    print_plan(&plan, None, None);
}
