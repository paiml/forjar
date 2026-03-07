//! Coverage tests for cli/run_task.rs — forjar run dispatch-mode task.

use std::io::Write;

fn write_task_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(yaml.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const TASK_YAML: &str = r#"
version: "1.0"
name: task-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  greet:
    type: task
    machine: m
    command: "echo hello {{ name }}"
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#;

#[test]
fn run_task_basic() {
    let f = write_task_config(TASK_YAML);
    let r = super::run_task::cmd_run(f.path(), "greet", &[], false);
    assert!(r.is_ok());
}

#[test]
fn run_task_with_params() {
    let f = write_task_config(TASK_YAML);
    let params = vec!["name=world".to_string()];
    let r = super::run_task::cmd_run(f.path(), "greet", &params, false);
    assert!(r.is_ok());
}

#[test]
fn run_task_json_mode() {
    let f = write_task_config(TASK_YAML);
    let r = super::run_task::cmd_run(f.path(), "greet", &[], true);
    assert!(r.is_ok());
}

#[test]
fn run_task_not_found() {
    let f = write_task_config(TASK_YAML);
    let r = super::run_task::cmd_run(f.path(), "nonexistent", &[], false);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("not found"));
}

#[test]
fn run_task_wrong_type() {
    let f = write_task_config(TASK_YAML);
    let r = super::run_task::cmd_run(f.path(), "pkg", &[], false);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("not a task"));
}

#[test]
fn run_task_invalid_param_format() {
    let f = write_task_config(TASK_YAML);
    let params = vec!["badparam".to_string()];
    let r = super::run_task::cmd_run(f.path(), "greet", &params, false);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("KEY=VALUE"));
}

#[test]
fn run_task_empty_config() {
    let f = write_task_config("");
    let r = super::run_task::cmd_run(f.path(), "task", &[], false);
    assert!(r.is_err());
}
