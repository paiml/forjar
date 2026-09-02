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

// ── Refs #406 (E04): what may be written down ───────────────────────────────

fn slot(state: &std::path::Path) -> run_capture::RunSlot<'_> {
    run_capture::RunSlot::new(state, "intel", Some("r-406"))
}

/// The unsuppressed baseline: `capture_exec_output` writes a transcript and
/// hands back its path. Without this control the suppression test below could
/// pass because nothing writes transcripts at all.
#[test]
fn an_unrestricted_transcript_is_written_and_its_path_returned() {
    let tmp = tempfile::tempdir().unwrap();
    let executed = run_capture::Executed {
        resource_id: "res",
        resource_type: &crate::core::types::ResourceType::File,
        action: "create",
        script: "echo ok",
        transcript: run_capture::Transcript::unrestricted(),
    };
    let log = run_capture::capture_exec_output(
        &slot(tmp.path()),
        &executed,
        &make_output(0, "ok", ""),
        0.1,
    );
    let log = log.expect("a transcript path");
    assert!(log.exists());
    assert!(tmp.path().join("intel/runs/r-406/res.script").exists());
}

/// `sensitive: true`: no transcript, no returned path — but the run directory
/// and its meta.yaml still exist, so `forjar logs` can still say the run
/// happened.
#[test]
fn a_suppressed_transcript_writes_no_files_but_keeps_the_run_record() {
    let tmp = tempfile::tempdir().unwrap();
    let executed = run_capture::Executed {
        resource_id: "res",
        resource_type: &crate::core::types::ResourceType::File,
        action: "create",
        script: "echo secret",
        transcript: run_capture::Transcript {
            secrets: Vec::new(),
            suppress: true,
        },
    };
    let log = run_capture::capture_exec_output(
        &slot(tmp.path()),
        &executed,
        &make_output(0, "secret", ""),
        0.1,
    );
    assert!(
        log.is_none(),
        "a suppressed capture must name no transcript"
    );
    let dir = tmp.path().join("intel/runs/r-406");
    assert!(
        dir.join("meta.yaml").exists(),
        "the run must still be recorded"
    );
    assert!(!dir.join("res.script").exists());
    assert!(!dir.join("res.create.log").exists());
    assert!(!dir.join("res.create.json").exists());
}

/// Redaction reaches BOTH streams, not just the script.
#[test]
fn stdout_and_stderr_are_redacted_too() {
    let tmp = tempfile::tempdir().unwrap();
    let executed = run_capture::Executed {
        resource_id: "res",
        resource_type: &crate::core::types::ResourceType::File,
        action: "create",
        script: "echo s3cret-value-406",
        transcript: run_capture::Transcript {
            secrets: vec!["s3cret-value-406".to_string()],
            suppress: false,
        },
    };
    run_capture::capture_exec_output(
        &slot(tmp.path()),
        &executed,
        &make_output(1, "wrote s3cret-value-406", "failed on s3cret-value-406"),
        0.1,
    );
    let dir = tmp.path().join("intel/runs/r-406");
    for name in ["res.script", "res.create.log", "res.create.json"] {
        let body = std::fs::read_to_string(dir.join(name)).unwrap();
        assert!(
            !body.contains("s3cret-value-406"),
            "{name} still holds the secret:\n{body}"
        );
    }
}

/// Refs #406: a resource whose value arrives as `ENC[age,…]` gets the
/// `sensitive: true` treatment without saying so. The decrypted plaintext never
/// passes through a `{{secrets.*}}` span, so the redactor cannot name it; a
/// written transcript would hold it in the clear.
#[test]
fn a_ciphertext_bearing_resource_suppresses_its_own_transcript() {
    use crate::core::types::{Resource, ResourceType, SecretsConfig};
    let marker = {
        use crate::core::secrets::B64;
        use base64::Engine;
        format!(
            "ENC[age,{}]",
            B64.encode("age-ciphertext-stand-in-long-enough")
        )
    };
    let encrypted = Resource {
        resource_type: ResourceType::File,
        path: Some("/etc/app.conf".into()),
        content: Some(format!("api_token={marker}\n")),
        ..Default::default()
    };
    assert!(run_capture::Transcript::for_resource(&encrypted, &SecretsConfig::default()).suppress);
    // The control: an ordinary resource still gets a transcript.
    let plain = Resource {
        resource_type: ResourceType::File,
        path: Some("/etc/app.conf".into()),
        content: Some("api_token=plain\n".into()),
        ..Default::default()
    };
    assert!(!run_capture::Transcript::for_resource(&plain, &SecretsConfig::default()).suppress);
}
