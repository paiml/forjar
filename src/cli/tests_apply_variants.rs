//! Tests: Apply variants — dry-run graph, dry-run cost, canary.

use super::apply_variants::*;

fn make_config(dir: &std::path::Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

const MULTI_MACHINE_CONFIG: &str = r#"version: "1.0"
name: multi
machines:
  web:
    hostname: web
    addr: 127.0.0.1
  db:
    hostname: db
    addr: 127.0.0.1
resources:
  web-pkg:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
  db-pkg:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
    depends_on: [web-pkg]
"#;

const SIMPLE_CONFIG: &str = r#"version: "1.0"
name: simple
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
    path: /tmp/test.conf
    content: "test"
    depends_on: [pkg]
"#;

#[test]
fn dry_run_graph_shows_resources() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(dir.path(), SIMPLE_CONFIG);
    let result = cmd_apply_dry_run_graph(&file);
    assert!(result.is_ok());
}

#[test]
fn dry_run_graph_multi_machine() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(dir.path(), MULTI_MACHINE_CONFIG);
    let result = cmd_apply_dry_run_graph(&file);
    assert!(result.is_ok());
}

#[test]
fn dry_run_graph_empty_resources() {
    let dir = tempfile::tempdir().unwrap();
    let file = make_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let result = cmd_apply_dry_run_graph(&file);
    assert!(result.is_ok());
}

#[test]
fn dry_run_cost_basic() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = make_config(dir.path(), SIMPLE_CONFIG);
    let result = cmd_apply_dry_run_cost(&file, &state_dir, None);
    assert!(result.is_ok());
}

#[test]
fn dry_run_cost_with_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = make_config(dir.path(), MULTI_MACHINE_CONFIG);
    let result = cmd_apply_dry_run_cost(&file, &state_dir, Some("web"));
    assert!(result.is_ok());
}

#[test]
fn canary_machine_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let file = make_config(dir.path(), MULTI_MACHINE_CONFIG);
    let result = cmd_apply_canary_machine(&file, &state_dir, "nonexistent", &[], None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn dry_run_graph_nonexistent_config() {
    let result = cmd_apply_dry_run_graph(std::path::Path::new("/nonexistent.yaml"));
    assert!(result.is_err());
}

#[test]
fn dry_run_cost_nonexistent_config() {
    let dir = tempfile::tempdir().unwrap();
    let result = cmd_apply_dry_run_cost(
        std::path::Path::new("/nonexistent.yaml"),
        dir.path(),
        None,
    );
    assert!(result.is_err());
}
