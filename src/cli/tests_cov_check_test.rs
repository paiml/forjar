//! Coverage tests for cli/check_test.rs — print functions, artifacts, behavior/mutation/convergence.

use super::check_test::*;
use super::check_test_runners::{cmd_test_coverage, RunnerOpts};
use std::io::Write;

fn sample_rows() -> Vec<TestRow> {
    vec![
        TestRow {
            resource_id: "nginx-pkg".into(),
            machine: "web1".into(),
            resource_type: "package".into(),
            status: "pass".into(),
            detail: String::new(),
            duration_secs: 1.5,
        },
        TestRow {
            resource_id: "app-config".into(),
            machine: "web1".into(),
            resource_type: "file".into(),
            status: "FAIL".into(),
            detail: "exit 1".into(),
            duration_secs: 0.3,
        },
        TestRow {
            resource_id: "svc-nginx".into(),
            machine: "web2".into(),
            resource_type: "service".into(),
            status: "skip".into(),
            detail: "no check script".into(),
            duration_secs: 0.0,
        },
    ]
}

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(yaml.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

const BASIC_YAML: &str = r#"
version: "1.0"
name: test-stack
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [nginx]
  svc:
    type: service
    machine: m
    name: nginx
    depends_on: [pkg]
"#;

#[test]
fn test_print_table_all_pass() {
    let rows = vec![sample_rows()[0].clone()];
    let elapsed = std::time::Duration::from_secs_f64(1.5);
    print_test_table(&rows, 1, 0, 0, &elapsed);
}

#[test]
fn test_print_table_mixed() {
    let rows = sample_rows();
    let elapsed = std::time::Duration::from_secs_f64(2.0);
    print_test_table(&rows, 1, 1, 1, &elapsed);
}

#[test]
fn test_print_table_all_fail() {
    let rows = vec![sample_rows()[1].clone()];
    let elapsed = std::time::Duration::from_secs_f64(0.3);
    print_test_table(&rows, 0, 1, 0, &elapsed);
}

#[test]
fn test_print_table_empty() {
    let elapsed = std::time::Duration::from_secs_f64(0.0);
    print_test_table(&[], 0, 0, 0, &elapsed);
}

#[test]
fn test_print_json_all_pass() {
    let rows = vec![sample_rows()[0].clone()];
    let elapsed = std::time::Duration::from_secs_f64(1.5);
    let r = print_test_json(&rows, 1, 0, 0, &elapsed);
    assert!(r.is_ok());
}

#[test]
fn test_print_json_mixed() {
    let rows = sample_rows();
    let elapsed = std::time::Duration::from_secs_f64(2.0);
    let r = print_test_json(&rows, 1, 1, 1, &elapsed);
    assert!(r.is_ok());
}

#[test]
fn test_print_json_empty() {
    let elapsed = std::time::Duration::from_secs_f64(0.0);
    let r = print_test_json(&[], 0, 0, 0, &elapsed);
    assert!(r.is_ok());
}

#[test]
fn test_collect_artifacts_empty() {
    let dir = tempfile::tempdir().unwrap();
    let artifact_dir = dir.path().join("artifacts");
    let artifacts = collect_test_artifacts(&[], &artifact_dir);
    assert_eq!(artifacts.len(), 1);
    assert!(artifact_dir.join("test-results.json").exists());
}

#[test]
fn test_collect_artifacts_with_data() {
    let dir = tempfile::tempdir().unwrap();
    let artifact_dir = dir.path().join("artifacts");
    let rows = sample_rows();
    let artifacts = collect_test_artifacts(&rows, &artifact_dir);
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].name, "test-results.json");
    assert!(artifacts[0].size_bytes.unwrap() > 0);
    let content = std::fs::read_to_string(artifact_dir.join("test-results.json")).unwrap();
    assert!(content.contains("nginx-pkg"));
    assert!(content.contains("FAIL"));
}

#[test]
fn test_behavior_mode_no_specs() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "").unwrap();
    let r = cmd_test_behavior(&config_path);
    assert!(r.is_ok());
}

