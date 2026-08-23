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

/// A config whose `greet` task substitutes the bare `{{ name }}` dispatch
/// shorthand and writes the result to `dir/greeting.txt`.
///
/// STRENGTHENED: `run_task_basic`, `run_task_with_params` and
/// `run_task_json_mode` used `command: "echo hello {{ name }}"` and asserted
/// only `r.is_ok()`. Against the old code that assertion was true because the
/// literal seven characters `{{ name` were handed to the shell and `echo`
/// exits 0 whatever it prints — the exact defect, passing as a test. They now
/// name a value for the template and assert the text that reached the shell.
fn greeting_config(dir: &std::path::Path, param_line: &str) -> std::path::PathBuf {
    let out = dir.join("greeting.txt");
    let yaml = format!(
        r#"version: "1.0"
name: task-test
{param_line}machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  greet:
    type: task
    machine: m
    command: "echo hello {{{{ name }}}} > {}"
"#,
        out.display()
    );
    let path = dir.join("forjar.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

#[test]
fn run_task_basic() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = greeting_config(dir.path(), "params:\n  name: world\n");
    let r = super::run_task::cmd_run(&cfg, "greet", &[], false);
    assert!(r.is_ok(), "{r:?}");
    let body = std::fs::read_to_string(dir.path().join("greeting.txt")).unwrap();
    assert_eq!(body.trim(), "hello world", "got {body:?}");
}

#[test]
fn run_task_with_params() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = greeting_config(dir.path(), "");
    let params = vec!["name=world".to_string()];
    let r = super::run_task::cmd_run(&cfg, "greet", &params, false);
    assert!(r.is_ok(), "{r:?}");
    let body = std::fs::read_to_string(dir.path().join("greeting.txt")).unwrap();
    assert_eq!(body.trim(), "hello world", "--param never reached the shell");
}

#[test]
fn run_task_json_mode() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = greeting_config(dir.path(), "params:\n  name: world\n");
    let r = super::run_task::cmd_run(&cfg, "greet", &[], true);
    assert!(r.is_ok(), "{r:?}");
    let body = std::fs::read_to_string(dir.path().join("greeting.txt")).unwrap();
    assert_eq!(body.trim(), "hello world", "--json skipped execution");
}

/// A template with no value anywhere is an ERROR, not something to hand to a
/// shell verbatim and call `status: pass`.
#[test]
fn run_task_refuses_an_unresolvable_template() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = greeting_config(dir.path(), "");
    let r = super::run_task::cmd_run(&cfg, "greet", &[], false);
    let err = r.unwrap_err();
    assert!(err.contains("name"), "{err}");
    assert!(
        !dir.path().join("greeting.txt").exists(),
        "the unresolved command was executed anyway"
    );
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

// ========================================================================
// Ledger: run-executes-unresolved-templates-and-ignores-param
//         run-json-never-executes-and-masks-failures
//
// These assert THE EFFECT ON DISK, never the banner. `run` printing
// "status: pass" is exactly the defect: the message was the only thing that
// happened.
// ========================================================================

/// Write a config into `dir` whose `greet` task writes into `dir/out.txt`.
fn param_config(dir: &std::path::Path) -> std::path::PathBuf {
    let out = dir.join("out.txt");
    let yaml = format!(
        r#"version: "1.0"
name: run-params
params:
  who: alice
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  greet:
    type: task
    machine: m
    command: "echo hello {{{{params.who}}}} > {}"
"#,
        out.display()
    );
    let path = dir.join("forjar.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

#[test]
fn run_resolves_config_params_in_the_executed_command() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = param_config(dir.path());
    let out = dir.path().join("out.txt");

    super::run_task::cmd_run(&cfg, "greet", &[], false).unwrap();

    let body = std::fs::read_to_string(&out).expect("run must actually execute the task");
    assert!(
        !body.contains("{{"),
        "the LITERAL template reached the shell: {body:?}"
    );
    assert_eq!(body.trim(), "hello alice", "got {body:?}");
}

#[test]
fn run_param_override_reaches_the_executed_command() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = param_config(dir.path());
    let out = dir.path().join("out.txt");

    super::run_task::cmd_run(&cfg, "greet", &["who=bob".to_string()], false).unwrap();

    let body = std::fs::read_to_string(&out).expect("run must actually execute the task");
    assert_eq!(body.trim(), "hello bob", "--param was ignored: {body:?}");
}

/// A config with a side-effecting task and a task that fails with exit 3.
fn effect_config(dir: &std::path::Path) -> std::path::PathBuf {
    let effect = dir.join("effect.txt");
    let yaml = format!(
        r#"version: "1.0"
name: run-json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  side-effect:
    type: task
    machine: m
    command: "echo done > {}"
  boom:
    type: task
    machine: m
    command: "exit 3"
"#,
        effect.display()
    );
    let path = dir.join("forjar.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

#[test]
fn run_json_still_executes_the_task() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = effect_config(dir.path());
    let effect = dir.path().join("effect.txt");

    super::run_task::cmd_run(&cfg, "side-effect", &[], true).unwrap();

    assert!(
        effect.exists(),
        "--json is an output FORMAT, not a mode change: the task never ran"
    );
}

#[test]
fn run_json_propagates_a_failing_task() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = effect_config(dir.path());

    let plain = super::run_task::cmd_run(&cfg, "boom", &[], false);
    assert!(plain.is_err(), "plain form must fail on exit 3");

    let json = super::run_task::cmd_run(&cfg, "boom", &[], true);
    assert!(
        json.is_err(),
        "--json masked a failing task's exit code (plain form: {plain:?})"
    );
}
