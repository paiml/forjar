//! Coverage tests for cli/drift.rs — drift dry-run, collect locks, print summary.

const LOCK_WEB1: &str = r#"schema: "1"
machine: web1
hostname: web1
generated_at: "2025-01-01T00:00:00Z"
generator: forjar-test
blake3_version: "1.0"
resources:
  nginx:
    type: package
    status: converged
    hash: abc123
    applied_at: "2025-01-01T00:00:00Z"
    duration_seconds: 2.5
  mysql:
    type: package
    status: failed
    hash: def456
    applied_at: "2025-01-01T00:01:00Z"
    duration_seconds: 3.0
"#;

const LOCK_DB1: &str = r#"schema: "1"
machine: db1
hostname: db1
generated_at: "2025-01-01T00:00:00Z"
generator: forjar-test
blake3_version: "1.0"
resources:
  postgres:
    type: package
    status: converged
    hash: ghi789
    applied_at: "2025-01-01T00:02:00Z"
    duration_seconds: 1.0
"#;

fn setup_state_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("web1")).unwrap();
    std::fs::write(dir.path().join("web1/state.lock.yaml"), LOCK_WEB1).unwrap();
    std::fs::create_dir_all(dir.path().join("db1")).unwrap();
    std::fs::write(dir.path().join("db1/state.lock.yaml"), LOCK_DB1).unwrap();
    dir
}

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(yaml.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// ── cmd_drift_dry_run ──

#[test]
fn dry_run_text_with_machines() {
    let d = setup_state_dir();
    let r = super::drift::cmd_drift_dry_run(d.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn dry_run_json_with_machines() {
    let d = setup_state_dir();
    let r = super::drift::cmd_drift_dry_run(d.path(), None, true);
    assert!(r.is_ok());
}

#[test]
fn dry_run_text_with_filter() {
    let d = setup_state_dir();
    let r = super::drift::cmd_drift_dry_run(d.path(), Some("web1"), false);
    assert!(r.is_ok());
}

#[test]
fn dry_run_json_with_filter() {
    let d = setup_state_dir();
    let r = super::drift::cmd_drift_dry_run(d.path(), Some("web1"), true);
    assert!(r.is_ok());
}

#[test]
fn dry_run_empty_state() {
    let d = tempfile::tempdir().unwrap();
    let r = super::drift::cmd_drift_dry_run(d.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn dry_run_missing_dir() {
    let r = super::drift::cmd_drift_dry_run(std::path::Path::new("/nonexistent/state"), None, false);
    assert!(r.is_err());
}

#[test]
fn dry_run_filter_no_match() {
    let d = setup_state_dir();
    let r = super::drift::cmd_drift_dry_run(d.path(), Some("zzz_no_match"), false);
    assert!(r.is_ok());
}

// ── cmd_drift (dry_run=true bypasses transport) ──

#[test]
fn drift_dry_run_mode() {
    let d = setup_state_dir();
    let config = write_temp_config("version: '1.0'\nname: t\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources: {}\n");
    let r = super::drift::cmd_drift(
        config.path(), d.path(), None,
        false, None, false, true, false, false, None,
    );
    assert!(r.is_ok());
}

#[test]
fn drift_dry_run_json() {
    let d = setup_state_dir();
    let config = write_temp_config("version: '1.0'\nname: t\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources: {}\n");
    let r = super::drift::cmd_drift(
        config.path(), d.path(), None,
        false, None, false, true, true, false, None,
    );
    assert!(r.is_ok());
}

#[test]
fn drift_no_config_dry_run() {
    let d = setup_state_dir();
    let r = super::drift::cmd_drift(
        std::path::Path::new("/nonexistent/forjar.yaml"), d.path(), None,
        false, None, false, true, false, false, None,
    );
    assert!(r.is_ok());
}

#[test]
fn drift_no_config_no_state() {
    let d = tempfile::tempdir().unwrap();
    let r = super::drift::cmd_drift(
        std::path::Path::new("/nonexistent/forjar.yaml"), d.path(), None,
        false, None, false, false, false, false, None,
    );
    assert!(r.is_ok());
}

#[test]
fn drift_tripwire_mode_no_drift() {
    let d = tempfile::tempdir().unwrap();
    let r = super::drift::cmd_drift(
        std::path::Path::new("/nonexistent/forjar.yaml"), d.path(), None,
        true, None, false, false, false, false, None,
    );
    assert!(r.is_ok());
}

#[test]
fn drift_with_state_no_transport() {
    let d = setup_state_dir();
    let r = super::drift::cmd_drift(
        std::path::Path::new("/nonexistent/forjar.yaml"), d.path(), None,
        false, None, false, false, false, false, None,
    );
    assert!(r.is_ok());
}

#[test]
fn drift_with_state_json() {
    let d = setup_state_dir();
    let r = super::drift::cmd_drift(
        std::path::Path::new("/nonexistent/forjar.yaml"), d.path(), None,
        false, None, false, false, true, false, None,
    );
    assert!(r.is_ok());
}

#[test]
fn drift_with_machine_filter() {
    let d = setup_state_dir();
    let r = super::drift::cmd_drift(
        std::path::Path::new("/nonexistent/forjar.yaml"), d.path(), Some("web1"),
        false, None, false, false, false, false, None,
    );
    assert!(r.is_ok());
}
