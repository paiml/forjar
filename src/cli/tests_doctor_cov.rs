//! Coverage tests for doctor.rs — check_state_dir_existence, check_stale_lock,
//! output formatting, cmd_doctor_with_writer edge cases.

use super::doctor::*;
use super::output::TestWriter;

// ── cmd_doctor_with_writer: state dir checks ────────────────────────

#[test]
fn doctor_writer_captures_state_dir_pass() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(
        &file,
        "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
    ).unwrap();
    let mut w = TestWriter::new();
    let _ = cmd_doctor_with_writer(Some(&file), false, false, &mut w);
    // Should produce output without crashing
    assert!(!w.stderr.is_empty() || !w.stdout.is_empty());
}

#[test]
fn doctor_writer_json_mode() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(
        &file,
        "version: \"1.0\"\nname: test\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
    ).unwrap();
    let mut w = TestWriter::new();
    let _ = cmd_doctor_with_writer(Some(&file), true, false, &mut w);
    let out = w.stdout_text();
    // JSON output should contain brackets
    assert!(out.contains('[') || out.contains('{'));
}

#[test]
fn doctor_writer_no_config() {
    let mut w = TestWriter::new();
    let result = cmd_doctor_with_writer(None, false, false, &mut w);
    assert!(result.is_ok());
}

#[test]
fn doctor_writer_bad_config_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(&file, "{{{invalid").unwrap();
    let mut w = TestWriter::new();
    let result = cmd_doctor_with_writer(Some(&file), false, false, &mut w);
    assert!(result.is_err());
    let out = w.stderr_text();
    assert!(out.contains("config") || out.contains("parse"));
}

#[test]
fn doctor_writer_fix_mode_no_crash() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(
        &file,
        "version: \"1.0\"\nname: fix\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n",
    ).unwrap();
    let mut w = TestWriter::new();
    let _ = cmd_doctor_with_writer(Some(&file), false, true, &mut w);
}

// ── Config with SSH machines triggers SSH check ─────────────────────

#[test]
fn doctor_ssh_machine_check() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(
        &file,
        r#"version: "1.0"
name: ssh-test
machines:
  remote:
    hostname: remote
    addr: 10.0.0.1
    user: deploy
resources:
  f:
    type: file
    machine: remote
    path: /tmp/test
    content: test
"#,
    )
    .unwrap();
    let mut w = TestWriter::new();
    let _ = cmd_doctor_with_writer(Some(&file), false, false, &mut w);
    // SSH check should be included in output
    let out = w.stderr_text();
    assert!(out.contains("ssh") || w.stdout_text().contains("ssh"));
}

// ── Config with container machines triggers container check ─────────

#[test]
fn doctor_container_machine_check() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(
        &file,
        r#"version: "1.0"
name: ct-test
machines:
  box:
    hostname: box
    addr: container
    transport: container
    container:
      image: ubuntu:22.04
resources:
  f:
    type: file
    machine: box
    path: /tmp/test
    content: test
"#,
    )
    .unwrap();
    let mut w = TestWriter::new();
    let _ = cmd_doctor_with_writer(Some(&file), true, false, &mut w);
    let out = w.stdout_text();
    // JSON output should have check entries
    assert!(out.contains('['));
}

// ── JSON output formatting ──────────────────────────────────────────

#[test]
fn doctor_json_output_has_name_status_detail() {
    let mut w = TestWriter::new();
    let _ = cmd_doctor_with_writer(None, true, false, &mut w);
    let out = w.stdout_text();
    assert!(out.contains("name"));
    assert!(out.contains("status"));
    assert!(out.contains("detail"));
}

#[test]
fn doctor_text_output_has_summary_counts() {
    let mut w = TestWriter::new();
    let _ = cmd_doctor_with_writer(None, false, false, &mut w);
    let out = w.stdout_text();
    assert!(out.contains("checks:") || out.contains("pass"));
}

// ── Network subcommand ──────────────────────────────────────────────

#[test]
fn doctor_network_localhost_text() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(
        &file,
        r#"version: "1.0"
name: net
machines:
  local:
    hostname: local
    addr: localhost
resources:
  f:
    type: file
    machine: local
    path: /tmp/test
    content: test
"#,
    )
    .unwrap();
    let result = cmd_doctor_network(Some(&file), false);
    assert!(result.is_ok());
}

#[test]
fn doctor_network_localhost_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(
        &file,
        r#"version: "1.0"
name: net
machines:
  local:
    hostname: local
    addr: localhost
resources:
  f:
    type: file
    machine: local
    path: /tmp/test
    content: test
"#,
    )
    .unwrap();
    let result = cmd_doctor_network(Some(&file), true);
    assert!(result.is_ok());
}
