//! Smoke-level integration tests that exercise the forjar binary end-to-end.
//!
//! These tests invoke the compiled binary via `std::process::Command` and
//! verify exit codes and stdout/stderr for core CLI entry-points.

use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

#[test]
fn version_flag_prints_version() {
    let out = forjar().arg("--version").output().expect("failed to run");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("forjar"),
        "expected 'forjar' in version output, got: {stdout}"
    );
}

#[test]
fn help_flag_prints_usage() {
    let out = forjar().arg("--help").output().expect("failed to run");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage"), "expected 'Usage' in help output");
    assert!(
        stdout.contains("validate"),
        "expected 'validate' subcommand in help"
    );
    assert!(
        stdout.contains("init"),
        "expected 'init' subcommand in help"
    );
}

#[test]
fn validate_valid_config() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        r#"version: "1.0"
name: smoke-test
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
resources:
  hello:
    type: file
    machine: localhost
    path: /tmp/smoke.txt
    content: "hello"
    owner: root
    mode: "0644"
"#,
    )
    .expect("write config");

    let out = forjar()
        .args(["validate", "--file", cfg.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(
        out.status.success(),
        "validate should succeed on a valid config. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn validate_invalid_yaml_fails() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let cfg = dir.path().join("bad.yaml");
    std::fs::write(&cfg, "{{{{not valid yaml at all!!!!").expect("write");

    let out = forjar()
        .args(["validate", "--file", cfg.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(
        !out.status.success(),
        "validate should fail on invalid YAML"
    );
}

#[test]
fn validate_missing_file_fails() {
    let out = forjar()
        .args(["validate", "--file", "/tmp/forjar-nonexistent-999.yaml"])
        .output()
        .expect("failed to run");
    assert!(
        !out.status.success(),
        "validate should fail when file does not exist"
    );
}

#[test]
fn init_creates_config_and_state() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let out = forjar()
        .args(["init", dir.path().to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(
        out.status.success(),
        "init should succeed in empty dir. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let config = dir.path().join("forjar.yaml");
    assert!(config.exists(), "init should create forjar.yaml");

    let state = dir.path().join("state");
    assert!(state.exists(), "init should create state/ directory");

    let content = std::fs::read_to_string(&config).expect("read config");
    assert!(content.contains("version:"), "config should have version");
    assert!(
        content.contains("machines:"),
        "config should have machines section"
    );
}

#[test]
fn init_refuses_existing_config() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let first = forjar()
        .args(["init", dir.path().to_str().unwrap()])
        .output()
        .expect("run");
    assert!(first.status.success());

    // Second init should fail because forjar.yaml already exists.
    let second = forjar()
        .args(["init", dir.path().to_str().unwrap()])
        .output()
        .expect("run");
    assert!(
        !second.status.success(),
        "init should refuse when forjar.yaml already exists"
    );
}
