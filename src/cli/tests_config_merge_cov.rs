//! Coverage tests for config_merge.rs — merge configs with collision detection.

use super::config_merge::*;
use std::path::Path;

fn write_config(dir: &Path, name: &str, yaml: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, yaml).unwrap();
    p
}

const BASE_A: &str = r#"
version: "1.0"
name: app-a
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg-a:
    type: file
    machine: web
    path: /tmp/a
    content: "a"
"#;

const BASE_B: &str = r#"
version: "1.0"
name: app-b
machines:
  db:
    hostname: db
    addr: 127.0.0.1
resources:
  cfg-b:
    type: file
    machine: db
    path: /tmp/b
    content: "b"
"#;

#[test]
fn merge_no_collisions() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_config(dir.path(), "a.yaml", BASE_A);
    let b = write_config(dir.path(), "b.yaml", BASE_B);
    let result = cmd_config_merge(&a, &b, None, false);
    assert!(result.is_ok());
}

#[test]
fn merge_to_output_file() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_config(dir.path(), "a.yaml", BASE_A);
    let b = write_config(dir.path(), "b.yaml", BASE_B);
    let out = dir.path().join("merged.yaml");
    let result = cmd_config_merge(&a, &b, Some(&out), false);
    assert!(result.is_ok());
    assert!(out.exists());
    let content = std::fs::read_to_string(&out).unwrap();
    assert!(content.contains("web"));
    assert!(content.contains("db"));
}

#[test]
fn merge_machine_collision_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_config(dir.path(), "a.yaml", BASE_A);
    let b_collision = r#"
version: "1.0"
name: app-b
machines:
  web:
    hostname: web2
    addr: 10.0.0.1
resources:
  cfg-b:
    type: file
    machine: web
    path: /tmp/b
    content: "b"
"#;
    let b = write_config(dir.path(), "b.yaml", b_collision);
    let result = cmd_config_merge(&a, &b, None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("machine name collision"));
}

#[test]
fn merge_machine_collision_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_config(dir.path(), "a.yaml", BASE_A);
    let b_collision = r#"
version: "1.0"
name: app-b
machines:
  web:
    hostname: web2
    addr: 10.0.0.1
resources:
  cfg-b:
    type: file
    machine: web
    path: /tmp/b
    content: "b"
"#;
    let b = write_config(dir.path(), "b.yaml", b_collision);
    let result = cmd_config_merge(&a, &b, None, true);
    assert!(result.is_ok());
}

#[test]
fn merge_resource_collision_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_config(dir.path(), "a.yaml", BASE_A);
    let b_collision = r#"
version: "1.0"
name: app-b
machines:
  db:
    hostname: db
    addr: 127.0.0.1
resources:
  cfg-a:
    type: file
    machine: db
    path: /tmp/b
    content: "b"
"#;
    let b = write_config(dir.path(), "b.yaml", b_collision);
    let result = cmd_config_merge(&a, &b, None, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("resource ID collision"));
}

#[test]
fn merge_resource_collision_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_config(dir.path(), "a.yaml", BASE_A);
    let b_collision = r#"
version: "1.0"
name: app-b
machines:
  db:
    hostname: db
    addr: 127.0.0.1
resources:
  cfg-a:
    type: file
    machine: db
    path: /tmp/override
    content: "override"
"#;
    let b = write_config(dir.path(), "b.yaml", b_collision);
    let result = cmd_config_merge(&a, &b, None, true);
    assert!(result.is_ok());
}

#[test]
fn merge_invalid_config_a() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_config(dir.path(), "a.yaml", "{{{invalid");
    let b = write_config(dir.path(), "b.yaml", BASE_B);
    let result = cmd_config_merge(&a, &b, None, false);
    assert!(result.is_err());
}

#[test]
fn merge_invalid_config_b() {
    let dir = tempfile::tempdir().unwrap();
    let a = write_config(dir.path(), "a.yaml", BASE_A);
    let b = write_config(dir.path(), "b.yaml", "{{{invalid");
    let result = cmd_config_merge(&a, &b, None, false);
    assert!(result.is_err());
}
