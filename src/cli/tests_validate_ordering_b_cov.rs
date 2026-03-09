//! Coverage tests for validate_ordering_b.rs — 8 cmd_validate functions.

use super::validate_ordering_b::*;
use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

// ── dependency refs ─────────────────────────────────────────────────

#[test]
fn dep_refs_all_valid() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: package
    machine: m
    provider: apt
    packages: [curl]
  b:
    type: package
    machine: m
    provider: apt
    packages: [vim]
    depends_on: [a]
"#,
    );
    assert!(cmd_validate_check_resource_dependency_refs(&p, false).is_ok());
}

#[test]
fn dep_refs_missing_text() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: package
    machine: m
    provider: apt
    packages: [curl]
    depends_on: [nonexistent]
"#,
    );
    assert!(cmd_validate_check_resource_dependency_refs(&p, false).is_ok());
}

#[test]
fn dep_refs_missing_json() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: package
    machine: m
    provider: apt
    packages: [curl]
    depends_on: [nonexistent]
"#,
    );
    assert!(cmd_validate_check_resource_dependency_refs(&p, true).is_ok());
}

#[test]
fn dep_refs_trigger_missing() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  a:
    type: package
    machine: m
    provider: apt
    packages: [curl]
    triggers: [missing-trigger]
"#,
    );
    assert!(cmd_validate_check_resource_dependency_refs(&p, false).is_ok());
}

// ── trigger refs ────────────────────────────────────────────────────

#[test]
fn trigger_refs_all_valid() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  svc:
    type: service
    machine: m
    service_name: nginx
  cfg:
    type: file
    machine: m
    path: /etc/nginx/nginx.conf
    content: "server {}"
    triggers: [svc]
"#,
    );
    assert!(cmd_validate_check_resource_trigger_refs(&p, false).is_ok());
}

#[test]
fn trigger_refs_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/test
    content: "test"
    triggers: [no-such-resource]
"#,
    );
    assert!(cmd_validate_check_resource_trigger_refs(&p, true).is_ok());
}

// ── param type safety ───────────────────────────────────────────────

#[test]
fn param_type_valid_port() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
params:
  http_port: "8080"
  data_path: "/var/data"
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources: {}
"#,
    );
    assert!(cmd_validate_check_resource_param_type_safety(&p, false).is_ok());
}

#[test]
fn param_type_bad_port() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
params:
  api_port: "not-a-number"
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources: {}
"#,
    );
    assert!(cmd_validate_check_resource_param_type_safety(&p, false).is_ok());
}

#[test]
fn param_type_bad_path_json() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
params:
  log_path: "relative-no-slash"
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources: {}
"#,
    );
    assert!(cmd_validate_check_resource_param_type_safety(&p, true).is_ok());
}

#[test]
fn param_type_relative_path_ok() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
params:
  config_path: "./config"
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources: {}
"#,
    );
    assert!(cmd_validate_check_resource_param_type_safety(&p, false).is_ok());
}

// ── machine balance ─────────────────────────────────────────────────

#[test]
fn machine_balance_balanced() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 127.0.0.1
  b:
    hostname: b
    addr: 127.0.0.1
resources:
  r1:
    type: package
    machine: a
    provider: apt
    packages: [curl]
  r2:
    type: package
    machine: b
    provider: apt
    packages: [vim]
"#,
    );
    assert!(cmd_validate_check_resource_machine_balance(&p, false).is_ok());
}

#[test]
fn machine_balance_imbalanced_json() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 127.0.0.1
  b:
    hostname: b
    addr: 127.0.0.1
resources:
  r1:
    type: package
    machine: a
    provider: apt
    packages: [curl]
  r2:
    type: package
    machine: a
    provider: apt
    packages: [vim]
  r3:
    type: package
    machine: a
    provider: apt
    packages: [git]
"#,
    );
    assert!(cmd_validate_check_resource_machine_balance(&p, true).is_ok());
}

#[test]
fn machine_balance_empty_resources() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources: {}
"#,
    );
    assert!(cmd_validate_check_resource_machine_balance(&p, false).is_ok());
}

// ── env consistency ─────────────────────────────────────────────────

#[test]
fn env_consistency_all_declared() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
params:
  db_host: localhost
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/app.conf
    content: "host={{db_host}}"
"#,
    );
    assert!(cmd_validate_check_resource_env_consistency(&p, false).is_ok());
}

#[test]
fn env_consistency_undeclared_json() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/app.conf
    content: "host={{undefined_var}}"
"#,
    );
    assert!(cmd_validate_check_resource_env_consistency(&p, true).is_ok());
}

#[test]
fn env_consistency_no_content() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#,
    );
    assert!(cmd_validate_check_resource_env_consistency(&p, false).is_ok());
}

// ── secret rotation ─────────────────────────────────────────────────

#[test]
fn secret_rotation_tagged() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  db-password:
    type: file
    machine: m
    path: /etc/secrets/db
    content: "secret123"
    tags: [rotation-90d]
"#,
    );
    assert!(cmd_validate_check_resource_secret_rotation(&p, false).is_ok());
}

#[test]
fn secret_rotation_missing_tags_json() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  api-token:
    type: file
    machine: m
    path: /etc/secrets/api
    content: "tok123"
"#,
    );
    assert!(cmd_validate_check_resource_secret_rotation(&p, true).is_ok());
}

#[test]
fn secret_rotation_non_secret_ok() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  nginx-config:
    type: file
    machine: m
    path: /etc/nginx/nginx.conf
    content: "server {}"
"#,
    );
    assert!(cmd_validate_check_resource_secret_rotation(&p, false).is_ok());
}

// ── lifecycle completeness ──────────────────────────────────────────

#[test]
fn lifecycle_complete() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/app.conf
    content: "data"
    tags: [config]
"#,
    );
    assert!(cmd_validate_check_resource_lifecycle_completeness(&p, false).is_ok());
}

#[test]
fn lifecycle_incomplete_json() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  bare:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#,
    );
    assert!(cmd_validate_check_resource_lifecycle_completeness(&p, true).is_ok());
}

// ── provider compatibility ──────────────────────────────────────────

#[test]
fn provider_compat_standard_types() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
  cfg:
    type: file
    machine: m
    path: /etc/test
    content: "test"
  svc:
    type: service
    machine: m
    service_name: nginx
"#,
    );
    assert!(cmd_validate_check_resource_provider_compatibility(&p, false).is_ok());
}

#[test]
fn provider_compat_docker_type_json() {
    let dir = tempfile::tempdir().unwrap();
    let p = write_config(
        dir.path(),
        r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  app:
    type: docker
    machine: m
    name: myapp
    version: latest
"#,
    );
    assert!(cmd_validate_check_resource_provider_compatibility(&p, true).is_ok());
}
