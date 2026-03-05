//! Tests for cli/config_merge.rs — cmd_config_merge coverage.

use super::config_merge::*;
use std::io::Write;

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(yaml.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const CONFIG_A: &str = r#"
version: "1.0"
name: stack-a
machines:
  web:
    hostname: web
    addr: 10.0.0.1
resources:
  nginx:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
"#;

const CONFIG_B: &str = r#"
version: "1.0"
name: stack-b
machines:
  db:
    hostname: db
    addr: 10.0.0.2
resources:
  postgres:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
"#;

#[test]
fn test_config_merge_no_collision_stdout() {
    let a = write_temp_config(CONFIG_A);
    let b = write_temp_config(CONFIG_B);
    let result = cmd_config_merge(a.path(), b.path(), None, false);
    assert!(result.is_ok());
}

#[test]
fn test_config_merge_no_collision_to_file() {
    let a = write_temp_config(CONFIG_A);
    let b = write_temp_config(CONFIG_B);
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("merged.yaml");
    let result = cmd_config_merge(a.path(), b.path(), Some(&out), false);
    assert!(result.is_ok());
    assert!(out.exists());
    let content = std::fs::read_to_string(&out).unwrap();
    assert!(content.contains("stack-a"));
}

#[test]
fn test_config_merge_machine_collision_rejected() {
    let a = write_temp_config(CONFIG_A);
    // B has same machine name "web" as A
    let b_yaml = r#"
version: "1.0"
name: stack-b
machines:
  web:
    hostname: web2
    addr: 10.0.0.3
resources:
  curl:
    type: package
    machine: web
    provider: apt
    packages: [curl]
"#;
    let b = write_temp_config(b_yaml);
    let result = cmd_config_merge(a.path(), b.path(), None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("machine name collision"));
}

#[test]
fn test_config_merge_machine_collision_allowed() {
    let a = write_temp_config(CONFIG_A);
    let b_yaml = r#"
version: "1.0"
name: stack-b
machines:
  web:
    hostname: web2
    addr: 10.0.0.3
resources:
  curl:
    type: package
    machine: web
    provider: apt
    packages: [curl]
"#;
    let b = write_temp_config(b_yaml);
    let result = cmd_config_merge(a.path(), b.path(), None, true);
    assert!(result.is_ok());
}

#[test]
fn test_config_merge_resource_collision_rejected() {
    let a = write_temp_config(CONFIG_A);
    // B has same resource "nginx" as A
    let b_yaml = r#"
version: "1.0"
name: stack-b
machines:
  db:
    hostname: db
    addr: 10.0.0.2
resources:
  nginx:
    type: service
    machine: db
    name: nginx
"#;
    let b = write_temp_config(b_yaml);
    let result = cmd_config_merge(a.path(), b.path(), None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("resource ID collision"));
}

#[test]
fn test_config_merge_resource_collision_allowed() {
    let a = write_temp_config(CONFIG_A);
    let b_yaml = r#"
version: "1.0"
name: stack-b
machines:
  db:
    hostname: db
    addr: 10.0.0.2
resources:
  nginx:
    type: service
    machine: db
    name: nginx
"#;
    let b = write_temp_config(b_yaml);
    let result = cmd_config_merge(a.path(), b.path(), None, true);
    assert!(result.is_ok());
}

#[test]
fn test_config_merge_invalid_file() {
    let a = write_temp_config(CONFIG_A);
    let result = cmd_config_merge(
        a.path(),
        std::path::Path::new("/nonexistent/b.yaml"),
        None,
        false,
    );
    assert!(result.is_err());
}

#[test]
fn test_config_merge_params_and_outputs() {
    let a_yaml = r#"
version: "1.0"
name: a
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources: {}
params:
  env: production
outputs:
  url:
    value: "{{params.env}}"
"#;
    let b_yaml = r#"
version: "1.0"
name: b
machines:
  n:
    hostname: n
    addr: 127.0.0.2
resources: {}
params:
  region: us-east-1
outputs:
  api:
    value: "http://api.example.com"
"#;
    let a = write_temp_config(a_yaml);
    let b = write_temp_config(b_yaml);
    let result = cmd_config_merge(a.path(), b.path(), None, false);
    assert!(result.is_ok());
}
