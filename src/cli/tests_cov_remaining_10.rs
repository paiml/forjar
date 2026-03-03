//! Coverage tests: show.rs, store_cache, store_ops, store_archive gaps (FJ-1372).

#![allow(unused_imports)]
use super::show::*;
use super::store_archive::*;
use super::store_cache::*;
use super::store_ops::*;
use std::path::{Path, PathBuf};

fn write_cfg(dir: &Path, yaml: &str) -> PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn two_resource_cfg() -> &'static str {
    r#"version: "1.0"
name: compare-src
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "hello"
"#
}

fn changed_cfg() -> &'static str {
    r#"version: "1.0"
name: compare-dst
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "changed"
  svc:
    type: file
    machine: m1
    path: /etc/svc.conf
    content: "new-service"
"#
}

// ── cmd_compare ─────────────────────────────────────────────

#[test]
fn compare_text_added_removed_changed() {
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    let f1 = write_cfg(d1.path(), two_resource_cfg());
    let f2 = write_cfg(d2.path(), changed_cfg());
    assert!(cmd_compare(&f1, &f2, false).is_ok());
}

#[test]
fn compare_json_output() {
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    let f1 = write_cfg(d1.path(), two_resource_cfg());
    let f2 = write_cfg(d2.path(), changed_cfg());
    assert!(cmd_compare(&f1, &f2, true).is_ok());
}

#[test]
fn compare_identical_configs() {
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    let f1 = write_cfg(d1.path(), two_resource_cfg());
    let f2 = write_cfg(d2.path(), two_resource_cfg());
    assert!(cmd_compare(&f1, &f2, false).is_ok());
}

#[test]
fn compare_invalid_file() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), two_resource_cfg());
    assert!(cmd_compare(&f, Path::new("/nonexistent.yaml"), false).is_err());
}

// ── cmd_template ────────────────────────────────────────────

#[test]
fn template_text_with_vars() {
    let d = tempfile::tempdir().unwrap();
    let recipe = d.path().join("recipe.yaml");
    std::fs::write(&recipe, "name: {{inputs.app}}\nport: {{inputs.port}}\n").unwrap();
    assert!(cmd_template(
        &recipe,
        &["app=web".to_string(), "port=8080".to_string()],
        false
    )
    .is_ok());
}

#[test]
fn template_json_output() {
    let d = tempfile::tempdir().unwrap();
    let recipe = d.path().join("recipe.yaml");
    std::fs::write(&recipe, "name: {{inputs.svc}}\n").unwrap();
    assert!(cmd_template(&recipe, &["svc=api".to_string()], true).is_ok());
}

#[test]
fn template_no_vars() {
    let d = tempfile::tempdir().unwrap();
    let recipe = d.path().join("recipe.yaml");
    std::fs::write(&recipe, "static: content\n").unwrap();
    assert!(cmd_template(&recipe, &[], false).is_ok());
}

#[test]
fn template_invalid_file() {
    assert!(cmd_template(Path::new("/nonexistent.yaml"), &[], false).is_err());
}

// ── cmd_policy ──────────────────────────────────────────────

#[test]
fn policy_deny_violation_returns_err() {
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
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
"#,
    );
    let result = cmd_policy(&f, false);
    assert!(result.is_err());
}

#[test]
fn policy_deny_violation_json() {
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
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
"#,
    );
    let result = cmd_policy(&f, true);
    assert!(result.is_err());
}

#[test]
fn policy_warn_only_passes() {
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
policies:
  - type: warn
    message: "files should have owner"
    resource_type: file
    field: owner
"#,
    );
    assert!(cmd_policy(&f, false).is_ok());
}

// ── cmd_explain edge cases ──────────────────────────────────

#[test]
fn explain_with_deps_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: explain-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  base:
    type: file
    machine: local
    path: /tmp/base.txt
    content: "base"
  app:
    type: file
    machine: local
    path: /tmp/app.txt
    content: "app"
    depends_on: [base]
"#,
    );
    assert!(cmd_explain(&f, "app", false).is_ok());
}

#[test]
fn explain_ssh_transport() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: explain-ssh
machines:
  remote:
    hostname: web-1
    addr: 10.0.0.5
    ssh_key: /home/deploy/.ssh/id_ed25519
resources:
  cfg:
    type: file
    machine: remote
    path: /etc/app.conf
    content: "prod"
"#,
    );
    assert!(cmd_explain(&f, "cfg", false).is_ok());
}

#[test]
fn explain_ssh_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: explain-ssh
machines:
  remote:
    hostname: web-1
    addr: 10.0.0.5
    ssh_key: /home/deploy/.ssh/id_ed25519
resources:
  cfg:
    type: file
    machine: remote
    path: /etc/app.conf
    content: "prod"
    tags: [web]
    resource_group: frontend
"#,
    );
    assert!(cmd_explain(&f, "cfg", true).is_ok());
}
