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

#[test]
fn validate_format_errors_detected() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        r#"version: "1.0"
name: format-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  bad:
    type: file
    machine: m
    path: relative/path
    mode: "9999"
    owner: "Bad User"
"#,
    )
    .expect("write");

    let out = forjar()
        .args(["validate", "--file", cfg.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(!out.status.success(), "should fail on format errors");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid mode") || stderr.contains("must be absolute"),
        "should report format errors: {stderr}"
    );
}

#[test]
fn validate_unknown_fields_warned() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        r#"version: "1.0"
name: unknown-field-test
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
    packges: [typo]
"#,
    )
    .expect("write");

    let out = forjar()
        .args(["validate", "--file", cfg.to_str().unwrap()])
        .output()
        .expect("failed to run");
    // Should succeed (warnings only) but print warnings to stderr
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown field") || stderr.contains("packges"),
        "should warn about unknown field: {stderr}"
    );
}

#[test]
fn contracts_coverage_succeeds() {
    let out = forjar()
        .args(["contracts", "--coverage"])
        .output()
        .expect("failed to run");
    assert!(out.status.success(), "contracts --coverage should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Contract Coverage Report") || stdout.contains("total"));
}

#[test]
fn state_query_succeeds() {
    let out = forjar()
        .args(["state-query", "bash"])
        .output()
        .expect("failed to run");
    assert!(out.status.success(), "state-query should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Query:") || stdout.contains("bash"));
}

#[test]
fn oci_pack_missing_dir_fails() {
    let out = forjar()
        .args([
            "oci-pack",
            "/tmp/forjar-no-such-dir-999",
            "--tag",
            "test:v1",
        ])
        .output()
        .expect("failed to run");
    assert!(!out.status.success(), "oci-pack should fail on missing dir");
}

#[test]
fn logs_gc_succeeds() {
    let out = forjar()
        .args(["logs", "--gc"])
        .output()
        .expect("failed to run");
    assert!(out.status.success(), "logs --gc should succeed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("garbage collection") || stdout.contains("Log"));
}

#[test]
fn validate_deep_runs_all_checks() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        r#"version: "1.0"
name: deep-smoke
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
    )
    .expect("write");

    let out = forjar()
        .args(["validate", "--deep", "--file", cfg.to_str().unwrap()])
        .output()
        .expect("failed to run");
    assert!(
        out.status.success(),
        "deep validation should pass on valid config. stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Deep validation"),
        "should show deep validation summary: {stdout}"
    );
}