#[test]
fn test_behavior_mode_with_spec_file() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::write(dir.path().join("nginx.spec.yaml"), "name: nginx\nconfig: forjar.yaml\nbehaviors:\n  - name: installed\n    state: present\n").unwrap();
    let r = cmd_test_behavior(&config_path);
    assert!(r.is_ok());
}

#[test]
fn test_mutation_mode() {
    let f = write_temp_config(BASIC_YAML);
    let r = cmd_test_mutation(f.path(), &RunnerOpts::default());
    assert!(r.is_ok());
}

#[test]
fn test_convergence_mode() {
    let f = write_temp_config(BASIC_YAML);
    let r = cmd_test_convergence(f.path(), &RunnerOpts::default());
    assert!(r.is_ok());
}

#[test]
fn test_row_clone() {
    let row = TestRow {
        resource_id: "r1".into(),
        machine: "m1".into(),
        resource_type: "file".into(),
        status: "pass".into(),
        detail: String::new(),
        duration_secs: 0.5,
    };
    assert_eq!(row.resource_id, "r1");
    assert_eq!(row.machine, "m1");
    assert_eq!(row.status, "pass");
    assert_eq!(row.duration_secs, 0.5);
}

/// F15: Behavior specs now actually execute verify commands.
#[test]
fn test_behavior_mode_with_verify_command() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::write(
        dir.path().join("check.spec.yaml"),
        "name: verify-test\nconfig: forjar.yaml\nbehaviors:\n  - name: echo works\n    verify:\n      command: echo hello\n      stdout: hello\n  - name: true exits 0\n    verify:\n      command: \"true\"\n      exit_code: 0\n",
    )
    .unwrap();
    let r = cmd_test_behavior(&config_path);
    assert!(r.is_ok());
}

#[test]
fn test_behavior_mode_verify_failure() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::write(
        dir.path().join("fail.spec.yaml"),
        "name: fail-test\nconfig: forjar.yaml\nbehaviors:\n  - name: bad exit\n    verify:\n      command: \"false\"\n      exit_code: 0\n",
    )
    .unwrap();
    let r = cmd_test_behavior(&config_path);
    assert!(r.is_err());
}

#[test]
fn test_behavior_mode_verify_stdout_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::write(
        dir.path().join("mismatch.spec.yaml"),
        "name: mismatch-test\nconfig: forjar.yaml\nbehaviors:\n  - name: wrong output\n    verify:\n      command: echo wrong\n      stdout: expected\n",
    )
    .unwrap();
    let r = cmd_test_behavior(&config_path);
    assert!(r.is_err());
}

#[test]
fn test_behavior_mode_no_assertion() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::write(
        dir.path().join("empty.spec.yaml"),
        "name: no-assert\nconfig: forjar.yaml\nbehaviors:\n  - name: nothing defined\n",
    )
    .unwrap();
    let r = cmd_test_behavior(&config_path);
    assert!(r.is_err());
}

#[test]
fn test_behavior_mode_verify_stderr_contains() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::write(
        dir.path().join("stderr.spec.yaml"),
        "name: stderr-test\nconfig: forjar.yaml\nbehaviors:\n  - name: stderr check\n    verify:\n      command: echo warning >&2\n      stderr_contains: warning\n",
    )
    .unwrap();
    let r = cmd_test_behavior(&config_path);
    assert!(r.is_ok());
}

#[test]
fn test_behavior_mode_verify_file_exists() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "").unwrap();
    let target = dir.path().join("exists.txt");
    std::fs::write(&target, "data").unwrap();
    std::fs::write(
        dir.path().join("file.spec.yaml"),
        format!("name: file-test\nconfig: forjar.yaml\nbehaviors:\n  - name: file present\n    verify:\n      command: \"true\"\n      file_exists: {}\n", target.display()),
    )
    .unwrap();
    let r = cmd_test_behavior(&config_path);
    assert!(r.is_ok());
}

