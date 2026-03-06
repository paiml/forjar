//! Coverage tests for cli/fleet_reporting.rs — cmd_audit, cmd_export, cmd_compliance, cmd_suggest.

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
    hash: abc123def456
    applied_at: "2025-01-01T00:00:00Z"
    duration_seconds: 2.5
  config:
    type: file
    status: converged
    hash: ghi789jkl012
    applied_at: "2025-01-01T00:01:00Z"
    duration_seconds: 0.5
"#;

fn setup_state(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("web1")).unwrap();
    std::fs::write(dir.join("web1/state.lock.yaml"), LOCK_WEB1).unwrap();
    let events = concat!(
        r#"{"ts":"2025-01-01T00:00:00Z","event":"apply_started","machine":"web1","resources":2}"#, "\n",
        r#"{"ts":"2025-01-01T00:01:00Z","event":"resource_converged","machine":"web1","resource":"nginx","duration_seconds":2.5,"hash":"blake3:abc123"}"#, "\n",
        r#"{"ts":"2025-01-01T00:02:00Z","event":"resource_converged","machine":"web1","resource":"config","duration_seconds":0.5,"hash":"blake3:ghi789"}"#, "\n",
        r#"{"ts":"2025-01-01T00:03:00Z","event":"apply_completed","machine":"web1","converged":2,"failed":0,"unchanged":0}"#, "\n",
    );
    std::fs::write(dir.join("web1/events.jsonl"), events).unwrap();
}

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, yaml.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();
    f
}

// ── cmd_audit ──

#[test]
fn audit_empty_dir() {
    let d = tempfile::tempdir().unwrap();
    let r = super::fleet_reporting::cmd_audit(d.path(), None, 10, false);
    assert!(r.is_ok());
}

#[test]
fn audit_empty_dir_json() {
    let d = tempfile::tempdir().unwrap();
    let r = super::fleet_reporting::cmd_audit(d.path(), None, 10, true);
    assert!(r.is_ok());
}

#[test]
fn audit_with_events() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::fleet_reporting::cmd_audit(d.path(), None, 10, false);
    assert!(r.is_ok());
}

#[test]
fn audit_with_events_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::fleet_reporting::cmd_audit(d.path(), None, 10, true);
    assert!(r.is_ok());
}

#[test]
fn audit_machine_filter() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::fleet_reporting::cmd_audit(d.path(), Some("web1"), 10, false);
    assert!(r.is_ok());
}

#[test]
fn audit_limit() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::fleet_reporting::cmd_audit(d.path(), None, 1, false);
    assert!(r.is_ok());
}

#[test]
fn audit_missing_dir() {
    let d = tempfile::tempdir().unwrap();
    let r = super::fleet_reporting::cmd_audit(&d.path().join("no_such_dir"), None, 10, false);
    assert!(r.is_err());
}

// ── cmd_export ──

#[test]
fn export_csv() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::fleet_reporting::cmd_export(d.path(), "csv", None, None);
    assert!(r.is_ok());
}

#[test]
fn export_terraform() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::fleet_reporting::cmd_export(d.path(), "terraform", None, None);
    assert!(r.is_ok());
}

#[test]
fn export_ansible() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::fleet_reporting::cmd_export(d.path(), "ansible", None, None);
    assert!(r.is_ok());
}

#[test]
fn export_unknown_format() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::fleet_reporting::cmd_export(d.path(), "unknown", None, None);
    assert!(r.is_err());
}

#[test]
fn export_machine_filter() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let r = super::fleet_reporting::cmd_export(d.path(), "csv", Some("web1"), None);
    assert!(r.is_ok());
}

#[test]
fn export_to_file() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let out = d.path().join("export.csv");
    let r = super::fleet_reporting::cmd_export(d.path(), "csv", None, Some(&out));
    assert!(r.is_ok());
    assert!(out.exists());
}

#[test]
fn export_empty_state() {
    let d = tempfile::tempdir().unwrap();
    let r = super::fleet_reporting::cmd_export(d.path(), "csv", None, None);
    assert!(r.is_ok());
}

// ── cmd_compliance ──

#[test]
fn compliance_text() {
    let cfg = write_temp_config("version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n");
    let r = super::fleet_reporting::cmd_compliance(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn compliance_json() {
    let cfg = write_temp_config("version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n");
    let r = super::fleet_reporting::cmd_compliance(cfg.path(), true);
    assert!(r.is_ok());
}

#[test]
fn compliance_missing_file() {
    let r = super::fleet_reporting::cmd_compliance(std::path::Path::new("/nonexistent"), false);
    assert!(r.is_err());
}

// ── cmd_suggest ──

#[test]
fn suggest_text() {
    let cfg = write_temp_config("version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n");
    let r = super::fleet_reporting::cmd_suggest(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn suggest_json() {
    let cfg = write_temp_config("version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n");
    let r = super::fleet_reporting::cmd_suggest(cfg.path(), true);
    assert!(r.is_ok());
}

#[test]
fn suggest_missing_file() {
    let r = super::fleet_reporting::cmd_suggest(std::path::Path::new("/nonexistent"), false);
    assert!(r.is_err());
}
