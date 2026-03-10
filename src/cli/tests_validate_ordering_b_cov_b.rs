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