#[test]
fn test_behavior_mode_verify_file_missing() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, "").unwrap();
    std::fs::write(
        dir.path().join("missing.spec.yaml"),
        "name: missing-test\nconfig: forjar.yaml\nbehaviors:\n  - name: file missing\n    verify:\n      command: \"true\"\n      file_exists: /nonexistent/path/file.txt\n",
    )
    .unwrap();
    let r = cmd_test_behavior(&config_path);
    assert!(r.is_err());
}

/// F16: `forjar test coverage` now reports resource-level coverage.
#[test]
fn test_coverage_mode_basic() {
    let f = write_temp_config(BASIC_YAML);
    let r = cmd_test_coverage(f.path());
    assert!(r.is_ok());
}

#[test]
fn test_coverage_mode_with_behavior_specs() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("forjar.yaml");
    std::fs::write(&config_path, BASIC_YAML).unwrap();
    std::fs::write(
        dir.path().join("pkg.spec.yaml"),
        "name: pkg-spec\nconfig: forjar.yaml\nbehaviors:\n  - name: installed\n    resource: pkg\n    state: present\n",
    )
    .unwrap();
    let r = cmd_test_coverage(&config_path);
    assert!(r.is_ok());
}

impl Clone for TestRow {
    fn clone(&self) -> Self {
        TestRow {
            resource_id: self.resource_id.clone(),
            machine: self.machine.clone(),
            resource_type: self.resource_type.clone(),
            status: self.status.clone(),
            detail: self.detail.clone(),
            duration_secs: self.duration_secs,
        }
    }
}

// ── Verify assertion tests ──

fn make_verify(overrides: impl FnOnce(&mut crate::core::types::VerifyCommand)) -> crate::core::types::VerifyCommand {
    let mut v = crate::core::types::VerifyCommand {
        command: "true".into(),
        exit_code: Some(0),
        stdout: None,
        stderr_contains: None,
        file_exists: None,
        file_content: None,
        port_open: None,
        retries: None,
        retry_delay_secs: None,
    };
    overrides(&mut v);
    v
}

#[test]
fn verify_file_content_exact_match() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "hello world\n").unwrap();
    let v = make_verify(|v| {
        v.file_exists = Some(path.display().to_string());
        v.file_content = Some("hello world".into());
    });
    let r = check_verify_assertions(&v, 0, "", "");
    assert!(r.is_none(), "expected pass, got: {r:?}");
}

#[test]
fn verify_file_content_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "wrong content").unwrap();
    let v = make_verify(|v| {
        v.file_exists = Some(path.display().to_string());
        v.file_content = Some("expected content".into());
    });
    let r = check_verify_assertions(&v, 0, "", "");
    assert!(r.is_some());
    assert!(r.unwrap().contains("mismatch"));
}

#[test]
fn verify_file_content_blake3() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.bin");
    let data = b"test data for hashing";
    std::fs::write(&path, data).unwrap();
    let hash = blake3::hash(data).to_hex().to_string();
    let v = make_verify(|v| {
        v.file_exists = Some(path.display().to_string());
        v.file_content = Some(format!("blake3:{hash}"));
    });
    let r = check_verify_assertions(&v, 0, "", "");
    assert!(r.is_none(), "expected pass, got: {r:?}");
}

#[test]
fn verify_file_content_blake3_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("data.bin");
    std::fs::write(&path, b"actual data").unwrap();
    let v = make_verify(|v| {
        v.file_exists = Some(path.display().to_string());
        v.file_content = Some("blake3:0000000000000000".into());
    });
    let r = check_verify_assertions(&v, 0, "", "");
    assert!(r.is_some());
    assert!(r.unwrap().contains("hash mismatch"));
}

#[test]
fn verify_port_not_open() {
    // Port 1 is almost certainly not open
    let v = make_verify(|v| {
        v.port_open = Some(1);
    });
    let r = check_verify_assertions(&v, 0, "", "");
    assert!(r.is_some());
    assert!(r.unwrap().contains("not open"));
}
