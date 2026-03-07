//! Tests for run_capture.rs — verify log file I/O during apply.

use super::run_capture;
use crate::core::types::ResourceRunStatus;
use crate::transport::ExecOutput;

fn make_output(exit_code: i32, stdout: &str, stderr: &str) -> ExecOutput {
    ExecOutput {
        exit_code,
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
    }
}

#[test]
fn run_dir_path() {
    let dir = run_capture::run_dir(std::path::Path::new("/state"), "intel", "r-abc123");
    assert_eq!(dir, std::path::PathBuf::from("/state/intel/runs/r-abc123"));
}

#[test]
fn ensure_run_dir_creates_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("intel/runs/r-001");
    assert!(!dir.exists());
    run_capture::ensure_run_dir(&dir, "r-001", "intel", "apply");
    assert!(dir.exists());
    assert!(dir.join("meta.yaml").exists());
}

#[test]
fn ensure_run_dir_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("intel/runs/r-001");
    run_capture::ensure_run_dir(&dir, "r-001", "intel", "apply");
    let meta1 = std::fs::read_to_string(dir.join("meta.yaml")).unwrap();
    // Second call doesn't overwrite
    run_capture::ensure_run_dir(&dir, "r-001", "intel", "apply");
    let meta2 = std::fs::read_to_string(dir.join("meta.yaml")).unwrap();
    assert_eq!(meta1, meta2);
}

#[test]
fn capture_output_writes_log_and_script() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("intel/runs/r-001");
    std::fs::create_dir_all(&dir).unwrap();

    let output = make_output(0, "installed ok\n", "");
    run_capture::capture_output(
        &dir,
        "nginx",
        "package",
        "apply",
        "intel",
        "ssh",
        "apt-get install -y nginx",
        &output,
        1.5,
    );

    let log = std::fs::read_to_string(dir.join("nginx.apply.log")).unwrap();
    assert!(log.contains("=== FORJAR TRANSPORT LOG ==="));
    assert!(log.contains("resource: nginx"));
    assert!(log.contains("type: package"));
    assert!(log.contains("action: apply"));
    assert!(log.contains("=== STDOUT ==="));
    assert!(log.contains("installed ok"));
    assert!(log.contains("exit_code: 0"));

    let script = std::fs::read_to_string(dir.join("nginx.script")).unwrap();
    assert_eq!(script, "apt-get install -y nginx");
}

#[test]
fn capture_output_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("intel/runs/r-002");
    std::fs::create_dir_all(&dir).unwrap();

    let output = make_output(100, "", "E: Unable to locate package foo\n");
    run_capture::capture_output(
        &dir,
        "bad-pkg",
        "package",
        "apply",
        "intel",
        "ssh",
        "apt-get install -y foo",
        &output,
        0.8,
    );

    let log = std::fs::read_to_string(dir.join("bad-pkg.apply.log")).unwrap();
    assert!(log.contains("exit_code: 100"));
    assert!(log.contains("Unable to locate package foo"));
}

#[test]
fn capture_output_nonexistent_dir_noop() {
    let output = make_output(0, "ok", "");
    // Should not panic even if directory doesn't exist
    run_capture::capture_output(
        std::path::Path::new("/nonexistent/dir"),
        "res",
        "file",
        "apply",
        "m",
        "local",
        "echo ok",
        &output,
        0.1,
    );
}

#[test]
fn update_meta_resource_success() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("intel/runs/r-003");
    run_capture::ensure_run_dir(&dir, "r-003", "intel", "apply");

    run_capture::update_meta_resource(
        &dir,
        "nginx",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(1.5),
            failed: false,
        },
    );

    let meta_str = std::fs::read_to_string(dir.join("meta.yaml")).unwrap();
    let meta: crate::core::types::RunMeta = serde_yaml_ng::from_str(&meta_str).unwrap();
    assert_eq!(meta.summary.converged, 1);
    assert_eq!(meta.summary.total, 1);
    assert!(meta.resources.contains_key("nginx"));
}

#[test]
fn update_meta_resource_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("intel/runs/r-004");
    run_capture::ensure_run_dir(&dir, "r-004", "intel", "apply");

    run_capture::update_meta_resource(
        &dir,
        "bad-pkg",
        ResourceRunStatus::Converged {
            exit_code: Some(100),
            duration_secs: Some(0.5),
            failed: true,
        },
    );

    let meta_str = std::fs::read_to_string(dir.join("meta.yaml")).unwrap();
    let meta: crate::core::types::RunMeta = serde_yaml_ng::from_str(&meta_str).unwrap();
    assert_eq!(meta.summary.failed, 1);
    assert_eq!(meta.summary.total, 1);
}

#[test]
fn update_meta_resource_noop() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("intel/runs/r-005");
    run_capture::ensure_run_dir(&dir, "r-005", "intel", "apply");

    run_capture::update_meta_resource(&dir, "config-file", ResourceRunStatus::Noop);

    let meta_str = std::fs::read_to_string(dir.join("meta.yaml")).unwrap();
    let meta: crate::core::types::RunMeta = serde_yaml_ng::from_str(&meta_str).unwrap();
    assert_eq!(meta.summary.noop, 1);
}

#[test]
fn update_meta_resource_multiple() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("intel/runs/r-006");
    run_capture::ensure_run_dir(&dir, "r-006", "intel", "apply");

    run_capture::update_meta_resource(
        &dir,
        "nginx",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(1.0),
            failed: false,
        },
    );
    run_capture::update_meta_resource(&dir, "config", ResourceRunStatus::Noop);
    run_capture::update_meta_resource(
        &dir,
        "bad",
        ResourceRunStatus::Converged {
            exit_code: Some(1),
            duration_secs: Some(0.3),
            failed: true,
        },
    );

    let meta_str = std::fs::read_to_string(dir.join("meta.yaml")).unwrap();
    let meta: crate::core::types::RunMeta = serde_yaml_ng::from_str(&meta_str).unwrap();
    assert_eq!(meta.summary.total, 3);
    assert_eq!(meta.summary.converged, 1);
    assert_eq!(meta.summary.noop, 1);
    assert_eq!(meta.summary.failed, 1);
    assert_eq!(meta.resources.len(), 3);
}

#[test]
fn update_meta_missing_dir_noop() {
    // Should not panic
    run_capture::update_meta_resource(
        std::path::Path::new("/nonexistent/dir"),
        "res",
        ResourceRunStatus::Noop,
    );
}
